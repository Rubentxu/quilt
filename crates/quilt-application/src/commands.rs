//! Application commands (CQRS - Write)
//!
//! This module provides command handlers for modifying domain entities.
//! Commands follow the CQRS pattern by encapsulating all write operations.

use crate::errors::ApplicationError;
use quilt_domain::entities::{Block, BlockCreate, BlockUpdate, Page, PageCreate, UserSettings};
use quilt_domain::content::BlockContent;
use quilt_domain::repositories::{
    BlockRepository, BlockRepositoryExt, PageRepository, PageRepositoryExt, SettingsRepository,
};
use quilt_domain::services::{OutlinerService, TimezoneService};
use quilt_domain::value_objects::{BlockFormat, JournalDay, TaskMarker, Uuid};
use std::sync::Arc;
use tracing::instrument;

/// Command handler for block write operations.
///
/// Encapsulates all operations that modify block entities:
/// - Creating new blocks
/// - Updating existing blocks
/// - Deleting blocks
/// - Setting task markers on blocks
///
/// # Type Parameters
///
/// - `R`: The block repository implementation
pub struct BlockCommand<R: BlockRepository> {
    repository: Arc<R>,
    timezone: Arc<TimezoneService>,
}

impl<R: BlockRepository> BlockCommand<R> {
    /// Creates a new `BlockCommand` handler with the given repository and timezone.
    ///
    /// # Arguments
    ///
    /// * `repository` - An `Arc`-wrapped repository for block persistence
    /// * `timezone` - An `Arc`-wrapped timezone service for journal day calculation
    pub fn new(repository: Arc<R>, timezone: Arc<TimezoneService>) -> Self {
        Self {
            repository,
            timezone,
        }
    }

    /// Creates a new block within a page.
    ///
    /// The block is created with Markdown format and default empty properties.
    /// The block is assigned `order: 1.0` (future versions should calculate proper ordering).
    /// The journal_day is automatically set based on the user's timezone.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the parent page
    /// * `content` - The block's content as a string
    /// * `parent_id` - Optional UUID of the parent block for nested blocks
    ///
    /// # Returns
    ///
    /// Returns the newly created [`Block`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Domain validation fails
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn create(
        &self,
        page_id: Uuid,
        content: String,
        parent_id: Option<Uuid>,
    ) -> Result<Block, ApplicationError> {
        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text(content),
            parent_id,
            order: 1.0, // TODO: Calculate proper order
            marker: None,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        };

        let block = Block::new(create, &self.timezone).map_err(ApplicationError::Domain)?;

        self.repository
            .insert(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    /// Updates an existing block with new data.
    ///
    /// The updated_journal_day is automatically updated based on the user's timezone.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to update
    /// * `update` - The [`BlockUpdate`] containing fields to modify
    ///
    /// # Returns
    ///
    /// Returns the updated [`Block`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Block with given ID does not exist
    /// - Domain validation fails on update
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn update(
        &self,
        block_id: Uuid,
        update: BlockUpdate,
    ) -> Result<Block, ApplicationError> {
        let mut block = self
            .repository
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        block
            .update(update, &self.timezone)
            .map_err(ApplicationError::Domain)?;

        self.repository
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    /// Deletes a block by ID (soft-delete).
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to delete
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    ///
    /// # Note
    ///
    /// This does not check for or delete child blocks.
    /// The block can be restored using [`BlockCommand::restore`].
    #[instrument(skip(self))]
    pub async fn delete(&self, block_id: Uuid) -> Result<(), ApplicationError> {
        // TODO: Check for children
        self.repository
            .delete(block_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }

    /// Permanently deletes a block by ID (hard-delete).
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to permanently delete
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    ///
    /// # Warning
    ///
    /// This operation is irreversible. Use [`BlockCommand::delete`] for
    /// reversible deletion.
    #[instrument(skip(self))]
    pub async fn hard_delete(&self, block_id: Uuid) -> Result<(), ApplicationError> {
        self.repository
            .hard_delete(block_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }

    /// Restores a soft-deleted block.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to restore
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn restore(&self, block_id: Uuid) -> Result<(), ApplicationError> {
        self.repository
            .restore(block_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }

    /// Sets a task marker (e.g., todo, done) on a block.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block to modify
    /// * `marker` - The [`TaskMarker`] to set (e.g., `TaskMarker::Todo`, `TaskMarker::Done`)
    ///
    /// # Returns
    ///
    /// Returns the updated [`Block`] with the new marker.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Block with given ID does not exist
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn set_marker(
        &self,
        block_id: Uuid,
        marker: TaskMarker,
    ) -> Result<Block, ApplicationError> {
        let mut block = self
            .repository
            .get_or_fail(block_id)
            .await
            .map_err(ApplicationError::Domain)?;

        block
            .update(
                BlockUpdate {
                    marker: Some(marker),
                    ..Default::default()
                },
                &self.timezone,
            )
            .map_err(ApplicationError::Domain)?;

        self.repository
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(block)
    }

    /// Move a block to a new parent and position among siblings.
    ///
    /// Calculates the new fractional order using [`OutlinerService::calculate_order`]
    /// based on the target siblings and desired position. Validates the move is legal
    /// (no circular references) before persisting.
    ///
    /// # Arguments
    /// * `block_id` - The block to move
    /// * `new_parent` - The target parent (None for top-level on current page)
    /// * `position` - Position among siblings (0 = first, >= len = last)
    ///
    /// # Errors
    /// Returns [`ApplicationError::Domain`] if the move would create a circular
    /// reference or if the block is not found.
    #[instrument(skip(self))]
    pub async fn handle_move(
        &self,
        block_id: Uuid,
        new_parent: Option<Uuid>,
        position: usize,
    ) -> Result<(), ApplicationError> {
        // 1. Fetch the block or fail
        let mut block = self.repository.get_or_fail(block_id).await?;

        // 2. Get target siblings
        let siblings = match new_parent {
            Some(pid) => self.repository.get_children(pid).await?,
            None => self.repository.get_by_page(block.page_id).await?,
        };

        // 3. Validate no circular reference
        OutlinerService::validate_move(&block, new_parent, &siblings)?;

        // 4. Calculate new order
        let orders: Vec<f64> = siblings.iter().map(|b| b.order).collect();
        let new_order = OutlinerService::calculate_order(&orders, position);

        // 5. Update block fields
        block.parent_id = new_parent;
        block.order = new_order;
        block.level = match new_parent {
            Some(_) => siblings.first().map(|s| s.level + 1).unwrap_or(2),
            None => 1,
        };

        // 6. Persist the move
        self.repository
            .move_block(block_id, new_parent, new_order)
            .await?;

        // 7. Persist the level change (and any other updated fields)
        self.repository
            .update(&block)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(())
    }
}

/// Command handler for user settings operations.
pub struct SettingsCommand<R: SettingsRepository> {
    repository: Arc<R>,
}

impl<R: SettingsRepository> SettingsCommand<R> {
    /// Creates a new `SettingsCommand` handler with the given repository.
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Update user settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The new user settings to persist
    ///
    /// # Returns
    ///
    /// Returns the updated [`UserSettings`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Validation fails
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn update(&self, settings: UserSettings) -> Result<UserSettings, ApplicationError> {
        // Validate settings first
        settings.validate().map_err(ApplicationError::Domain)?;

        self.repository
            .update_user_settings(&settings)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(settings)
    }

    /// Reset settings to defaults.
    #[instrument(skip(self))]
    pub async fn reset_to_defaults(&self) -> Result<UserSettings, ApplicationError> {
        let defaults = UserSettings::default();

        self.repository
            .update_user_settings(&defaults)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(defaults)
    }
}

/// Command handler for page write operations.
///
/// Encapsulates all operations that modify page entities:
/// - Creating new pages
/// - Creating journal pages
/// - Renaming pages
/// - Deleting pages
///
/// # Type Parameters
///
/// - `R`: The page repository implementation
pub struct PageCommand<R: PageRepository> {
    repository: Arc<R>,
}

impl<R: PageRepository> PageCommand<R> {
    /// Creates a new `PageCommand` handler with the given repository.
    ///
    /// # Arguments
    ///
    /// * `repository` - An `Arc`-wrapped repository for page persistence
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Creates a new page with the given name and optional metadata.
    ///
    /// # Arguments
    ///
    /// * `name` - The unique name identifier for the page
    /// * `title` - Optional display title (defaults to name if None)
    /// * `namespace_id` - Optional namespace UUID for organizing pages
    ///
    /// # Returns
    ///
    /// Returns the newly created [`Page`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Domain validation fails
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn create(
        &self,
        name: String,
        title: Option<String>,
        namespace_id: Option<Uuid>,
    ) -> Result<Page, ApplicationError> {
        let create = PageCreate {
            name,
            title,
            namespace_id,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        };

        let page = Page::new(create).map_err(ApplicationError::Domain)?;

        self.repository
            .insert(&page)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(page)
    }

    /// Creates a new journal page for the specified day.
    ///
    /// Journal pages are special pages associated with a specific calendar date.
    ///
    /// # Arguments
    ///
    /// * `day` - The [`JournalDay`] representing the date for this journal entry
    ///
    /// # Returns
    ///
    /// Returns the newly created journal [`Page`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Domain validation fails
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn create_journal(&self, day: JournalDay) -> Result<Page, ApplicationError> {
        let page =
            Page::new_journal(day, BlockFormat::Markdown).map_err(ApplicationError::Domain)?;

        self.repository
            .insert(&page)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(page)
    }

    /// Renames a page to a new name.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page to rename
    /// * `new_name` - The new name for the page
    ///
    /// # Returns
    ///
    /// Returns the renamed [`Page`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Page with given ID does not exist
    /// - Repository rename operation fails
    #[instrument(skip(self))]
    pub async fn rename(&self, page_id: Uuid, new_name: &str) -> Result<Page, ApplicationError> {
        self.repository
            .rename(page_id, new_name)
            .await
            .map_err(ApplicationError::Domain)?;

        self.repository
            .get_or_fail(page_id)
            .await
            .map_err(ApplicationError::Domain)
    }

    /// Deletes a page by ID.
    ///
    /// This is a soft-delete that sets the deleted_at timestamp.
    /// Use [`PageCommand::hard_delete`] for permanent removal.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page to delete
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn delete(&self, page_id: Uuid) -> Result<(), ApplicationError> {
        self.repository
            .delete(page_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }

    /// Permanently deletes a page by ID (hard-delete).
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page to permanently delete
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    ///
    /// # Warning
    ///
    /// This operation is irreversible. Use [`PageCommand::delete`] for
    /// reversible deletion.
    #[instrument(skip(self))]
    pub async fn hard_delete(&self, page_id: Uuid) -> Result<(), ApplicationError> {
        self.repository
            .hard_delete(page_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }

    /// Restores a soft-deleted page.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page to restore
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if:
    /// - Repository operation fails
    #[instrument(skip(self))]
    pub async fn restore(&self, page_id: Uuid) -> Result<(), ApplicationError> {
        self.repository
            .restore(page_id)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(())
    }
}

/// Type alias for `BlockCommand` implementing the command handler pattern.
///
/// This is the concrete type returned by command factory functions and
/// used in dependency injection for request handlers.
pub type BlockCommandHandler<R> = BlockCommand<R>;

/// Type alias for `PageCommand` implementing the command handler pattern.
///
/// This is the concrete type returned by command factory functions and
/// used in dependency injection for request handlers.
pub type PageCommandHandler<R> = PageCommand<R>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ApplicationError;
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock BlockRepository for testing
    #[derive(Clone)]
    struct MockBlockRepository {
        blocks: Arc<Mutex<HashMap<Uuid, Block>>>,
    }

    impl MockBlockRepository {
        fn new(blocks: Vec<Block>) -> Self {
            Self {
                blocks: Arc::new(Mutex::new(blocks.into_iter().map(|b| (b.id, b)).collect())),
            }
        }
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepository {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
            Ok(self.blocks.lock().unwrap().get(&id).cloned())
        }

        async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            let blocks: Vec<Block> = self
                .blocks
                .lock()
                .unwrap()
                .values()
                .filter(|b| b.page_id == page_id && b.parent_id.is_none())
                .cloned()
                .collect();
            Ok(blocks)
        }

        async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            let blocks: Vec<Block> = self
                .blocks
                .lock()
                .unwrap()
                .values()
                .filter(|b| b.parent_id == Some(parent_id))
                .cloned()
                .collect();
            Ok(blocks)
        }

        async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            let block = self
                .blocks
                .lock()
                .unwrap()
                .get(&id)
                .cloned()
                .ok_or(DomainError::BlockNotFound(id))?;
            Ok((block, Vec::new()))
        }

        async fn insert(&self, block: &Block) -> Result<(), DomainError> {
            self.blocks.lock().unwrap().insert(block.id, block.clone());
            Ok(())
        }

        async fn update(&self, block: &Block) -> Result<(), DomainError> {
            if !self.blocks.lock().unwrap().contains_key(&block.id) {
                return Err(DomainError::BlockNotFound(block.id));
            }
            self.blocks.lock().unwrap().insert(block.id, block.clone());
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
            self.blocks.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError> {
            self.blocks.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn move_block(
            &self,
            id: Uuid,
            new_parent: Option<Uuid>,
            new_order: f64,
        ) -> Result<(), DomainError> {
            let mut blocks = self.blocks.lock().unwrap();
            let block = blocks.get_mut(&id).ok_or(DomainError::BlockNotFound(id))?;
            block.parent_id = new_parent;
            block.order = new_order;
            Ok(())
        }

        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            Ok(0)
        }

        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }

        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError> {
            Ok(Vec::new())
        }
    }

    fn test_timezone() -> Arc<TimezoneService> {
        Arc::new(TimezoneService::from_tz_string("UTC").expect("Failed to create test timezone"))
    }

    fn create_test_block(
        id: Uuid,
        page_id: Uuid,
        parent_id: Option<Uuid>,
        order: f64,
        level: u8,
    ) -> Block {
        Block {
            id,
            page_id,
            parent_id,
            order,
            level,
            content: String::new(),
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
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

    #[tokio::test]
    async fn test_handle_move_happy_path() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let block_to_move = create_test_block(Uuid::new_v4(), page_id, None, 100.0, 1);
        let sibling1 = create_test_block(Uuid::new_v4(), page_id, Some(parent_id), 100.0, 2);
        let sibling2 = create_test_block(Uuid::new_v4(), page_id, Some(parent_id), 200.0, 2);

        let repo = MockBlockRepository::new(vec![
            block_to_move.clone(),
            sibling1.clone(),
            sibling2.clone(),
        ]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        // Move block_to_move to position 1 among siblings (after first sibling)
        let result = command
            .handle_move(block_to_move.id, Some(parent_id), 1)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_move_to_top_level() {
        let page_id = Uuid::new_v4();
        let block_to_move =
            create_test_block(Uuid::new_v4(), page_id, Some(Uuid::new_v4()), 100.0, 2);

        let repo = MockBlockRepository::new(vec![block_to_move.clone()]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        // Move block_to_move to top-level (new_parent = None)
        let result = command.handle_move(block_to_move.id, None, 0).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_move_block_not_found() {
        let repo = MockBlockRepository::new(vec![]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command.handle_move(Uuid::new_v4(), None, 0).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::Domain(DomainError::BlockNotFound(_)) => (),
            other => panic!("Expected DomainError::BlockNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_handle_move_level_recalculation() {
        let page_id = Uuid::new_v4();
        let new_parent = Uuid::new_v4();
        let block_to_move = create_test_block(Uuid::new_v4(), page_id, None, 100.0, 1);
        let sibling = create_test_block(Uuid::new_v4(), page_id, Some(new_parent), 100.0, 2);

        let repo = MockBlockRepository::new(vec![block_to_move.clone(), sibling.clone()]);
        let repo_clone = repo.clone();
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        // Move to a parent - level should become parent's level + 1
        let result = command
            .handle_move(block_to_move.id, Some(new_parent), 0)
            .await;

        assert!(result.is_ok());
        // Verify the block was updated correctly
        let repo_blocks = repo_clone.blocks.lock().unwrap();
        let moved_block = repo_blocks.get(&block_to_move.id).unwrap();
        assert_eq!(moved_block.parent_id, Some(new_parent));
        assert_eq!(moved_block.level, sibling.level + 1);
    }

    #[tokio::test]
    async fn test_handle_move_top_level_level_recalculation() {
        let page_id = Uuid::new_v4();
        let block_to_move =
            create_test_block(Uuid::new_v4(), page_id, Some(Uuid::new_v4()), 100.0, 3);

        let repo = MockBlockRepository::new(vec![block_to_move.clone()]);
        let repo_clone = repo.clone();
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        // Move to top-level (None parent) - level should become 1
        let result = command.handle_move(block_to_move.id, None, 0).await;

        assert!(result.is_ok());
        let repo_blocks = repo_clone.blocks.lock().unwrap();
        let moved_block = repo_blocks.get(&block_to_move.id).unwrap();
        assert_eq!(moved_block.parent_id, None);
        assert_eq!(moved_block.level, 1);
    }

    // ── BlockCommand::create tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_block_create_success() {
        let page_id = Uuid::new_v4();
        let repo = MockBlockRepository::new(vec![]);
        let repo_clone = repo.clone();
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command
            .create(page_id, "Test content".to_string(), None)
            .await;

        assert!(result.is_ok());
        let block = result.unwrap();
        assert_eq!(block.page_id, page_id);
        assert_eq!(block.content, "Test content");
        assert_eq!(block.format, BlockFormat::Markdown);

        // Verify it was inserted
        let blocks = repo_clone.blocks.lock().unwrap();
        assert!(blocks.contains_key(&block.id));
    }

    #[tokio::test]
    async fn test_block_create_with_parent() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let repo = MockBlockRepository::new(vec![]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command
            .create(page_id, "Child block".to_string(), Some(parent_id))
            .await;

        assert!(result.is_ok());
        let block = result.unwrap();
        assert_eq!(block.page_id, page_id);
        assert_eq!(block.parent_id, Some(parent_id));
    }

    // ── BlockCommand::update tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_block_update_success() {
        let page_id = Uuid::new_v4();
        let existing_block = create_test_block(Uuid::new_v4(), page_id, None, 1.0, 1);
        let repo = MockBlockRepository::new(vec![existing_block.clone()]);
        let repo_clone = repo.clone();
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let update = BlockUpdate {
            content: Some("Updated content".to_string()),
            ..Default::default()
        };
        let result = command.update(existing_block.id, update).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert_eq!(updated.content, "Updated content");

        // Verify it was updated
        let blocks = repo_clone.blocks.lock().unwrap();
        assert_eq!(
            blocks.get(&existing_block.id).unwrap().content,
            "Updated content"
        );
    }

    #[tokio::test]
    async fn test_block_update_not_found() {
        let repo = MockBlockRepository::new(vec![]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let update = BlockUpdate {
            content: Some("New content".to_string()),
            ..Default::default()
        };
        let result = command.update(Uuid::new_v4(), update).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::Domain(DomainError::BlockNotFound(_)) => (),
            other => panic!("Expected BlockNotFound, got {:?}", other),
        }
    }

    // ── BlockCommand::delete tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_block_delete_success() {
        let page_id = Uuid::new_v4();
        let block_id = Uuid::new_v4();
        let existing_block = create_test_block(block_id, page_id, None, 1.0, 1);
        let repo = MockBlockRepository::new(vec![existing_block]);
        let repo_clone = repo.clone();
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command.delete(block_id).await;

        assert!(result.is_ok());

        // Verify it was deleted
        let blocks = repo_clone.blocks.lock().unwrap();
        assert!(!blocks.contains_key(&block_id));
    }

    #[tokio::test]
    async fn test_block_delete_idempotent() {
        // Deleting a non-existent block should succeed (idempotent)
        let repo = MockBlockRepository::new(vec![]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command.delete(Uuid::new_v4()).await;

        // delete doesn't fail on not found - it just succeeds
        assert!(result.is_ok());
    }

    // ── BlockCommand::set_marker tests ─────────────────────────────────

    #[tokio::test]
    async fn test_block_set_marker_todo() {
        let page_id = Uuid::new_v4();
        let existing_block = create_test_block(Uuid::new_v4(), page_id, None, 1.0, 1);
        let repo = MockBlockRepository::new(vec![existing_block.clone()]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command
            .set_marker(existing_block.id, TaskMarker::Todo)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().marker, Some(TaskMarker::Todo));
    }

    #[tokio::test]
    async fn test_block_set_marker_done() {
        let page_id = Uuid::new_v4();
        let existing_block = create_test_block(Uuid::new_v4(), page_id, None, 1.0, 1);
        let repo = MockBlockRepository::new(vec![existing_block.clone()]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command
            .set_marker(existing_block.id, TaskMarker::Done)
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().marker, Some(TaskMarker::Done));
    }

    #[tokio::test]
    async fn test_block_set_marker_not_found() {
        let repo = MockBlockRepository::new(vec![]);
        let command = BlockCommand::new(Arc::new(repo), test_timezone());

        let result = command.set_marker(Uuid::new_v4(), TaskMarker::Todo).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::Domain(DomainError::BlockNotFound(_)) => (),
            other => panic!("Expected BlockNotFound, got {:?}", other),
        }
    }

    // ── PageCommand tests ───────────────────────────────────────────────

    /// Mock PageRepository for testing
    #[derive(Clone)]
    struct MockPageRepository {
        pages: Arc<Mutex<HashMap<Uuid, Page>>>,
    }

    impl MockPageRepository {
        fn new(pages: Vec<Page>) -> Self {
            Self {
                pages: Arc::new(Mutex::new(pages.into_iter().map(|p| (p.id, p)).collect())),
            }
        }
    }

    #[async_trait]
    impl PageRepository for MockPageRepository {
        async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
            Ok(self.pages.lock().unwrap().get(&id).cloned())
        }

        async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
            Ok(self
                .pages
                .lock()
                .unwrap()
                .values()
                .find(|p| p.name == name)
                .cloned())
        }

        async fn get_journal(&self, _day: JournalDay) -> Result<Option<Page>, DomainError> {
            Ok(None)
        }

        async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
            Ok(self.pages.lock().unwrap().values().cloned().collect())
        }

        async fn get_namespace_pages(&self, _namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn insert(&self, page: &Page) -> Result<(), DomainError> {
            self.pages.lock().unwrap().insert(page.id, page.clone());
            Ok(())
        }

        async fn update(&self, page: &Page) -> Result<(), DomainError> {
            if !self.pages.lock().unwrap().contains_key(&page.id) {
                return Err(DomainError::PageNotFound(page.id));
            }
            self.pages.lock().unwrap().insert(page.id, page.clone());
            Ok(())
        }

        async fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError> {
            let mut pages = self.pages.lock().unwrap();
            let page = pages.get_mut(&id).ok_or(DomainError::PageNotFound(id))?;
            page.name = new_name.to_string();
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
            self.pages.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
            self.pages.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError> {
            self.pages.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
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
            Ok(self.pages.lock().unwrap().len())
        }

        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }

        async fn get_orphan_pages(&self) -> Result<Vec<Page>, DomainError> {
            Ok(vec![])
        }
    }

    fn create_test_page(name: &str) -> Page {
        use quilt_domain::entities::PageCreate;
        let create = PageCreate {
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        };
        Page::new(create).expect("Failed to create test page")
    }

    #[tokio::test]
    async fn test_page_create_success() {
        let repo = MockPageRepository::new(vec![]);
        let repo_clone = repo.clone();
        let command = PageCommand::new(Arc::new(repo));

        let result = command.create("test-page".to_string(), None, None).await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.name, "test-page");
        assert!(page.title.is_none());
        assert!(!page.journal);

        // Verify it was inserted
        let pages = repo_clone.pages.lock().unwrap();
        assert!(pages.contains_key(&page.id));
    }

    #[tokio::test]
    async fn test_page_create_with_title() {
        let repo = MockPageRepository::new(vec![]);
        let command = PageCommand::new(Arc::new(repo));

        let result = command
            .create(
                "my-page".to_string(),
                Some("My Page Title".to_string()),
                None,
            )
            .await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.name, "my-page");
        assert_eq!(page.title, Some("My Page Title".to_string()));
    }

    #[tokio::test]
    async fn test_page_create_journal() {
        let repo = MockPageRepository::new(vec![]);
        let repo_clone = repo.clone();
        let command = PageCommand::new(Arc::new(repo));

        let day = JournalDay::from_i32(20260504).unwrap();
        let result = command.create_journal(day).await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert!(page.journal);
        assert_eq!(page.journal_day, Some(day));

        // Verify it was inserted
        let pages = repo_clone.pages.lock().unwrap();
        assert!(pages.contains_key(&page.id));
    }

    #[tokio::test]
    async fn test_page_rename_success() {
        let page = create_test_page("old-name");
        let repo = MockPageRepository::new(vec![page.clone()]);
        let repo_clone = repo.clone();
        let command = PageCommand::new(Arc::new(repo));

        let result = command.rename(page.id, "new-name").await;

        assert!(result.is_ok());
        let renamed = result.unwrap();
        assert_eq!(renamed.name, "new-name");

        // Verify the rename persisted
        let pages = repo_clone.pages.lock().unwrap();
        assert_eq!(pages.get(&page.id).unwrap().name, "new-name");
    }

    #[tokio::test]
    async fn test_page_rename_not_found() {
        let repo = MockPageRepository::new(vec![]);
        let command = PageCommand::new(Arc::new(repo));

        let result = command.rename(Uuid::new_v4(), "new-name").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::Domain(DomainError::PageNotFound(_)) => (),
            other => panic!("Expected PageNotFound, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_page_delete_success() {
        let page = create_test_page("to-delete");
        let repo = MockPageRepository::new(vec![page.clone()]);
        let repo_clone = repo.clone();
        let command = PageCommand::new(Arc::new(repo));

        let result = command.delete(page.id).await;

        assert!(result.is_ok());

        // Verify it was deleted
        let pages = repo_clone.pages.lock().unwrap();
        assert!(!pages.contains_key(&page.id));
    }

    #[tokio::test]
    async fn test_page_delete_idempotent() {
        // Deleting a non-existent page should succeed
        let repo = MockPageRepository::new(vec![]);
        let command = PageCommand::new(Arc::new(repo));

        let result = command.delete(Uuid::new_v4()).await;

        assert!(result.is_ok());
    }
}
