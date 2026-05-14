//! Application queries (CQRS - Read)
//!
//! This module provides query handlers for reading domain entities.
//! Queries follow the CQRS pattern by encapsulating all read operations.

use crate::errors::ApplicationError;
use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockRepository, BlockRepositoryExt, PageRepository};
use quilt_domain::value_objects::{BlockFormat, Uuid};
use std::sync::Arc;
use tracing::instrument;

/// Query handler for block read operations.
///
/// Encapsulates all operations that read block entities:
/// - Getting a single block by ID
/// - Getting a block with all its children recursively
/// - Getting backlinks (blocks that reference this block)
/// - Getting blocks by page
/// - Full-text search across blocks
///
/// # Type Parameters
///
/// - `R`: The block repository implementation
/// - `P`: The page repository implementation
pub struct BlockQuery<R: BlockRepository + BlockRepositoryExt, P: PageRepository> {
    repository: Arc<R>,
    page_repo: Arc<P>,
}

impl<R: BlockRepository + BlockRepositoryExt, P: PageRepository> BlockQuery<R, P> {
    /// Creates a new `BlockQuery` handler with the given repositories.
    ///
    /// # Arguments
    ///
    /// * `repository` - An `Arc`-wrapped repository for block persistence
    /// * `page_repo` - An `Arc`-wrapped repository for page lookups
    pub fn new(repository: Arc<R>, page_repo: Arc<P>) -> Self {
        Self {
            repository,
            page_repo,
        }
    }

    /// Retrieves a single block by its UUID.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Block))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get(&self, block_id: Uuid) -> Result<Option<Block>, ApplicationError> {
        self.repository
            .get_by_id(block_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Retrieves a block and all its descendants recursively.
    ///
    /// The result includes the block itself as the first element,
    /// followed by all children (and their children, etc.) in depth-first order.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the root block
    ///
    /// # Returns
    ///
    /// Returns a `Vec` containing the block and all its descendants.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Block with given ID does not exist
    /// - Repository operation fails during recursion
    #[instrument(skip(self))]
    pub async fn get_with_children(&self, block_id: Uuid) -> Result<Vec<Block>, ApplicationError> {
        let block = self
            .repository
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let mut result = vec![block];
        self.collect_children(block_id, &mut result).await?;

        Ok(result)
    }

    /// Recursively collects all children of a block into the result vector.
    ///
    /// This is an internal helper method used by [`get_with_children`].
    async fn collect_children(
        &self,
        parent_id: Uuid,
        result: &mut Vec<Block>,
    ) -> Result<(), ApplicationError> {
        let children = self
            .repository
            .get_children(parent_id)
            .await
            .map_err(ApplicationError::Domain)?;

        for child in children {
            result.push(child.clone());
            Box::pin(self.collect_children(child.id, result)).await?;
        }

        Ok(())
    }

    /// Retrieves all blocks that link back to the specified block.
    ///
    /// Backlinks are blocks that contain a reference to this block,
    /// typically via `[[block-id]]` or similar page reference syntax.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to find backlinks for
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of all blocks that reference this block.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, ApplicationError> {
        self.repository
            .get_backlinks(block_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Performs full-text search across all blocks.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of blocks matching the search query.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, ApplicationError> {
        self.repository
            .search(query, limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Get all blocks on a page, optionally filtered by format.
    ///
    /// # Arguments
    ///
    /// * `page_name` - The name of the page to get blocks for
    /// * `format` - Optional filter for block format (Markdown or Org)
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of blocks on the page, sorted by order.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Page with given name does not exist
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn get_page_blocks(
        &self,
        page_name: &str,
        format: Option<BlockFormat>,
    ) -> Result<Vec<Block>, ApplicationError> {
        let page = self
            .page_repo
            .get_by_name(page_name)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Domain(quilt_domain::DomainError::NotFound(format!(
                    "Page '{}' not found",
                    page_name
                )))
            })?;
        let mut blocks = self
            .repository
            .get_by_page(page.id)
            .await
            .map_err(ApplicationError::Domain)?;
        if let Some(fmt) = format {
            blocks.retain(|b| b.format == fmt);
        }
        blocks.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(blocks)
    }

    /// Get all blocks belonging to a page by page ID.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of blocks on the page.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, ApplicationError> {
        self.repository
            .get_by_page(page_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Check if a block exists by ID.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to check
    ///
    /// # Returns
    ///
    /// Returns `true` if the block exists, `false` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn check_exists(&self, block_id: Uuid) -> Result<bool, ApplicationError> {
        self.repository
            .exists(block_id)
            .await
            .map_err(ApplicationError::Domain)
    }
}

/// Query handler for page read operations.
///
/// Encapsulates all operations that read page entities:
/// - Getting a single page by ID
/// - Getting a page by name
/// - Getting journal pages by date
/// - Getting all pages or recent pages
///
/// # Type Parameters
///
/// - `R`: The page repository implementation
pub struct PageQuery<R: PageRepository> {
    repository: Arc<R>,
}

impl<R: PageRepository> PageQuery<R> {
    /// Creates a new `PageQuery` handler with the given repository.
    ///
    /// # Arguments
    ///
    /// * `repository` - An `Arc`-wrapped repository for page persistence
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Retrieves a single page by its UUID.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Page))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get(
        &self,
        page_id: Uuid,
    ) -> Result<Option<quilt_domain::entities::Page>, ApplicationError> {
        self.repository
            .get_by_id(page_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Retrieves a page by its unique name.
    ///
    /// Page names are unique identifiers used for lookups
    /// and page references (e.g., `[[Page Name]]`).
    ///
    /// # Arguments
    ///
    /// * `name` - The unique name of the page
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Page))` if found, `Ok(None)` if not found.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Option<quilt_domain::entities::Page>, ApplicationError> {
        self.repository
            .get_by_name(name)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Retrieves the journal page for a specific day.
    ///
    /// Journal pages are created for specific calendar dates.
    /// Each day can have at most one journal page.
    ///
    /// # Arguments
    ///
    /// * `day` - The JournalDay representing the date to look up
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(Page))` if a journal exists for that day, `Ok(None)` otherwise.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_journal(
        &self,
        day: quilt_domain::value_objects::JournalDay,
    ) -> Result<Option<quilt_domain::entities::Page>, ApplicationError> {
        self.repository
            .get_journal(day)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Retrieves all pages in the repository.
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of all pages.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_all(&self) -> Result<Vec<quilt_domain::entities::Page>, ApplicationError> {
        self.repository
            .get_all()
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Retrieves the most recently accessed or modified pages.
    ///
    /// # Arguments
    ///
    /// * `limit` - Maximum number of pages to return
    ///
    /// # Returns
    ///
    /// Returns a `Vec` of the most recent pages (up to `limit`).
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the repository operation fails.
    #[instrument(skip(self))]
    pub async fn get_recent(
        &self,
        limit: usize,
    ) -> Result<Vec<quilt_domain::entities::Page>, ApplicationError> {
        self.repository
            .get_recent(limit)
            .await
            .map_err(ApplicationError::Domain)
    }
}

/// Type alias for `BlockQuery` implementing the query handler pattern.
///
/// This is the concrete type returned by query factory functions and
/// used in dependency injection for request handlers.
pub type BlockQueryHandler<R, P> = BlockQuery<R, P>;

/// Type alias for `PageQuery` implementing the query handler pattern.
///
/// This is the concrete type returned by query factory functions and
/// used in dependency injection for request handlers.
pub type PageQueryHandler<R> = PageQuery<R>;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::entities::{Block, Page};
    use quilt_domain::errors::DomainError;
    use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Mock BlockRepository for testing
    struct MockBlockRepository {
        blocks: Vec<Block>,
    }

    impl MockBlockRepository {
        fn new(blocks: Vec<Block>) -> Self {
            Self { blocks }
        }
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepository {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
            Ok(self.blocks.iter().find(|b| b.id == id).cloned())
        }

        async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(self
                .blocks
                .iter()
                .filter(|b| b.page_id == page_id)
                .cloned()
                .collect())
        }

        async fn get_children(&self, _parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn get_with_refs(&self, _id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            unimplemented!()
        }

        async fn insert(&self, _block: &Block) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn update(&self, _block: &Block) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn hard_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            Ok(self.blocks.iter().filter(|b| b.page_id == _page_id).count())
        }

        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }

        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
    }

    // Note: BlockRepositoryExt is auto-implemented for MockBlockRepository
    // via the blanket impl in quilt_domain

    /// Mock PageRepository for testing
    struct MockPageRepository {
        pages: Vec<Page>,
    }

    impl MockPageRepository {
        fn new(pages: Vec<Page>) -> Self {
            Self { pages }
        }
    }

    #[async_trait]
    impl PageRepository for MockPageRepository {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
            Ok(self.pages.iter().find(|p| p.id == id).cloned())
        }

        async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
            Ok(self.pages.iter().find(|p| p.name == name).cloned())
        }

        async fn get_journal(&self, _day: JournalDay) -> Result<Option<Page>, DomainError> {
            Ok(None)
        }

        async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
            Ok(self.pages.clone())
        }

        async fn get_namespace_pages(&self, _namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn insert(&self, _page: &Page) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn update(&self, _page: &Page) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn rename(&self, _id: Uuid, _new_name: &str) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn soft_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn hard_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            unimplemented!()
        }

        async fn recycle_bin(&self) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn get_recent(&self, _limit: usize) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn count(&self) -> Result<usize, DomainError> {
            Ok(self.pages.len())
        }

        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn get_orphan_pages(&self) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }
    }

    fn create_test_block(
        id: &str,
        page_id: Uuid,
        content: &str,
        order: f64,
        format: BlockFormat,
    ) -> Block {
        Block {
            id: Uuid::parse_str(id).unwrap(),
            page_id,
            parent_id: None,
            order,
            level: 1,
            format,
            marker: None,
            priority: None,
            content: content.to_string(),
            properties: HashMap::new(),
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            journal_day: None,
            updated_journal_day: None,
        }
    }

    fn create_test_page(id: &str, name: &str) -> Page {
        Page {
            id: Uuid::parse_str(id).unwrap(),
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            original_name: None,
            journal: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_get_page_blocks_success() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![
            create_test_block(
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                page_id,
                "Block 1",
                1.0,
                BlockFormat::Markdown,
            ),
            create_test_block(
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                page_id,
                "Block 2",
                2.0,
                BlockFormat::Markdown,
            ),
            create_test_block(
                "cccccccc-cccc-cccc-cccc-cccccccccccc",
                page_id,
                "Block 3",
                3.0,
                BlockFormat::Org,
            ),
        ];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get_page_blocks("Test Page", None).await;
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 3);
        // Verify sorted by order
        assert_eq!(blocks[0].content, "Block 1");
        assert_eq!(blocks[1].content, "Block 2");
        assert_eq!(blocks[2].content, "Block 3");
    }

    #[tokio::test]
    async fn test_get_page_blocks_with_format_filter() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![
            create_test_block(
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                page_id,
                "MD Block",
                1.0,
                BlockFormat::Markdown,
            ),
            create_test_block(
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                page_id,
                "Org Block",
                2.0,
                BlockFormat::Org,
            ),
        ];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        // Filter by Markdown
        let result = query
            .get_page_blocks("Test Page", Some(BlockFormat::Markdown))
            .await;
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "MD Block");

        // Filter by Org
        let result = query
            .get_page_blocks("Test Page", Some(BlockFormat::Org))
            .await;
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Org Block");
    }

    #[tokio::test]
    async fn test_get_page_blocks_page_not_found() {
        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get_page_blocks("Non-existent Page", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_page_blocks_empty_page() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Empty Page");

        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get_page_blocks("Empty Page", None).await;
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert!(blocks.is_empty());
    }

    #[tokio::test]
    async fn test_check_exists_returns_true() {
        let block_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![create_test_block(
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            page_id,
            "Block 1",
            1.0,
            BlockFormat::Markdown,
        )];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.check_exists(block_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_check_exists_returns_false() {
        let block_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.check_exists(block_id).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_get_by_page() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![
            create_test_block(
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                page_id,
                "Block 1",
                1.0,
                BlockFormat::Markdown,
            ),
            create_test_block(
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                page_id,
                "Block 2",
                2.0,
                BlockFormat::Markdown,
            ),
        ];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get_by_page(page_id).await;
        assert!(result.is_ok());
        let blocks = result.unwrap();
        assert_eq!(blocks.len(), 2);
    }

    // ── BlockQuery method tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_block_get_found() {
        let block_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let blocks = vec![create_test_block(
            "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
            page_id,
            "Found Block",
            1.0,
            BlockFormat::Markdown,
        )];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get(block_id).await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Found Block");
    }

    #[tokio::test]
    async fn test_block_get_not_found() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_block_get_with_children() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");
        let parent_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let child_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();

        let blocks = vec![
            create_test_block(
                "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                page_id,
                "Parent",
                1.0,
                BlockFormat::Markdown,
            ),
            create_test_block(
                "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                page_id,
                "Child",
                2.0,
                BlockFormat::Markdown,
            ),
        ];

        let block_repo = Arc::new(MockBlockRepository::new(blocks));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        // Override get_children for this test
        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        // Note: get_with_children uses collect_children recursively
        // The mock returns empty for get_children, so we just test the structure
        let result = query.get_with_children(parent_id).await;
        assert!(result.is_ok());
        // Only the parent since mock returns empty children
        assert_eq!(result.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_block_get_with_children_not_found() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        let result = query.get_with_children(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_block_get_backlinks() {
        let block_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        // Mock returns empty backlinks
        let result = query.get_backlinks(block_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_block_search() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let block_repo = Arc::new(MockBlockRepository::new(vec![]));
        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = BlockQuery::new(block_repo.clone(), page_repo.clone());

        // Mock returns empty search results
        let result = query.search("test", 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // ── PageQuery method tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_page_query_get_found() {
        let page_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "Test Page");

        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get(page_id).await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Page");
    }

    #[tokio::test]
    async fn test_page_query_get_not_found() {
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get(Uuid::new_v4()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_page_query_get_by_name_found() {
        let page = create_test_page("11111111-1111-1111-1111-111111111111", "My Special Page");

        let page_repo = Arc::new(MockPageRepository::new(vec![page]));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get_by_name("My Special Page").await;
        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "My Special Page");
    }

    #[tokio::test]
    async fn test_page_query_get_by_name_not_found() {
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get_by_name("Non-existent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_page_query_get_journal() {
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = PageQuery::new(page_repo.clone());

        // Mock returns None for get_journal
        let day = JournalDay::from_i32(20260504).unwrap();
        let result = query.get_journal(day).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_page_query_get_all() {
        let pages = vec![
            create_test_page("11111111-1111-1111-1111-111111111111", "Page A"),
            create_test_page("22222222-2222-2222-2222-222222222222", "Page B"),
        ];

        let page_repo = Arc::new(MockPageRepository::new(pages));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get_all().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_page_query_get_all_empty() {
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = PageQuery::new(page_repo.clone());

        let result = query.get_all().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_page_query_get_recent() {
        let page_repo = Arc::new(MockPageRepository::new(vec![]));

        let query = PageQuery::new(page_repo.clone());

        // Mock returns empty for get_recent
        let result = query.get_recent(10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
