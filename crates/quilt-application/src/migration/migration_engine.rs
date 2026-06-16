//! MigrationEngine - imports external data into Quilt.
//!
//! Currently supports Markdown-flavored files (Quilt format).

use crate::migration::{RawBlock, parse_md_import};
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::properties::resolver::PropertyKeyResolver;
use quilt_domain::repositories::{BlockRepository, PageRepository, PropertyRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
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

/// Migration engine - non-generic, uses trait objects for dependency injection.
pub struct MigrationEngine {
    page_repo: Arc<dyn PageRepository>,
    block_repo: Arc<dyn BlockRepository>,
    property_repo: Arc<dyn PropertyRepository>,
}

impl MigrationEngine {
    pub fn new(
        page_repo: Arc<dyn PageRepository>,
        block_repo: Arc<dyn BlockRepository>,
        property_repo: Arc<dyn PropertyRepository>,
    ) -> Self {
        Self {
            page_repo,
            block_repo,
            property_repo,
        }
    }

    /// Import a single Markdown file.
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

        // Parse the Markdown import
        let (frontmatter, raw_blocks) =
            parse_md_import(source).map_err(|e| MigrationError::Import(e.to_string()))?;

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
        let page = Page::new(page_create).map_err(|e| MigrationError::Import(e.to_string()))?;

        self.page_repo
            .insert(&page)
            .await
            .map_err(|e| MigrationError::Import(e.to_string()))?;

        // Create blocks from parsed raw blocks
        let resolver = PropertyKeyResolver::new(self.property_repo.clone());
        let blocks_created =
            create_blocks_from_raw(self.block_repo.as_ref(), &resolver, &raw_blocks, page_id)
                .await?;

        Ok(ImportResult {
            pages_created: 1,
            blocks_created,
            warnings: Vec::new(),
        })
    }

    /// Import all Markdown files from a directory.
    #[instrument(skip(self))]
    pub async fn import_directory(
        &self,
        dir_path: &Path,
    ) -> Result<Vec<ImportResult>, MigrationError> {
        let mut results = Vec::new();

        let entries = std::fs::read_dir(dir_path).map_err(|e| MigrationError::Io(e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("untitled")
                    .to_string();

                let source = std::fs::read_to_string(&path).map_err(|e| MigrationError::Io(e))?;

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
/// Uses PropertyKeyResolver to normalize property keys (case-insensitive, builtin match).
async fn create_blocks_from_raw(
    block_repo: &dyn BlockRepository,
    resolver: &PropertyKeyResolver,
    raw_blocks: &[RawBlock],
    page_id: Uuid,
) -> Result<usize, MigrationError> {
    let mut blocks_created = 0;

    // Work stack: (raw_blocks, parent_id, start_order)
    let mut stack: Vec<(&[RawBlock], Option<Uuid>, u32)> = vec![(raw_blocks, None, 0)];

    while let Some((blocks, parent_id, mut order)) = stack.pop() {
        for raw in blocks {
            // Convert raw properties to PropertyValue map
            // Use resolver to normalize keys (case-insensitive, builtin match)
            let mut properties = std::collections::HashMap::new();
            for prop in &raw.properties {
                let pv = infer_property_value(&prop.value);
                // Resolve the key to get the canonical (lowercase) property key
                let canonical_key = match resolver.resolve(&prop.key).await {
                    Ok(def) => def.db_ident.clone(),
                    Err(_) => prop.key.to_lowercase(),
                };
                properties.insert(canonical_key, pv);
            }

            let block_create = BlockCreate {
                page_id,
                content: raw.content.clone(),
                parent_id,
                order: order as f64,
                marker: None,
                format: BlockFormat::Markdown,
                block_type: BlockType::Paragraph,
                properties,
            };
            let block =
                Block::new(block_create).map_err(|e| MigrationError::Import(e.to_string()))?;

            block_repo
                .insert(&block)
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
/// Used internally during block creation from parsed Markdown content.
pub fn infer_property_value(value: &str) -> PropertyValue {
    let trimmed = value.trim();

    // Check for boolean (true/false - checkbox detection)
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

    // Check for date: YYYY-MM-DDTHH:MM:SS format first
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return PropertyValue::Date(dt.and_utc());
    }

    // Check for date: YYYY-MM-DD format
    if let Ok(date) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        return PropertyValue::Date(dt.and_utc());
    }

    // Default to string
    PropertyValue::String(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::parse_md_import;
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use quilt_domain::properties::definition::PropertyDefinition;
    use quilt_domain::properties::types::{Cardinality, PropertyType, PropertyVisibility, ViewContext};
    use std::collections::HashMap;

    #[test]
    fn infer_property_value_tests() {
        assert!(matches!(
            infer_property_value("true"),
            PropertyValue::Boolean(true)
        ));
        assert!(matches!(
            infer_property_value("TRUE"),
            PropertyValue::Boolean(true)
        ));
        assert!(matches!(
            infer_property_value("false"),
            PropertyValue::Boolean(false)
        ));
        assert!(matches!(
            infer_property_value("123"),
            PropertyValue::Integer(123)
        ));
        assert!(
            matches!(infer_property_value("3.14"), PropertyValue::Float(f) if (f - 3.14).abs() < 0.001)
        );
        assert!(matches!(infer_property_value("hello"), PropertyValue::String(s) if s == "hello"));
    }

    #[test]
    fn infer_property_value_date_tests() {
        // Date detection: YYYY-MM-DD format
        let result = infer_property_value("2024-01-15");
        assert!(matches!(result, PropertyValue::Date(_)));
        if let PropertyValue::Date(dt) = result {
            assert_eq!(dt.naive_utc().date().to_string(), "2024-01-15");
        }

        // DateTime detection: YYYY-MM-DDTHH:MM:SS format
        let result = infer_property_value("2024-12-31T23:59:59");
        assert!(matches!(result, PropertyValue::Date(_)));
        if let PropertyValue::Date(dt) = result {
            assert_eq!(dt.naive_utc().date().to_string(), "2024-12-31");
            assert_eq!(dt.naive_utc().time().to_string(), "23:59:59");
        }

        // Edge case: YYYY-MM-DD at month boundary
        let result = infer_property_value("2024-01-01");
        assert!(matches!(result, PropertyValue::Date(_)));

        // Not a date: YYYY-MM-DD but with invalid month/day falls back to String
        let result = infer_property_value("2024-13-45");
        assert!(matches!(result, PropertyValue::String(_)));

        // Not a date: looks like date but has extra text
        let result = infer_property_value("2024-01-15 some text");
        assert!(matches!(result, PropertyValue::String(_)));
    }

    // Mock PropertyRepository for testing resolver integration
    mod mock_property_repo {
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        #[derive(Default)]
        pub struct MockPropertyRepo {
            pub properties: HashMap<String, PropertyDefinition>,
            pub consult_count: AtomicUsize,
        }

        #[async_trait]
        impl PropertyRepository for MockPropertyRepo {
            async fn get_by_id(
                &self,
                _id: Uuid,
            ) -> Result<Option<PropertyDefinition>, DomainError> {
                Ok(None)
            }

            async fn get_by_db_ident(
                &self,
                ident: &str,
            ) -> Result<Option<PropertyDefinition>, DomainError> {
                self.consult_count.fetch_add(1, Ordering::SeqCst);
                Ok(self.properties.get(ident).cloned())
            }

            async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
                Ok(self.properties.values().cloned().collect())
            }

            async fn insert(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
                Ok(())
            }

            async fn update(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
                Ok(())
            }

            async fn get_closed_values(
                &self,
                _property_id: Uuid,
            ) -> Result<Vec<quilt_domain::properties::types::ClosedValue>, DomainError>
            {
                Ok(Vec::new())
            }

            async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
                Ok(())
            }
            async fn get_by_db_idents(
                &self,
                _idents: &[&str],
            ) -> Result<Vec<PropertyDefinition>, DomainError> {
                Ok(Vec::new())
            }
            async fn search(
                &self,
                _query: &str,
                _limit: usize,
            ) -> Result<Vec<PropertyDefinition>, DomainError> {
                Ok(Vec::new())
            }
            async fn list_by_usage(
                &self,
                _limit: usize,
            ) -> Result<Vec<PropertyDefinition>, DomainError> {
                Ok(Vec::new())
            }
            async fn get_co_occurrences(
                &self,
                _limit: usize,
            ) -> Result<Vec<quilt_domain::properties::analytics::PropertyCoOccurrence>, DomainError>
            {
                Ok(vec![])
            }
            async fn get_trends(
                &self,
                _period_days: u32,
                _limit: usize,
            ) -> Result<Vec<quilt_domain::properties::analytics::PropertyTrend>, DomainError>
            {
                Ok(vec![])
            }
            async fn count_distinct_properties(&self) -> Result<u64, DomainError> {
                Ok(0)
            }
            async fn count_blocks_with_properties(&self) -> Result<u64, DomainError> {
                Ok(0)
            }
        }
    }

    #[tokio::test]
    async fn test_resolver_normalizes_case_insensitive_keys() {
        use mock_property_repo::MockPropertyRepo;

        // Create a mock property repo with a custom property
        let custom_prop = PropertyDefinition::new(
            Uuid::new_v4(),
            "custom/property".to_string(),
            "Custom Property".to_string(),
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Inline);

        let mock_repo = MockPropertyRepo {
            properties: HashMap::from([(custom_prop.db_ident.clone(), custom_prop.clone())]),
            ..Default::default()
        };

        let repo = Arc::new(mock_repo);
        let resolver = PropertyKeyResolver::new(repo.clone());

        // Test that mixed-case key resolves to the same definition
        let result1 = resolver.resolve("CUSTOM/PROPERTY").await;
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap().db_ident, "custom/property");

        let result2 = resolver.resolve("Custom/Property").await;
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap().db_ident, "custom/property");

        // Verify the repo was consulted (not just builtin fallback)
        // The mock should have been called for these lookups
        assert!(repo.consult_count.load(std::sync::atomic::Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn test_resolver_falls_back_to_builtin() {
        use mock_property_repo::MockPropertyRepo;

        let mock_repo: Arc<MockPropertyRepo> = Arc::new(Default::default());
        let resolver = PropertyKeyResolver::new(mock_repo.clone());

        // "quilt.property/priority" is a builtin property
        let result = resolver.resolve("quilt.property/priority").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().db_ident, "quilt.property/priority");
    }

    #[tokio::test]
    async fn test_resolver_unknown_key_returns_not_found() {
        use mock_property_repo::MockPropertyRepo;

        let mock_repo: Arc<MockPropertyRepo> = Arc::new(Default::default());
        let resolver = PropertyKeyResolver::new(mock_repo);

        let result = resolver.resolve("nonexistent/property").await;
        assert!(result.is_err());
    }
}
