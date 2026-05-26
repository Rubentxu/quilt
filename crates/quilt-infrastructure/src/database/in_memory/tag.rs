//! In-memory TagRepository implementation for testing.

use async_trait::async_trait;
use parking_lot::RwLock;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::TagRepository;
use quilt_domain::value_objects::Uuid;
use std::collections::HashMap;

/// In-memory TagRepository using HashMap storage.
/// Maps tag names to sets of page IDs.
#[deprecated(
    since = "0.1.0",
    note = "Use `quilt_test_helpers::InMemoryTagRepo` instead"
)]
#[derive(Debug, Default)]
pub struct InMemoryTagRepository {
    /// Maps tag name -> set of page IDs with that tag
    tags: RwLock<HashMap<String, std::collections::HashSet<Uuid>>>,
}

impl InMemoryTagRepository {
    /// Create a new empty in-memory tag repository.
    pub fn new() -> Self {
        Self {
            tags: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TagRepository for InMemoryTagRepository {
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<String>, DomainError> {
        let tags = self.tags.read();
        Ok(tags
            .iter()
            .filter(|(_, page_ids)| page_ids.contains(&page_id))
            .map(|(name, _)| name.clone())
            .collect())
    }

    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError> {
        let tags = self.tags.read();
        Ok(tags
            .get(tag)
            .map(|page_ids| page_ids.iter().cloned().collect())
            .unwrap_or_default())
    }

    async fn add_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        let mut tags = self.tags.write();
        tags.entry(tag.to_string())
            .or_insert_with(std::collections::HashSet::new)
            .insert(page_id);
        Ok(())
    }

    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        let mut tags = self.tags.write();
        if let Some(page_ids) = tags.get_mut(tag) {
            page_ids.remove(&page_id);
            // Clean up empty tag entries
            if page_ids.is_empty() {
                tags.remove(tag);
            }
        }
        Ok(())
    }

    async fn get_all_tags(&self) -> Result<Vec<String>, DomainError> {
        let tags = self.tags.read();
        Ok(tags.keys().cloned().collect())
    }

    async fn get_tag_counts(&self) -> Result<Vec<(String, usize)>, DomainError> {
        let tags = self.tags.read();
        Ok(tags
            .iter()
            .map(|(name, page_ids)| (name.clone(), page_ids.len()))
            .collect())
    }

    async fn search_tags(&self, prefix: &str, limit: usize) -> Result<Vec<String>, DomainError> {
        let tags = self.tags.read();
        Ok(tags
            .keys()
            .filter(|name| name.starts_with(prefix))
            .take(limit)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_get_by_page() {
        let repo = InMemoryTagRepository::new();
        let page_id = Uuid::new_v4();

        repo.add_tag(page_id, "rust").await.unwrap();
        repo.add_tag(page_id, "programming").await.unwrap();

        let tags = repo.get_by_page(page_id).await.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
    }

    #[tokio::test]
    async fn test_get_pages_with_tag() {
        let repo = InMemoryTagRepository::new();
        let page1 = Uuid::new_v4();
        let page2 = Uuid::new_v4();

        repo.add_tag(page1, "rust").await.unwrap();
        repo.add_tag(page2, "rust").await.unwrap();
        repo.add_tag(page1, "python").await.unwrap();

        let pages = repo.get_pages_with_tag("rust").await.unwrap();
        assert_eq!(pages.len(), 2);
        assert!(pages.contains(&page1));
        assert!(pages.contains(&page2));

        let pages2 = repo.get_pages_with_tag("python").await.unwrap();
        assert_eq!(pages2.len(), 1);
        assert!(pages2.contains(&page1));
    }

    #[tokio::test]
    async fn test_remove_tag() {
        let repo = InMemoryTagRepository::new();
        let page_id = Uuid::new_v4();

        repo.add_tag(page_id, "rust").await.unwrap();
        repo.remove_tag(page_id, "rust").await.unwrap();

        let tags = repo.get_by_page(page_id).await.unwrap();
        assert!(tags.is_empty());

        let pages = repo.get_pages_with_tag("rust").await.unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn test_get_all_tags() {
        let repo = InMemoryTagRepository::new();
        let page_id = Uuid::new_v4();

        repo.add_tag(page_id, "rust").await.unwrap();
        repo.add_tag(page_id, "programming").await.unwrap();

        let all_tags = repo.get_all_tags().await.unwrap();
        assert_eq!(all_tags.len(), 2);
        assert!(all_tags.contains(&"rust".to_string()));
        assert!(all_tags.contains(&"programming".to_string()));
    }

    #[tokio::test]
    async fn test_get_tag_counts() {
        let repo = InMemoryTagRepository::new();
        let page1 = Uuid::new_v4();
        let page2 = Uuid::new_v4();

        repo.add_tag(page1, "rust").await.unwrap();
        repo.add_tag(page2, "rust").await.unwrap();
        repo.add_tag(page1, "python").await.unwrap();

        let counts = repo.get_tag_counts().await.unwrap();
        let rust_count = counts.iter().find(|(name, _)| name == "rust").unwrap();
        assert_eq!(rust_count.1, 2);
    }

    #[tokio::test]
    async fn test_search_tags() {
        let repo = InMemoryTagRepository::new();
        let page_id = Uuid::new_v4();

        repo.add_tag(page_id, "rust").await.unwrap();
        repo.add_tag(page_id, "rust-async").await.unwrap();
        repo.add_tag(page_id, "python").await.unwrap();
        repo.add_tag(page_id, "python asyncio").await.unwrap();

        let results = repo.search_tags("rust", 10).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"rust".to_string()));
        assert!(results.contains(&"rust-async".to_string()));
    }

    #[tokio::test]
    async fn test_search_tags_with_limit() {
        let repo = InMemoryTagRepository::new();
        let page_id = Uuid::new_v4();

        repo.add_tag(page_id, "rust").await.unwrap();
        repo.add_tag(page_id, "rust-async").await.unwrap();
        repo.add_tag(page_id, "python").await.unwrap();

        let results = repo.search_tags("r", 1).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
