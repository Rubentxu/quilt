//! Factory for creating a `(Page, Vec<Block>)` pair with aligned IDs.
//!
//! This fixture creates a `Page` and top-level `Block`s that all share the
//! same `page_id`, making them ready to insert into the respective repositories.
//!
//! # Example
//!
//! ```
//! use quilt_test_helpers::page_with_blocks;
//!
//! let (page, blocks) = page_with_blocks("My Page", vec!["First block", "Second block"]).unwrap();
//! assert_eq!(blocks.iter().filter(|b| b.page_id == page.id).count(), 2);
//! ```

use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::errors::DomainError;
use quilt_domain::value_objects::{BlockFormat, BlockType};

/// Create a page and its top-level blocks with aligned `page_id` values.
///
/// All blocks will have:
/// - `page_id` set to the page's ID
/// - `parent_id` set to `None` (top-level blocks)
/// - Incrementing `order` values (1.0, 2.0, 3.0, ...)
/// - `level` set to 1 (top-level)
///
/// # Arguments
///
/// * `page_name` — The name for the new page (will be normalized to lowercase)
/// * `block_contents` — The content strings for each top-level block
///
/// # Returns
///
/// A `Result` containing a tuple of `(Page, Vec<Block>)` ready to be inserted
/// into repositories, or a `DomainError` if page/block creation fails.
pub fn page_with_blocks(
    page_name: &str,
    block_contents: Vec<&str>,
) -> Result<(Page, Vec<Block>), DomainError> {
    let page = Page::new(PageCreate {
        name: page_name.to_string(),
        title: Some(page_name.to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: std::collections::HashMap::new(),
        // Fixture pages are test artifacts, not ingested from files
        source_path: None,
        source_mtime: None,
    })
    .map_err(|e| {
        DomainError::InvalidData(format!("page_with_blocks: failed to create page: {}", e))
    })?;

    let blocks: Vec<Block> = block_contents
        .into_iter()
        .enumerate()
        .map(|(idx, content)| {
            Block::new(BlockCreate {
                page_id: page.id,
                content: content.to_string(),
                parent_id: None,
                order: (idx + 1) as f64,
                marker: None,
                format: BlockFormat::Markdown,
                block_type: BlockType::Paragraph,
                properties: std::collections::HashMap::new(),
            })
            .map_err(|e| {
                DomainError::InvalidData(format!("page_with_blocks: failed to create block: {}", e))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok((page, blocks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_with_blocks_creates_aligned_ids() {
        let (page, blocks) = page_with_blocks("Test Page", vec!["Block 1", "Block 2", "Block 3"])
            .expect("page_with_blocks should succeed");

        // Page should be valid
        assert!(!page.name.is_empty());

        // All blocks should have the same page_id as the page
        for block in &blocks {
            assert_eq!(block.page_id, page.id);
        }

        // All blocks should be top-level (no parent)
        for block in &blocks {
            assert!(block.parent_id.is_none());
        }
    }

    #[test]
    fn test_page_with_blocks_incrementing_order() {
        let (_page, blocks) =
            page_with_blocks("Test", vec!["A", "B", "C"]).expect("page_with_blocks should succeed");

        assert_eq!(blocks[0].order, 1.0);
        assert_eq!(blocks[1].order, 2.0);
        assert_eq!(blocks[2].order, 3.0);
    }

    #[test]
    fn test_page_with_blocks_correct_count() {
        let (_page, blocks) =
            page_with_blocks("Test", vec!["A", "B"]).expect("page_with_blocks should succeed");
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_page_with_blocks_empty() {
        let (_page, blocks) =
            page_with_blocks("Test", vec![]).expect("page_with_blocks should succeed");
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_page_with_blocks_content_preserved() {
        let (_page, blocks) = page_with_blocks("Test", vec!["Hello world", "Goodbye world"])
            .expect("page_with_blocks should succeed");

        assert_eq!(blocks[0].content, "Hello world");
        assert_eq!(blocks[1].content, "Goodbye world");
    }

    #[test]
    fn test_page_name_normalization() {
        let (page, _) =
            page_with_blocks("UPPERCASE PAGE", vec![]).expect("page_with_blocks should succeed");
        // Page name should be normalized to lowercase
        assert_eq!(page.name, "uppercase page");
    }
}
