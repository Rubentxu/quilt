//! TagRepository trait - abstraction for tag data access

use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// TagRepository is the abstraction for tag data access.
#[async_trait]
pub trait TagRepository: Send + Sync {
    /// Get all tags for a page
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<String>, DomainError>;

    /// Get all pages that have a given tag
    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError>;

    /// Add a tag to a page
    async fn add_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError>;

    /// Remove a tag from a page
    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError>;

    /// Get all unique tags in the graph
    async fn get_all_tags(&self) -> Result<Vec<String>, DomainError>;

    /// Get tag counts (for suggestions)
    async fn get_tag_counts(&self) -> Result<Vec<(String, usize)>, DomainError>;

    /// Search tags by prefix
    async fn search_tags(&self, prefix: &str, limit: usize) -> Result<Vec<String>, DomainError>;
}

/// TagRepositoryExt provides additional convenience methods
#[async_trait]
pub trait TagRepositoryExt: TagRepository {
    /// Check if a page has a tag
    async fn page_has_tag(&self, page_id: Uuid, tag: &str) -> Result<bool, DomainError> {
        let tags = self.get_by_page(page_id).await?;
        Ok(tags.contains(&tag.to_string()))
    }

    /// Get all tags with a given prefix
    async fn get_tags_with_prefix(&self, prefix: &str) -> Result<Vec<String>, DomainError> {
        let all_tags = self.get_all_tags().await?;
        Ok(all_tags
            .into_iter()
            .filter(|t| t.starts_with(prefix))
            .collect())
    }
}

impl<T: TagRepository + ?Sized> TagRepositoryExt for T {}
