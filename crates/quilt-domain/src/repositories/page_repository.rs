//! PageRepository trait - abstraction for page data access

use crate::entities::Page;
use crate::errors::DomainError;
use crate::properties::entry::DefaultPropertyEntry;
use crate::value_objects::{JournalDay, PropertyValue, Uuid};
use async_trait::async_trait;
use std::collections::HashMap;

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

    /// Update the typed properties of a page (F5 + F8 + F9).
    ///
    /// Behavior:
    /// 1. Load the existing page (return `PageNotFound` if missing).
    /// 2. For each incoming key, resolve it via `PropertyKeyResolver` and
    ///    reject any key whose `PropertyDefinition.read_only == true` with
    ///    `DomainError::PropertyReadOnly(<key>)`. Rejection is atomic —
    ///    a single bad key fails the entire call with no partial write.
    /// 3. Merge `props` into the existing `page.properties` map using
    ///    `merge_properties` (LWW by timestamp per key).
    /// 4. Persist the merged map.
    /// 5. Return the updated `Page`.
    async fn update_properties(
        &self,
        page_id: Uuid,
        props: HashMap<String, DefaultPropertyEntry<PropertyValue>>,
    ) -> Result<Page, DomainError>;
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
