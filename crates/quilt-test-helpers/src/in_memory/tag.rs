//! In-memory TagRepository wrapper with Arc-wrapped builder API.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use quilt_domain::errors::DomainError;
use quilt_domain::repositories::TagRepository;
use quilt_domain::value_objects::Uuid;

/// In-memory TagRepository using HashMap storage, wrapped for test usability.
#[derive(Debug)]
pub struct InMemoryTagRepo {
    /// Maps tag name -> set of page IDs with that tag
    tags: RwLock<HashMap<String, std::collections::HashSet<Uuid>>>,
}

impl Default for InMemoryTagRepo {
    fn default() -> Self {
        Self {
            tags: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemoryTagRepo {
    /// Create a new empty in-memory tag repository.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tags: RwLock::new(HashMap::new()),
        })
    }

    /// Add pre-existing tags to the repository.
    ///
    /// Each tuple is `(tag_name, page_id)`.
    ///
    /// Consumes `self` and returns an `Arc<Self>` for chaining.
    pub fn with_tags(self: Arc<Self>, tags: Vec<(String, Uuid)>) -> Arc<Self> {
        {
            let mut repo = self.tags.write();
            for (tag_name, page_id) in tags {
                repo.entry(tag_name)
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(page_id);
            }
        }
        self
    }

    /// Get a trait object reference for use in traits that require `dyn TagRepository`.
    pub fn as_trait(self: Arc<Self>) -> Arc<dyn TagRepository> {
        self
    }
}

#[async_trait]
impl TagRepository for InMemoryTagRepo {
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
    async fn test_new() {
        let repo = InMemoryTagRepo::new();
        let all = repo.get_all_tags().await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_with_tags() {
        let page_id = Uuid::new_v4();
        let tags = vec![
            ("rust".to_string(), page_id),
            ("programming".to_string(), page_id),
        ];
        let repo = InMemoryTagRepo::new().with_tags(tags);
        let all = repo.get_all_tags().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_as_trait() {
        let repo = InMemoryTagRepo::new();
        let _trait_repo: Arc<dyn TagRepository> = repo.as_trait();
    }

    #[tokio::test]
    async fn test_chaining() {
        let page_id = Uuid::new_v4();
        let repo = InMemoryTagRepo::new()
            .with_tags(vec![("rust".to_string(), page_id)])
            .with_tags(vec![("programming".to_string(), page_id)]);
        assert_eq!(repo.get_all_tags().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_by_page() {
        let page_id = Uuid::new_v4();
        let tags = vec![
            ("rust".to_string(), page_id),
            ("programming".to_string(), page_id),
        ];
        let repo = InMemoryTagRepo::new().with_tags(tags);
        let page_tags = repo.get_by_page(page_id).await.unwrap();
        assert_eq!(page_tags.len(), 2);
    }
}
