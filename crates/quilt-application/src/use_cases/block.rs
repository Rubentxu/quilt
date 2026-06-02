//! Block use cases
//!
//! Implements [`BlockUseCases`] trait for block CRUD and linking operations.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::{Block, BlockCreate, BlockUpdate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, BlockRepositoryExt, PageRepository};
use quilt_domain::value_objects::{BlockFormat, Priority, PropertyValue, TaskMarker, Uuid};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Block use cases trait - block CRUD and linking operations.
///
/// This trait is object-safe (`Send + Sync`) and uses `#[async_trait]`
/// for async ergonomics. Implementations use generics internally for
/// zero-cost abstraction through monomorphization.
#[async_trait]
pub trait BlockUseCases: Send + Sync {
    /// Create a new block on a page, creating the page if it doesn't exist.
    async fn create_with_page(
        &self,
        page_name: &str,
        content: &str,
        parent_id: Option<Uuid>,
        marker: Option<TaskMarker>,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<Block, ApplicationError>;

    /// Create a new task block on a page.
    async fn create_task(
        &self,
        page_name: &str,
        content: &str,
        deadline: Option<chrono::NaiveDate>,
        priority: Option<&str>,
    ) -> Result<Block, ApplicationError>;

    /// Delete a block by ID.
    async fn delete(&self, block_id: Uuid) -> Result<(), ApplicationError>;

    /// Link two blocks together (add target to source's refs).
    async fn link(&self, source_id: Uuid, target_id: Uuid) -> Result<(), ApplicationError>;

    /// Get a block with its children (tree structure).
    async fn get_tree(&self, block_id: Uuid) -> Result<BlockTree, ApplicationError>;

    /// Get all blocks that link back to this block.
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, ApplicationError>;

    /// List blocks whose `properties` map contains the given `key` mapped
    /// to the given string `value`.
    ///
    /// This is the primary lookup for the `created_by` convention
    /// (`user::name`, `agent::claude`, ...). Returns at most `limit`
    /// blocks, ordered newest first. `limit == 0` means "no limit".
    async fn list_by_property(
        &self,
        key: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<Block>, ApplicationError>;
}

/// Block tree structure returned by [`BlockUseCases::get_tree`].
///
/// Note: Block doesn't implement Serialize/Deserialize, so this is
/// primarily for internal use. Use the individual fields as needed.
#[derive(Debug, Clone)]
pub struct BlockTree {
    /// The root block
    pub root: Block,
    /// Child blocks
    pub children: Vec<Block>,
}

/// Implementation of [`BlockUseCases`] for generic repository types.
///
/// Type parameters:
/// - `BR`: Block repository
/// - `PR`: Page repository
pub struct BlockUseCasesImpl<BR: BlockRepository, PR: PageRepository> {
    block_repo: Arc<BR>,
    page_repo: Arc<PR>,
}

impl<BR: BlockRepository, PR: PageRepository> BlockUseCasesImpl<BR, PR> {
    /// Create a new BlockUseCasesImpl instance.
    pub fn new(block_repo: Arc<BR>, page_repo: Arc<PR>) -> Self {
        Self {
            block_repo,
            page_repo,
        }
    }

    /// Resolve a page name to a page, creating it if it doesn't exist.
    async fn get_or_create_page(&self, page_name: &str) -> Result<Page, ApplicationError> {
        // Try to find existing page
        if let Some(page) = self
            .page_repo
            .get_by_name(page_name)
            .await
            .map_err(ApplicationError::Domain)?
        {
            return Ok(page);
        }

        // Create new page
        let page = Page::new(PageCreate {
            name: page_name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        })
        .map_err(ApplicationError::Domain)?;

        self.page_repo
            .insert(&page)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(page)
    }
}

#[async_trait]
impl<BR: BlockRepository + 'static, PR: PageRepository + 'static> BlockUseCases
    for BlockUseCasesImpl<BR, PR>
{
    #[instrument(skip(self))]
    async fn create_with_page(
        &self,
        page_name: &str,
        content: &str,
        parent_id: Option<Uuid>,
        marker: Option<TaskMarker>,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<Block, ApplicationError> {
        // Find or create the page
        let page = self.get_or_create_page(page_name).await?;

        // Create the block
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id,
            order: 1.0,
            marker,
            format: BlockFormat::Markdown,
            properties,
        })
        .map_err(ApplicationError::Domain)?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    #[instrument(skip(self))]
    async fn create_task(
        &self,
        page_name: &str,
        content: &str,
        deadline: Option<chrono::NaiveDate>,
        priority: Option<&str>,
    ) -> Result<Block, ApplicationError> {
        // Find or create the page
        let page = self.get_or_create_page(page_name).await?;

        // Build properties with deadline if provided
        let mut properties = HashMap::new();
        if let Some(deadline_date) = deadline {
            let deadline_dt = deadline_date
                .and_hms_opt(0, 0, 0)
                .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc));
            if let Some(dt) = deadline_dt {
                properties.insert("deadline".to_string(), PropertyValue::Date(dt));
            }
        }

        // Parse priority if provided
        let priority_value = priority.and_then(|p| match p.to_uppercase().as_str() {
            "A" => Some(Priority::A),
            "B" => Some(Priority::B),
            "C" => Some(Priority::C),
            _ => None,
        });

        // Create the block with TaskMarker::Todo and priority
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: Some(TaskMarker::Todo),
            format: BlockFormat::Markdown,
            properties,
        })
        .map_err(ApplicationError::Domain)?;

        // Always insert the block first
        let mut block = block;
        self.block_repo
            .insert(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        // Update with priority if set
        if let Some(pri) = priority_value {
            block
                .update(BlockUpdate {
                    priority: Some(pri),
                    ..Default::default()
                })
                .map_err(ApplicationError::Domain)?;

            self.block_repo
                .update(&block)
                .await
                .map_err(ApplicationError::Domain)?;
        }

        Ok(block)
    }

    #[instrument(skip(self))]
    async fn delete(&self, block_id: Uuid) -> Result<(), ApplicationError> {
        // Verify block exists
        self.block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        self.block_repo
            .delete(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn link(&self, source_id: Uuid, target_id: Uuid) -> Result<(), ApplicationError> {
        // Verify both blocks exist
        let mut source = self
            .block_repo
            .get_or_fail(source_id)
            .await
            .map_err(ApplicationError::Domain)?;

        self.block_repo
            .get_or_fail(target_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Add reference if not already present
        if !source.refs.contains(&target_id) {
            source.add_ref(target_id);
            self.block_repo
                .update(&source)
                .await
                .map_err(ApplicationError::Domain)?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_tree(&self, block_id: Uuid) -> Result<BlockTree, ApplicationError> {
        let root = self
            .block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        let children = self
            .block_repo
            .get_children(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(BlockTree { root, children })
    }

    #[instrument(skip(self))]
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, ApplicationError> {
        self.block_repo
            .get_backlinks(block_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_by_property(
        &self,
        key: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<Block>, ApplicationError> {
        self.block_repo
            .list_by_property(key, value, limit)
            .await
            .map_err(ApplicationError::Domain)
    }
}
