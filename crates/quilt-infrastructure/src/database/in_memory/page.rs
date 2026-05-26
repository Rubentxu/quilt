//! In-memory PageRepository implementation for testing.

use async_trait::async_trait;
use parking_lot::RwLock;
use quilt_domain::entities::Page;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use std::collections::HashMap;

/// In-memory PageRepository using HashMap storage.
#[deprecated(
    since = "0.1.0",
    note = "Use `quilt_test_helpers::InMemoryPageRepo` instead"
)]
#[derive(Debug, Default)]
pub struct InMemoryPageRepository {
    pages: RwLock<HashMap<Uuid, Page>>,
}

impl InMemoryPageRepository {
    /// Create a new empty in-memory page repository.
    pub fn new() -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl PageRepository for InMemoryPageRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
        let pages = self.pages.read();
        Ok(pages.get(&id).cloned())
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
        let name_lower = name.to_lowercase();
        let pages = self.pages.read();
        Ok(pages
            .values()
            .find(|p| p.name.to_lowercase() == name_lower)
            .cloned())
    }

    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError> {
        let pages = self.pages.read();
        Ok(pages.values().find(|p| p.journal_day == Some(day)).cloned())
    }

    async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
        let pages = self.pages.read();
        Ok(pages.values().cloned().collect())
    }

    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
        let pages = self.pages.read();
        Ok(pages
            .values()
            .filter(|p| p.namespace_id == Some(namespace_id))
            .cloned()
            .collect())
    }

    async fn insert(&self, page: &Page) -> Result<(), DomainError> {
        let mut pages = self.pages.write();
        pages.insert(page.id, page.clone());
        Ok(())
    }

    async fn update(&self, page: &Page) -> Result<(), DomainError> {
        let mut pages = self.pages.write();
        if pages.contains_key(&page.id) {
            pages.insert(page.id, page.clone());
            Ok(())
        } else {
            Err(DomainError::PageNotFound(page.id))
        }
    }

    async fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError> {
        let mut pages = self.pages.write();
        if let Some(page) = pages.get_mut(&id) {
            page.rename(new_name)?;
            Ok(())
        } else {
            Err(DomainError::PageNotFound(id))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut pages = self.pages.write();
        pages.remove(&id);
        Ok(())
    }

    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Page>, DomainError> {
        let pages = self.pages.read();
        Ok(pages
            .values()
            .filter(|p| p.updated_at >= since)
            .cloned()
            .collect())
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError> {
        let pages = self.pages.read();
        let mut all: Vec<Page> = pages.values().cloned().collect();
        all.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        all.truncate(limit);
        Ok(all)
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let pages = self.pages.read();
        Ok(pages.len())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError> {
        let query_lower = query.to_lowercase();
        let pages = self.pages.read();
        Ok(pages
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

    fn create_test_page(name: &str) -> Page {
        Page::new(PageCreate {
            name: name.to_string(),
            title: Some(name.to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("My Page");
        let page_id = page.id;

        repo.insert(&page).await.unwrap();
        let retrieved = repo.get_by_id(page_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "my page");
    }

    #[tokio::test]
    async fn test_get_by_name() {
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("My Test Page");

        repo.insert(&page).await.unwrap();

        let retrieved = repo.get_by_name("my test page").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "my test page");

        // Case insensitive
        let retrieved2 = repo.get_by_name("MY TEST PAGE").await.unwrap();
        assert!(retrieved2.is_some());
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("To Delete");
        let page_id = page.id;

        repo.insert(&page).await.unwrap();
        repo.delete(page_id).await.unwrap();

        let retrieved = repo.get_by_id(page_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_all() {
        let repo = InMemoryPageRepository::new();

        let page1 = create_test_page("Page One");
        let page2 = create_test_page("Page Two");

        repo.insert(&page1).await.unwrap();
        repo.insert(&page2).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_search() {
        let repo = InMemoryPageRepository::new();

        let page1 = create_test_page("Rust Programming");
        let page2 = create_test_page("Python Basics");
        let page3 = create_test_page("Rust Web Development");

        repo.insert(&page1).await.unwrap();
        repo.insert(&page2).await.unwrap();
        repo.insert(&page3).await.unwrap();

        let results = repo.search("rust", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_rename() {
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("Old Name");
        let page_id = page.id;

        repo.insert(&page).await.unwrap();
        repo.rename(page_id, "New Name").await.unwrap();

        let retrieved = repo.get_by_id(page_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "new name");
    }

    #[tokio::test]
    async fn test_get_recent() {
        let repo = InMemoryPageRepository::new();

        let page1 = create_test_page("First");
        let page2 = create_test_page("Second");
        let page3 = create_test_page("Third");

        // Insert in order but they have different updated_at
        repo.insert(&page1).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        repo.insert(&page2).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        repo.insert(&page3).await.unwrap();

        let recent = repo.get_recent(2).await.unwrap();
        assert_eq!(recent.len(), 2);
        // Most recently updated should be first
        assert_eq!(recent[0].name, "third");
    }

    #[tokio::test]
    async fn test_count() {
        let repo = InMemoryPageRepository::new();

        let page1 = create_test_page("Page One");
        let page2 = create_test_page("Page Two");

        assert_eq!(repo.count().await.unwrap(), 0);

        repo.insert(&page1).await.unwrap();
        repo.insert(&page2).await.unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_journal_pages() {
        let repo = InMemoryPageRepository::new();
        let day = JournalDay::from_ymd(2026, 5, 25).unwrap();

        let journal = Page::new_journal(day, BlockFormat::Markdown).unwrap();
        repo.insert(&journal).await.unwrap();

        let retrieved = repo.get_journal(day).await.unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().journal);
    }
}
