//! BlockRepository trait - abstraction for block data access

use crate::entities::Block;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// BlockRepository is the abstraction for block data access.
///
/// Implementations (like SqliteBlockRepository) implement this trait,
/// allowing the domain to be independent of the storage mechanism.
#[async_trait]
pub trait BlockRepository: Send + Sync {
    /// Get a block by its ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError>;

    /// Get all blocks belonging to a page
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Get direct children of a block
    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Get a block with its references
    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError>;

    /// Insert a new block
    async fn insert(&self, block: &Block) -> Result<(), DomainError>;

    /// Update an existing Block
    async fn update(&self, block: &Block) -> Result<(), DomainError>;

    /// Soft-delete a block by ID (sets deleted_at timestamp)
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Hard-delete a block by ID (permanent removal from database)
    async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Restore a soft-deleted block (sets deleted_at to NULL)
    async fn restore(&self, id: Uuid) -> Result<(), DomainError>;

    /// Get blocks that were soft-deleted since a given timestamp
    async fn get_deleted_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Block>, DomainError>;

    /// Get all soft-deleted blocks (recycle bin)
    ///
    /// Returns all blocks where deleted_at is not NULL, ordered by deletion time.
    async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError>;

    /// Move a block to a new parent with new order
    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError>;

    /// Get all blocks that reference a given block (backlinks)
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError>;

    /// Search blocks by content (full-text or fuzzy)
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError>;

    /// Get all blocks updated since a given timestamp
    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Block>, DomainError>;

    /// Get the count of blocks on a page
    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError>;
}

/// BlockRepositoryExt provides additional convenience methods
#[async_trait]
pub trait BlockRepositoryExt: BlockRepository {
    /// Check if a block exists (excluding soft-deleted)
    async fn exists(&self, id: Uuid) -> Result<bool, DomainError> {
        Ok(self.get_by_id(id).await?.is_some())
    }

    /// Get a block or fail with an error
    async fn get_or_fail(&self, id: Uuid) -> Result<Block, DomainError> {
        self.get_by_id(id)
            .await?
            .ok_or(DomainError::BlockNotFound(id))
    }

    /// Soft-delete a block or fail if not found
    async fn soft_delete_or_fail(&self, id: Uuid) -> Result<(), DomainError> {
        self.get_or_fail(id).await?;
        self.delete(id).await
    }
}

impl<T: BlockRepository + ?Sized> BlockRepositoryExt for T {}
