//! DeepLinkRepository trait - abstraction for deep link data access

use crate::entities::{DeepLink, LinkSourceType, LinkType};
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// DeepLinkRepository is the abstraction for deep link data access.
///
/// Implementations (like SqliteDeepLinkRepository) implement this trait,
/// allowing the domain to be independent of the storage mechanism.
#[async_trait]
pub trait DeepLinkRepository: Send + Sync {
    /// Get a deep link by its ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<DeepLink>, DomainError>;

    /// Get all deep links from a source block/page
    async fn get_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<Vec<DeepLink>, DomainError>;

    /// Get all deep links to a target block/page
    async fn get_by_target(&self, target_id: Uuid) -> Result<Vec<DeepLink>, DomainError>;

    /// Get all deep links of a specific type
    async fn get_by_type(&self, link_type: LinkType) -> Result<Vec<DeepLink>, DomainError>;

    /// Get all deep links from a source with a specific link type
    async fn get_by_source_and_type(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
        link_type: LinkType,
    ) -> Result<Vec<DeepLink>, DomainError>;

    /// Insert a new deep link
    async fn insert(&self, link: &DeepLink) -> Result<(), DomainError>;

    /// Delete a deep link by ID
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Delete all deep links from a source
    async fn delete_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<(), DomainError>;

    /// Delete all deep links to a target
    async fn delete_by_target(&self, target_id: Uuid) -> Result<(), DomainError>;

    /// Get the count of deep links from a source
    async fn count_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<usize, DomainError>;

    /// Get the count of deep links to a target
    async fn count_by_target(&self, target_id: Uuid) -> Result<usize, DomainError>;

    /// Get all deep links (for a specific page/block) paginated
    async fn get_page(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<DeepLink>, DomainError>;

    /// Search deep links by link text
    async fn search_by_text(&self, query: &str, limit: usize)
        -> Result<Vec<DeepLink>, DomainError>;
}

/// DeepLinkRepositoryExt provides additional convenience methods
#[async_trait]
pub trait DeepLinkRepositoryExt: DeepLinkRepository {
    /// Check if a deep link exists
    async fn exists(&self, id: Uuid) -> Result<bool, DomainError> {
        Ok(self.get_by_id(id).await?.is_some())
    }

    /// Get a deep link or fail with an error
    async fn get_or_fail(&self, id: Uuid) -> Result<DeepLink, DomainError> {
        self.get_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("DeepLink not found: {}", id)))
    }
}

impl<T: DeepLinkRepository + ?Sized> DeepLinkRepositoryExt for T {}
