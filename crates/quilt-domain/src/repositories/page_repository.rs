//! PageRepository trait - abstraction for page data access

use crate::entities::Page;
use crate::errors::DomainError;
use crate::value_objects::{JournalDay, Uuid};
use async_trait::async_trait;

/// PageRepository is the abstraction for page data access.
#[async_trait]
pub trait PageRepository: Send + Sync {
    /// Get a page by its ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError>;

    /// Get a page by its name (case-insensitive)
    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError>;

    /// Get a journal page by its day
    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError>;

    /// Get all pages
    async fn get_all(&self) -> Result<Vec<Page>, DomainError>;

    /// Get pages in a namespace
    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError>;

    /// Insert a new page
    async fn insert(&self, page: &Page) -> Result<(), DomainError>;

    /// Update an existing page
    async fn update(&self, page: &Page) -> Result<(), DomainError>;

    /// Rename a page
    async fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError>;

    /// Delete a page by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Soft-delete a page by ID (sets deleted_at timestamp)
    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Hard-delete a page by ID (permanent removal from database)
    async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Restore a soft-deleted page (sets deleted_at to NULL)
    async fn restore(&self, id: Uuid) -> Result<(), DomainError>;

    /// Get all soft-deleted pages (recycle bin)
    ///
    /// Returns all pages where deleted_at is not NULL, ordered by deletion time.
    async fn recycle_bin(&self) -> Result<Vec<Page>, DomainError>;

    /// Get pages that were soft-deleted since a given timestamp
    async fn get_deleted_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Page>, DomainError>;

    /// Get pages updated since a given timestamp
    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Page>, DomainError>;

    /// Get recent pages (ordered by updated_at descending)
    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError>;

    /// Get the count of all pages
    async fn count(&self) -> Result<usize, DomainError>;

    /// Search pages by name
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError>;

    /// Get all orphan pages (pages with no blocks)
    ///
    /// An orphan page is one that has no associated blocks.
    /// This is useful for cleanup and data integrity checks.
    async fn get_orphan_pages(&self) -> Result<Vec<Page>, DomainError>;
}

/// PageRepositoryExt provides additional convenience methods
#[async_trait]
pub trait PageRepositoryExt: PageRepository {
    /// Check if a page exists by name
    async fn exists_by_name(&self, name: &str) -> Result<bool, DomainError> {
        Ok(self.get_by_name(name).await?.is_some())
    }

    /// Get a page or fail with an error
    async fn get_or_fail(&self, id: Uuid) -> Result<Page, DomainError> {
        self.get_by_id(id)
            .await?
            .ok_or(DomainError::PageNotFound(id))
    }

    /// Get or create a journal page for a given day
    async fn get_or_create_journal(&self, day: JournalDay) -> Result<Page, DomainError> {
        if let Some(page) = self.get_journal(day).await? {
            return Ok(page);
        }
        // Note: This requires a factory method - implementation may vary
        Err(DomainError::NotFound("Journal page not found".to_string()))
    }
}

impl<T: PageRepository + ?Sized> PageRepositoryExt for T {}
