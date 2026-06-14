//! Block use cases
//!
//! Implements [`BlockUseCases`] trait for block CRUD and linking operations.

use crate::errors::ApplicationError;
use crate::services::ref_service::{parse_refs_from_content, RefServiceTrait};
use async_trait::async_trait;
use quilt_domain::entities::{Block, BlockCreate, BlockUpdate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, BlockRepositoryExt, PageRepository};
use quilt_domain::references::RefType;
use quilt_domain::value_objects::{
    BlockFormat, BlockType, Priority, PropertyValue, TaskMarker, Uuid,
};
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
    /// Create a new block on a page.
    ///
    /// This is the primary method for creating blocks - it handles
    /// page resolution, order calculation, property parsing, and
    /// reference index updates.
    async fn create_block(
        &self,
        page_name: &str,
        content: &str,
        parent_id: Option<Uuid>,
        preceding_block_id: Option<Uuid>,
        marker: Option<TaskMarker>,
        block_type: BlockType,
        created_by: Option<&str>,
        raw_properties: HashMap<String, serde_json::Value>,
    ) -> Result<Block, ApplicationError>;

    /// Create a new block on a page, creating the page if it doesn't exist.
    async fn create_with_page(
        &self,
        page_name: &str,
        content: &str,
        parent_id: Option<Uuid>,
        marker: Option<TaskMarker>,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<Block, ApplicationError>;

    /// Create a new block and insert it after a given sibling block.
    ///
    /// Calculates the new order based on the sibling's order and the next sibling (if any).
    async fn create_with_insert_after(
        &self,
        page_name: &str,
        content: &str,
        after_block_id: Uuid,
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

    /// Update an existing block with the given update.
    async fn update_block(
        &self,
        block_id: Uuid,
        update: BlockUpdate,
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

    /// List distinct author identifiers from block properties.
    ///
    /// Returns unique `author` property values found across all blocks,
    /// optionally filtered by prefix (e.g., `agent::` for AI authors).
    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, ApplicationError>;

    /// Set a property on a block.
    async fn set_property(
        &self,
        block_id: Uuid,
        key: String,
        value: PropertyValue,
    ) -> Result<Block, ApplicationError>;

    /// Delete a property from a block.
    async fn delete_property(
        &self,
        block_id: Uuid,
        key: &str,
    ) -> Result<Block, ApplicationError>;

    /// Get all properties of a block.
    async fn get_properties(
        &self,
        block_id: Uuid,
    ) -> Result<HashMap<String, PropertyValue>, ApplicationError>;
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
/// Uses `Arc<dyn Trait>` for dependency injection following the pattern
/// established with MigrationEngine.
pub struct BlockUseCasesImpl {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    ref_service: Arc<dyn RefServiceTrait>,
}

impl BlockUseCasesImpl {
    /// Create a new BlockUseCasesImpl instance.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        ref_service: Arc<dyn RefServiceTrait>,
    ) -> Self {
        Self {
            block_repo,
            page_repo,
            ref_service,
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
            properties: std::collections::HashMap::new(),
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
impl BlockUseCases for BlockUseCasesImpl {
    #[instrument(skip(self))]
    async fn create_block(
        &self,
        page_name: &str,
        content: &str,
        parent_id: Option<Uuid>,
        preceding_block_id: Option<Uuid>,
        marker: Option<TaskMarker>,
        block_type: BlockType,
        created_by: Option<&str>,
        raw_properties: HashMap<String, serde_json::Value>,
    ) -> Result<Block, ApplicationError> {
        // Find or create the page
        let page = self.get_or_create_page(page_name).await?;

        // Determine order value
        let order = if let Some(preceding_id) = preceding_block_id {
            // Calculate midpoint between preceding block and its next sibling
            let preceding = self
                .block_repo
                .get_or_fail(preceding_id)
                .await
                .map_err(ApplicationError::Domain)?;

            let siblings = self
                .block_repo
                .get_children(preceding.parent_id.unwrap_or(preceding.id))
                .await
                .map_err(ApplicationError::Domain)?;

            let next_sibling = siblings
                .iter()
                .find(|b| b.order > preceding.order);

            match next_sibling {
                Some(next) => (preceding.order + next.order) / 2.0,
                None => preceding.order + 1000.0,
            }
        } else {
            // Calculate max order + 1.0 for append
            let existing_blocks = self
                .block_repo
                .get_by_page(page.id)
                .await
                .map_err(ApplicationError::Domain)?;

            existing_blocks
                .iter()
                .map(|b| b.order)
                .fold(0.0_f64, |a, b| a.max(b)) + 1.0
        };

        // Parse raw_properties into HashMap<String, PropertyValue>
        let mut properties: HashMap<String, PropertyValue> = HashMap::new();
        for (key, value) in raw_properties {
            if let Some(prop_value) = PropertyValue::from_json(&value) {
                properties.insert(key, prop_value);
            }
        }

        // Apply created_by convention unless already set
        if let Some(author) = created_by {
            let trimmed = author.trim();
            if !trimmed.is_empty() && !properties.contains_key("created_by") {
                if let Some(value) =
                    PropertyValue::from_json(&serde_json::Value::String(trimmed.to_string()))
                {
                    properties.insert("created_by".to_string(), value);
                }
            }
        }

        // Create the block
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id,
            order,
            marker,
            format: BlockFormat::Markdown,
            block_type,
            properties,
        })
        .map_err(ApplicationError::Domain)?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        // Update reference index - parse content for [[page]] and ((uuid)) refs
        let parsed = parse_refs_from_content(&block.content);

        // Resolve page names to UUIDs via page repository
        let mut page_refs: Vec<(Uuid, RefType)> = Vec::new();
        for name in &parsed.page_names {
            if let Ok(Some(p)) = self.page_repo.get_by_name(name).await {
                page_refs.push((p.id, RefType::PageRef));
            }
        }

        // Call ref_service to update the index
        self.ref_service
            .on_block_saved(block.id, &block.content, page_refs)
            .await
            .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;

        Ok(block)
    }

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
            block_type: BlockType::Paragraph,
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
            block_type: BlockType::Paragraph,
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

    #[instrument(skip(self))]
    async fn create_with_insert_after(
        &self,
        page_name: &str,
        content: &str,
        after_block_id: Uuid,
        marker: Option<TaskMarker>,
        properties: HashMap<String, PropertyValue>,
    ) -> Result<Block, ApplicationError> {
        // Find or create the page
        let page = self.get_or_create_page(page_name).await?;

        // Get the after block to find its order
        let after_block = self
            .block_repo
            .get_or_fail(after_block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Get siblings to calculate order (same parent)
        let siblings = self
            .block_repo
            .get_children(after_block.parent_id.unwrap_or(after_block.id))
            .await
            .map_err(ApplicationError::Domain)?;

        // Calculate new order: midpoint between after_block and next sibling
        let after_order = after_block.order;
        let next_sibling = siblings.iter().find(|b| b.order > after_order);
        let new_order = match next_sibling {
            Some(next) => (after_order + next.order) / 2.0,
            None => after_order + 1000.0,
        };

        // Create the block
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id: after_block.parent_id,
            order: new_order,
            marker,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
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
    async fn update_block(
        &self,
        block_id: Uuid,
        update: BlockUpdate,
    ) -> Result<Block, ApplicationError> {
        let mut block = self
            .block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        // Track whether content changed before applying update
        let content_changed = update.content.is_some();

        block.update(update).map_err(ApplicationError::Domain)?;

        self.block_repo
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        // Update reference index if content changed
        if content_changed {
            let parsed = parse_refs_from_content(&block.content);

            // Resolve page names to UUIDs via page repository
            let mut page_refs: Vec<(Uuid, RefType)> = Vec::new();
            for name in &parsed.page_names {
                if let Ok(Some(p)) = self.page_repo.get_by_name(name).await {
                    page_refs.push((p.id, RefType::PageRef));
                }
            }

            self.ref_service
                .on_block_saved(block.id, &block.content, page_refs)
                .await
                .map_err(|e| ApplicationError::Infrastructure(e.to_string()))?;
        }

        Ok(block)
    }

    #[instrument(skip(self))]
    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, ApplicationError> {
        self.block_repo
            .list_distinct_authors(prefix)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self, key, value))]
    async fn set_property(
        &self,
        block_id: Uuid,
        key: String,
        value: PropertyValue,
    ) -> Result<Block, ApplicationError> {
        let mut block = self
            .block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        block.properties.insert(key, value);

        self.block_repo
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    #[instrument(skip(self))]
    async fn delete_property(
        &self,
        block_id: Uuid,
        key: &str,
    ) -> Result<Block, ApplicationError> {
        let mut block = self
            .block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        block.properties.remove(key);

        self.block_repo
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    #[instrument(skip(self))]
    async fn get_properties(
        &self,
        block_id: Uuid,
    ) -> Result<HashMap<String, PropertyValue>, ApplicationError> {
        let block = self
            .block_repo
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block.properties)
    }
}
