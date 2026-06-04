//! MigrationEngine - imports external data into Quilt.
//!
//! Currently supports Logseq-flavored markdown files.

use crate::migration::{parse_logseq, Frontmatter, MigrationError as ParseMigrationError, RawBlock};
use async_trait::async_trait;
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::properties::types::PropertyType;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, PropertyValue, Uuid};
use quilt_domain::errors::DomainError;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

/// Result of a successful import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Number of pages created.
    pub pages_created: usize,
    /// Number of blocks created.
    pub blocks_created: usize,
    /// Warning messages (e.g., page collisions).
    pub warnings: Vec<String>,
}

/// Errors that can occur during migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("Import failed: {0}")]
    Import(String),

    #[error("Page already exists: {0}")]
    PageAlreadyExists(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Migration engine - generic over page and block repositories.
pub struct MigrationEngine<PR: PageRepository, BR: BlockRepository> {
    page_repo: Arc<PR>,
    block_repo: Arc<BR>,
}

impl<PR: PageRepository, BR: BlockRepository> MigrationEngine<PR, BR> {
    pub fn new(page_repo: Arc<PR>, block_repo: Arc<BR>) -> Self {
        Self {
            page_repo,
            block_repo,
        }
    }

    /// Import a single Logseq markdown file.
    #[instrument(skip(self, source))]
    pub async fn import_file(
        &self,
        source: &str,
        page_name: &str,
    ) -> Result<ImportResult, MigrationError> {
        // Check if page already exists
        if let Ok(Some(_)) = self.page_repo.get_by_name(page_name).await {
            return Ok(ImportResult {
                pages_created: 0,
                blocks_created: 0,
                warnings: vec![format!("Page '{}' already exists, skipping", page_name)],
            });
        }

        // Parse the Logseq markdown
        let (frontmatter, raw_blocks) =
            parse_logseq(source).map_err(|e| MigrationError::Import(e.to_string()))?;

        // Create the page
        let page_id = Uuid::new_v4();
        let page_create = PageCreate {
            name: page_name.to_string(),
            title: frontmatter
                .properties
                .iter()
                .find(|p| p.key == "title")
                .map(|p| p.value.clone()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        };
        let page = Page::new(page_create)
            .map_err(|e| MigrationError::Import(e.to_string()))?;

        self.page_repo
            .insert(&page)
            .await
            .map_err(|e| MigrationError::Import(e.to_string()))?;

        // Create blocks from parsed raw blocks
        let blocks_created = create_blocks_from_raw(
            self.block_repo.as_ref(),
            &raw_blocks,
            page_id,
        )
        .await?;

        Ok(ImportResult {
            pages_created: 1,
            blocks_created,
            warnings: Vec::new(),
        })
    }

    /// Import all Logseq markdown files from a directory.
    #[instrument(skip(self))]
    pub async fn import_directory(&self, dir_path: &Path) -> Result<Vec<ImportResult>, MigrationError> {
        let mut results = Vec::new();

        let entries = std::fs::read_dir(dir_path)
            .map_err(|e| MigrationError::Io(e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();

                let source = std::fs::read_to_string(&path)
                    .map_err(|e| MigrationError::Io(e))?;

                match self.import_file(&source, &file_name).await {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        results.push(ImportResult {
                            pages_created: 0,
                            blocks_created: 0,
                            warnings: vec![format!("Failed to import {:?}: {}", path, e)],
                        });
                    }
                }
            }
        }

        Ok(results)
    }
}

/// Create blocks from parsed raw blocks using iterative approach with a work stack.
async fn create_blocks_from_raw<BR: BlockRepository>(
    block_repo: &BR,
    raw_blocks: &[RawBlock],
    page_id: Uuid,
) -> Result<usize, MigrationError> {
    let mut blocks_created = 0;
    
    // Work stack: (raw_blocks, parent_id, start_order)
    let mut stack: Vec<(&[RawBlock], Option<Uuid>, u32)> = vec![(raw_blocks, None, 0)];
    
    while let Some((blocks, parent_id, mut order)) = stack.pop() {
        for raw in blocks {
            // Convert raw properties to PropertyValue map
            let mut properties = std::collections::HashMap::new();
            for prop in &raw.properties {
                let pv = infer_property_value(&prop.value);
                properties.insert(prop.key.clone(), pv);
            }

            let block_create = BlockCreate {
                page_id,
                content: raw.content.clone(),
                parent_id,
                order: order as f64,
                marker: None,
                format: BlockFormat::Markdown,
                properties,
            };
            let block = Block::new(block_create)
                .map_err(|e| MigrationError::Import(e.to_string()))?;

            block_repo.insert(&block)
                .await
                .map_err(|e| MigrationError::Import(e.to_string()))?;
            blocks_created += 1;
            order += 1;

            // Push children to stack (reversed to maintain order)
            if !raw.children.is_empty() {
                stack.push((&raw.children[..], Some(block.id), order));
            }
        }
    }

    Ok(blocks_created)
}

/// Infer PropertyValue from a string value.
/// Used internally during block creation from parsed Logseq content.
pub fn infer_property_value(value: &str) -> PropertyValue {
    let trimmed = value.trim();

    // Check for boolean
    if trimmed.eq_ignore_ascii_case("true") {
        return PropertyValue::Boolean(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return PropertyValue::Boolean(false);
    }

    // Check for number (integer or float)
    if let Ok(i) = trimmed.parse::<i64>() {
        return PropertyValue::Integer(i);
    }
    if let Ok(f) = trimmed.parse::<f64>() {
        return PropertyValue::Float(f);
    }

    // Default to string
    PropertyValue::String(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::parse_logseq;

    #[test]
    fn infer_property_value_tests() {
        assert!(matches!(infer_property_value("true"), PropertyValue::Boolean(true)));
        assert!(matches!(infer_property_value("TRUE"), PropertyValue::Boolean(true)));
        assert!(matches!(infer_property_value("false"), PropertyValue::Boolean(false)));
        assert!(matches!(infer_property_value("123"), PropertyValue::Integer(123)));
        assert!(matches!(infer_property_value("3.14"), PropertyValue::Float(f) if (f - 3.14).abs() < 0.001));
        assert!(matches!(infer_property_value("hello"), PropertyValue::String(s) if s == "hello"));
    }
}
