//! In-memory PageRepository wrapper with Arc-wrapped builder API.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use quilt_domain::entities::Page;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::{JournalDay, Uuid};

/// In-memory PageRepository using HashMap storage, wrapped for test usability.
#[derive(Debug)]
pub struct InMemoryPageRepo {
    repo: RwLock<HashMap<Uuid, Page>>,
}

impl Default for InMemoryPageRepo {
    fn default() -> Self {
        Self {
            repo: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemoryPageRepo {
    /// Create a new empty in-memory page repository.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            repo: RwLock::new(HashMap::new()),
        })
    }

    /// Add pre-existing pages to the repository.
    ///
    /// Consumes `self` and returns an `Arc<Self>` for chaining.
    pub fn with_pages(self: Arc<Self>, pages: Vec<Page>) -> Arc<Self> {
        {
            let mut repo = self.repo.write();
            for page in pages {
                repo.insert(page.id, page);
            }
        }
        self
    }

    /// Get a trait object reference for use in traits that require `dyn PageRepository`.
    pub fn as_trait(self: Arc<Self>) -> Arc<dyn PageRepository> {
        self
    }
}

#[async_trait]
impl PageRepository for InMemoryPageRepo {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
        let repo = self.repo.read();
        Ok(repo.get(&id).cloned())
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
        let name_lower = name.to_lowercase();
        let repo = self.repo.read();
        Ok(repo
            .values()
            .find(|p| p.name.to_lowercase() == name_lower)
            .cloned())
    }

    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError> {
        let repo = self.repo.read();
        Ok(repo.values().find(|p| p.journal_day == Some(day)).cloned())
    }

    async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
        let repo = self.repo.read();
        Ok(repo.values().cloned().collect())
    }

    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|p| p.namespace_id == Some(namespace_id))
            .cloned()
            .collect())
    }

    async fn insert(&self, page: &Page) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        repo.insert(page.id, page.clone());
        Ok(())
    }

    async fn update(&self, page: &Page) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if repo.contains_key(&page.id) {
            repo.insert(page.id, page.clone());
            Ok(())
        } else {
            Err(DomainError::PageNotFound(page.id))
        }
    }

    async fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if let Some(page) = repo.get_mut(&id) {
            page.rename(new_name)?;
            Ok(())
        } else {
            Err(DomainError::PageNotFound(id))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        repo.remove(&id);
        Ok(())
    }

    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Page>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|p| p.updated_at >= since)
            .cloned()
            .collect())
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError> {
        let repo = self.repo.read();
        let mut all: Vec<Page> = repo.values().cloned().collect();
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        all.truncate(limit);
        Ok(all)
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let repo = self.repo.read();
        Ok(repo.len())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError> {
        let query_lower = query.to_lowercase();
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|p| p.name.to_lowercase().contains(&query_lower))
            .take(limit)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::PageCreate;
    use quilt_domain::value_objects::BlockFormat;

    fn make_page(name: &str) -> Page {
        Page::new(PageCreate {
            name: name.to_string(),
            title: Some(name.to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn test_new() {
        let repo = InMemoryPageRepo::new();
        assert_eq!(repo.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_with_pages() {
        let pages = vec![make_page("Page 1"), make_page("Page 2")];
        let repo = InMemoryPageRepo::new().with_pages(pages);
        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_as_trait() {
        let repo = InMemoryPageRepo::new();
        let _trait_repo: Arc<dyn PageRepository> = repo.as_trait();
    }

    #[tokio::test]
    async fn test_chaining() {
        let repo = InMemoryPageRepo::new()
            .with_pages(vec![make_page("Page 1")])
            .with_pages(vec![make_page("Page 2")]);
        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let pages = vec![make_page("Test Page")];
        let repo = InMemoryPageRepo::new().with_pages(pages);
        let found = repo.get_by_name("test page").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test page");
    }
}
