//! In-memory PageRepository implementation for testing.

use async_trait::async_trait;
use parking_lot::RwLock;
use quilt_domain::entities::Page;
use quilt_domain::errors::DomainError;
use quilt_domain::properties::entry::{DefaultPropertyEntry, HasValue};
use quilt_domain::repositories::{PageRepository, PropertyRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, PropertyValue, Uuid};
use std::collections::HashMap;
use std::sync::Arc;

/// In-memory PageRepository using HashMap storage.
#[deprecated(
    since = "0.1.0",
    note = "Use `quilt_test_helpers::InMemoryPageRepo` instead"
)]
pub struct InMemoryPageRepository {
    pages: RwLock<HashMap<Uuid, Page>>,
    /// Optional property repository for read-only checks in `update_properties`.
    /// When None, falls back to a hardcoded list of system property keys.
    property_repo: Option<Arc<dyn PropertyRepository>>,
}

impl std::fmt::Debug for InMemoryPageRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InMemoryPageRepository")
            .field("pages_count", &self.pages.read().len())
            .field("has_property_repo", &self.property_repo.is_some())
            .finish()
    }
}

impl Default for InMemoryPageRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryPageRepository {
    /// Create a new empty in-memory page repository.
    pub fn new() -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
            property_repo: None,
        }
    }

    /// Create a new in-memory page repository with a property repository for
    /// read-only checks. Used by integration tests (T-B.14, T-B.15).
    pub fn with_property_repo(repo: Arc<dyn PropertyRepository>) -> Self {
        Self {
            pages: RwLock::new(HashMap::new()),
            property_repo: Some(repo),
        }
    }

    /// Check whether a key resolves to a read-only PropertyDefinition. Uses
    /// the property repository if available, otherwise falls back to the
    /// hardcoded system property list.
    async fn is_read_only_key(&self, key: &str) -> bool {
        if let Some(repo) = &self.property_repo {
            if let Ok(Some(def)) = repo.get_by_db_ident(key).await {
                return def.read_only;
            }
            // Check builtin fallback.
            if let Some(def) = quilt_domain::properties::builtin::get_builtin_property(key) {
                return def.read_only;
            }
            // Unknown key — not read-only (allow).
            return false;
        }
        // No repo: fall back to hardcoded system keys.
        matches!(key, "id" | "created_at" | "updated_at")
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

    async fn update_properties(
        &self,
        page_id: Uuid,
        props: HashMap<String, DefaultPropertyEntry<PropertyValue>>,
    ) -> Result<Page, DomainError> {
        // 1. Read-only check: reject any key that resolves to read-only.
        //    This is atomic — first read-only key fails the whole call.
        for key in props.keys() {
            if self.is_read_only_key(key).await {
                return Err(DomainError::PropertyReadOnly(key.clone()));
            }
        }

        // 2. Load page, merge, persist.
        let mut pages = self.pages.write();
        let page = pages
            .get_mut(&page_id)
            .ok_or(DomainError::PageNotFound(page_id))?;
        let merged = quilt_domain::properties::merge_properties(&page.properties, props);
        page.properties = merged;
        page.updated_at = chrono::Utc::now();
        Ok(page.clone())
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
            properties: std::collections::HashMap::new(),
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

        let journal = Page::new_journal(day, BlockFormat::Markdown, "%Y-%m-%d").unwrap();
        repo.insert(&journal).await.unwrap();

        let retrieved = repo.get_journal(day).await.unwrap();
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().journal);
    }

    // ── F5 + F8 + F9: update_properties parity tests ──

    fn entry_str_ts(
        s: &str,
        ts: chrono::DateTime<chrono::Utc>,
    ) -> DefaultPropertyEntry<PropertyValue> {
        DefaultPropertyEntry::with_timestamp(PropertyValue::string(s), ts)
    }

    #[tokio::test]
    async fn test_inmem_update_properties_rejects_read_only() {
        // Hardcoded fallback list (no property_repo) covers system keys.
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("ro-page");
        let id = page.id;
        repo.insert(&page).await.unwrap();

        let mut props = HashMap::new();
        props.insert(
            "created_at".to_string(),
            DefaultPropertyEntry::new(PropertyValue::string("2026-01-01")),
        );
        let result = repo.update_properties(id, props).await;
        assert!(matches!(result, Err(DomainError::PropertyReadOnly(k)) if k == "created_at"));
    }

    #[tokio::test]
    async fn test_inmem_update_properties_merge_preserves_keys() {
        // Distinct keys both survive. Same key with newer timestamp wins.
        let repo = InMemoryPageRepository::new();
        let page = create_test_page("merge-page");
        let id = page.id;
        repo.insert(&page).await.unwrap();

        let t0 = chrono::Utc::now();
        let mut seed = HashMap::new();
        seed.insert("a".to_string(), entry_str_ts("A0", t0));
        seed.insert("b".to_string(), entry_str_ts("B0", t0));
        repo.update_properties(id, seed).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        let t1 = chrono::Utc::now();
        let mut upd = HashMap::new();
        upd.insert("a".to_string(), entry_str_ts("A1", t1));
        upd.insert("c".to_string(), entry_str_ts("C1", t1));
        let updated = repo.update_properties(id, upd).await.unwrap();

        // a updated, b preserved, c added.
        assert_eq!(
            updated.properties["a"].value(),
            &PropertyValue::String("A1".to_string())
        );
        assert_eq!(
            updated.properties["b"].value(),
            &PropertyValue::String("B0".to_string())
        );
        assert_eq!(
            updated.properties["c"].value(),
            &PropertyValue::String("C1".to_string())
        );
    }

    #[tokio::test]
    async fn test_inmem_update_properties_not_found() {
        // Updating a non-existent page returns PageNotFound.
        let repo = InMemoryPageRepository::new();
        let bogus_id = Uuid::new_v4();
        let mut props = HashMap::new();
        props.insert("x".to_string(), entry_str_ts("v", chrono::Utc::now()));
        let result = repo.update_properties(bogus_id, props).await;
        assert!(matches!(result, Err(DomainError::PageNotFound(_))));
    }
}
