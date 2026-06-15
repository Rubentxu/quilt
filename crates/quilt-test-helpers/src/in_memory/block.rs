//! In-memory BlockRepository wrapper with Arc-wrapped builder API.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use quilt_domain::entities::{Block, Page};
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;

/// In-memory BlockRepository using HashMap storage, wrapped for test usability.
///
/// Provides a builder API that returns `Arc<Self>` so it can be cloned and
/// passed around without needing to wrap in `Arc` manually.
#[derive(Debug)]
pub struct InMemoryBlockRepo {
    /// The inner repository state
    repo: RwLock<HashMap<Uuid, Block>>,
}

impl Default for InMemoryBlockRepo {
    fn default() -> Self {
        Self {
            repo: RwLock::new(HashMap::new()),
        }
    }
}

impl InMemoryBlockRepo {
    /// Create a new empty in-memory block repository.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            repo: RwLock::new(HashMap::new()),
        })
    }

    /// Add pre-existing blocks to the repository.
    ///
    /// Consumes `self` and returns an `Arc<Self>` for chaining.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use quilt_test_helpers::{InMemoryBlockRepo, page_with_blocks};
    ///
    /// let (page, blocks) = page_with_blocks("Test", vec!["A", "B"]);
    /// let repo = InMemoryBlockRepo::new()
    ///     .with_blocks(blocks);
    /// ```
    pub fn with_blocks(self: Arc<Self>, blocks: Vec<Block>) -> Arc<Self> {
        {
            let mut repo = self.repo.write();
            for block in blocks {
                repo.insert(block.id, block);
            }
        }
        self
    }

    /// Add a page and its top-level blocks to the repository.
    ///
    /// The blocks are created via `Block::new()` with `page_id` set correctly.
    /// Consumes `self` and returns `Result<Arc<Self>, DomainError>` for chaining.
    ///
    /// # Validation
    ///
    /// Each block's `page_id` must match the provided page's `id`.
    /// Returns `Err(DomainError::InvalidData)` if any block has a mismatched `page_id`.
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_test_helpers::{InMemoryBlockRepo, page_with_blocks};
    ///
    /// let (page, blocks) = page_with_blocks("Test", vec!["A", "B"]).unwrap();
    /// let repo = InMemoryBlockRepo::new()
    ///     .with_page(page, blocks)
    ///     .expect("blocks must belong to the page");
    /// ```
    pub fn with_page(
        self: Arc<Self>,
        page: Page,
        blocks: Vec<Block>,
    ) -> Result<Arc<Self>, DomainError> {
        // Validate that all blocks belong to the provided page
        for block in &blocks {
            if block.page_id != page.id {
                return Err(DomainError::InvalidData(format!(
                    "Block {} has page_id {} but expected {}",
                    block.id, block.page_id, page.id
                )));
            }
        }
        {
            let mut repo = self.repo.write();
            for block in blocks {
                repo.insert(block.id, block);
            }
        }
        Ok(self)
    }

    /// Get a trait object reference for use in traits that require `dyn BlockRepository`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use quilt_domain::repositories::BlockRepository;
    /// use quilt_test_helpers::InMemoryBlockRepo;
    ///
    /// let repo = InMemoryBlockRepo::new();
    /// let trait_repo: Arc<dyn BlockRepository> = repo.as_trait();
    /// ```
    pub fn as_trait(self: Arc<Self>) -> Arc<dyn BlockRepository> {
        self
    }
}

#[async_trait]
impl BlockRepository for InMemoryBlockRepo {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
        let repo = self.repo.read();
        Ok(repo.get(&id).cloned())
    }

    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|b| b.page_id == page_id)
            .cloned()
            .collect())
    }

    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|b| b.parent_id == Some(parent_id))
            .cloned()
            .collect())
    }

    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
        let repo = self.repo.read();
        repo.get(&id)
            .map(|b| (b.clone(), b.refs.clone()))
            .ok_or_else(|| DomainError::BlockNotFound(id))
            .map(|(block, refs)| (block, refs))
    }

    async fn insert(&self, block: &Block) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        repo.insert(block.id, block.clone());
        Ok(())
    }

    async fn update(&self, block: &Block) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if repo.contains_key(&block.id) {
            repo.insert(block.id, block.clone());
            Ok(())
        } else {
            Err(DomainError::BlockNotFound(block.id))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        repo.remove(&id);
        Ok(())
    }

    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError> {
        let mut repo = self.repo.write();
        if let Some(block) = repo.get_mut(&id) {
            block.parent_id = new_parent;
            block.order = new_order;
            block.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(DomainError::BlockNotFound(id))
        }
    }

    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|b| b.refs.contains(&block_id))
            .cloned()
            .collect())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError> {
        let query_lower = query.to_lowercase();
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|b| b.content.to_lowercase().contains(&query_lower))
            .take(limit)
            .cloned()
            .collect())
    }

    async fn get_updated_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Block>, DomainError> {
        let repo = self.repo.read();
        Ok(repo
            .values()
            .filter(|b| b.updated_at >= since)
            .cloned()
            .collect())
    }

    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError> {
        let repo = self.repo.read();
        Ok(repo.values().filter(|b| b.page_id == page_id).count())
    }

    async fn count_all(&self) -> Result<usize, DomainError> {
        let repo = self.repo.read();
        Ok(repo.len())
    }

    async fn query_dsl(&self, _sql: &str, _params: &[String]) -> Result<Vec<Block>, DomainError> {
        Err(DomainError::Storage(
            "query_dsl not supported by in-memory repository".to_string(),
        ))
    }

    async fn list_by_property(
        &self,
        key: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<Block>, DomainError> {
        let repo = self.repo.read();
        let mut out: Vec<Block> = repo
            .values()
            .filter(|b| match b.properties.get(key) {
                Some(quilt_domain::value_objects::PropertyValue::String(s)) => s == value,
                _ => false,
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if limit > 0 {
            out.truncate(limit);
        }
        Ok(out)
    }

    async fn list_distinct_keys(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, DomainError> {
        // Scan every block and collect distinct top-level keys.
        // We then sort lexicographically and apply the cursor+limit
        // slice. This mirrors the SQLite `json_each` + ORDER BY
        // behavior at the contract level.
        let repo = self.repo.read();
        let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for block in repo.values() {
            for k in block.properties.keys() {
                keys.insert(k.clone());
            }
        }

        let mut out: Vec<String> = keys.into_iter().collect();
        if let Some(c) = cursor {
            // Strictly greater than cursor (per the trait contract).
            out.retain(|k| k.as_str() > c);
        }
        // Truncate to `limit` (caller is responsible for bounds — the
        // trait says implementations trust the input).
        out.truncate(limit as usize);
        Ok(out)
    }

    async fn list_by_property_key(&self, key: &str, limit: u32) -> Result<Vec<Block>, DomainError> {
        // Scan every block; keep the ones that have the key in their
        // `properties` map (value is irrelevant). Mirrors the SQLite
        // `json_extract IS NOT NULL` check.
        let repo = self.repo.read();
        let mut out: Vec<Block> = repo
            .values()
            .filter(|b| b.properties.contains_key(key))
            .cloned()
            .collect();
        out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        if limit > 0 {
            out.truncate(limit as usize);
        }
        Ok(out)
    }

    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, DomainError> {
        // Mirror the SQLite semantics: distinct string values of the
        // `created_by` property, sorted ASC, optionally filtered by
        // a `LIKE` prefix. NULLs and non-string values are skipped.
        let repo = self.repo.read();
        let mut authors: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for block in repo.values() {
            if let Some(value) = block.properties.get("created_by") {
                if let quilt_domain::value_objects::PropertyValue::String(s) = value {
                    let matches_prefix = match prefix {
                        Some(p) => s.starts_with(p),
                        None => true,
                    };
                    if matches_prefix && !s.is_empty() {
                        authors.insert(s.clone());
                    }
                }
            }
        }
        Ok(authors.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{BlockCreate, PageCreate};
    use quilt_domain::value_objects::{BlockFormat, BlockType};

    fn make_block(page_id: Uuid, content: &str) -> Block {
        Block::new(BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: std::collections::HashMap::new(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn test_new() {
        let repo = InMemoryBlockRepo::new();
        assert_eq!(repo.count_all().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_with_blocks() {
        let page_id = Uuid::new_v4();
        let blocks = vec![
            make_block(page_id, "Block 1"),
            make_block(page_id, "Block 2"),
        ];

        let repo = InMemoryBlockRepo::new().with_blocks(blocks);

        assert_eq!(repo.count_all().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_with_page() {
        let page = Page::new(PageCreate {
            name: "Test Page".to_string(),
            title: Some("Test Page".to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap();

        let blocks = vec![
            make_block(page.id, "Block 1"),
            make_block(page.id, "Block 2"),
        ];

        let repo = InMemoryBlockRepo::new()
            .with_page(page.clone(), blocks)
            .expect("blocks should belong to the page");

        assert_eq!(repo.count_by_page(page.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_as_trait() {
        let repo = InMemoryBlockRepo::new();
        let _trait_repo: Arc<dyn BlockRepository> = repo.as_trait();
        // Just verify it compiles and returns the right type
    }

    #[tokio::test]
    async fn test_chaining() {
        let page = Page::new(PageCreate {
            name: "Test Page".to_string(),
            title: Some("Test Page".to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap();

        let blocks = vec![
            make_block(page.id, "Block 1"),
            make_block(page.id, "Block 2"),
        ];

        let repo = InMemoryBlockRepo::new()
            .with_blocks(vec![])
            .with_page(page.clone(), blocks)
            .expect("blocks should belong to the page");

        assert_eq!(repo.count_by_page(page.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_with_page_invalid_block_page_id() {
        let page = Page::new(PageCreate {
            name: "Test Page".to_string(),
            title: Some("Test Page".to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        })
        .unwrap();

        // Create a block with a different page_id
        let other_page_id = Uuid::new_v4();
        let blocks = vec![make_block(other_page_id, "Block 1")];

        let result = InMemoryBlockRepo::new().with_page(page.clone(), blocks);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidData(_)));
    }

    // ── list_distinct_keys tests (T1 of property-keys-endpoint) ─────

    /// Build a block with the given properties map.
    fn make_block_with_properties(
        page_id: Uuid,
        content: &str,
        properties: std::collections::HashMap<String, quilt_domain::value_objects::PropertyValue>,
    ) -> Block {
        Block::new(BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties,
        })
        .unwrap()
    }

    fn props(
        pairs: &[(&str, &str)],
    ) -> std::collections::HashMap<String, quilt_domain::value_objects::PropertyValue> {
        pairs
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    quilt_domain::value_objects::PropertyValue::string(*v),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn test_list_distinct_keys_empty_db() {
        let repo = InMemoryBlockRepo::new();
        let keys = repo.list_distinct_keys(None, 50).await.unwrap();
        assert!(keys.is_empty(), "empty DB should yield no keys");
    }

    #[tokio::test]
    async fn test_list_distinct_keys_blocks_with_empty_properties() {
        let page_id = Uuid::new_v4();
        let blocks = vec![
            make_block(page_id, "Block 1"),
            make_block(page_id, "Block 2"),
        ];
        let repo = InMemoryBlockRepo::new().with_blocks(blocks);

        let keys = repo.list_distinct_keys(None, 50).await.unwrap();
        assert!(
            keys.is_empty(),
            "blocks with empty properties should yield no keys"
        );
    }

    #[tokio::test]
    async fn test_list_distinct_keys_returns_distinct_keys_sorted() {
        // GIVEN multiple blocks whose properties contain overlapping
        // and unique keys, in non-alphabetical insertion order.
        let page_id = Uuid::new_v4();
        let blocks = vec![
            make_block_with_properties(
                page_id,
                "B1",
                props(&[("status", "Doing"), ("priority", "A")]),
            ),
            make_block_with_properties(
                page_id,
                "B2",
                props(&[("status", "Done"), ("deadline", "2026-01-01")]),
            ),
            make_block_with_properties(page_id, "B3", props(&[("alpha", "x")])),
        ];
        let repo = InMemoryBlockRepo::new().with_blocks(blocks);

        let keys = repo.list_distinct_keys(None, 50).await.unwrap();

        // All 4 distinct keys, sorted lexicographically ASC.
        assert_eq!(
            keys,
            vec![
                "alpha".to_string(),
                "deadline".to_string(),
                "priority".to_string(),
                "status".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn test_list_distinct_keys_cursor_filters_strictly_greater() {
        // GIVEN 5 distinct keys: a, b, c, d, e
        // WHEN cursor = "b" → returns strictly c, d, e
        let page_id = Uuid::new_v4();
        let mut all_blocks = Vec::new();
        for (i, k) in ["a", "b", "c", "d", "e"].iter().enumerate() {
            all_blocks.push(make_block_with_properties(
                page_id,
                &format!("B{i}"),
                props(&[(k, "v")]),
            ));
        }
        let repo = InMemoryBlockRepo::new().with_blocks(all_blocks);

        let keys = repo.list_distinct_keys(Some("b"), 50).await.unwrap();
        assert_eq!(
            keys,
            vec!["c".to_string(), "d".to_string(), "e".to_string()],
            "cursor must be strictly greater than"
        );

        // Cursor at first key returns the rest.
        let keys = repo.list_distinct_keys(Some("a"), 50).await.unwrap();
        assert_eq!(
            keys,
            vec![
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
                "e".to_string()
            ]
        );

        // Cursor at the last key returns nothing.
        let keys = repo.list_distinct_keys(Some("e"), 50).await.unwrap();
        assert!(keys.is_empty());
    }

    #[tokio::test]
    async fn test_list_distinct_keys_limit_slices() {
        // GIVEN 10 distinct keys: k00, k01, ..., k09
        let page_id = Uuid::new_v4();
        let mut all_blocks = Vec::new();
        for i in 0..10 {
            let key = format!("k{i:02}");
            all_blocks.push(make_block_with_properties(
                page_id,
                &format!("B{i}"),
                props(&[(&key, "v")]),
            ));
        }
        let repo = InMemoryBlockRepo::new().with_blocks(all_blocks);

        // limit=3 → 3 smallest keys
        let keys = repo.list_distinct_keys(None, 3).await.unwrap();
        assert_eq!(
            keys,
            vec!["k00".to_string(), "k01".to_string(), "k02".to_string()]
        );

        // limit=10 → all 10
        let keys = repo.list_distinct_keys(None, 10).await.unwrap();
        assert_eq!(keys.len(), 10);

        // limit=100 → still only 10 (no error on limit > total)
        let keys = repo.list_distinct_keys(None, 100).await.unwrap();
        assert_eq!(keys.len(), 10);
    }

    #[tokio::test]
    async fn test_list_distinct_keys_utf8_cursor() {
        // A real-world key with a `/` in it (URL-encoded on the wire).
        // Confirms we don't accidentally split on `/` or other separators.
        let page_id = Uuid::new_v4();
        let blocks = vec![
            make_block_with_properties(
                page_id,
                "B1",
                props(&[("priority/level", "high"), ("status", "Doing")]),
            ),
            make_block_with_properties(page_id, "B2", props(&[("priority/level", "low")])),
        ];
        let repo = InMemoryBlockRepo::new().with_blocks(blocks);

        // Without cursor → both keys, sorted: "priority/level" < "status"
        let keys = repo.list_distinct_keys(None, 50).await.unwrap();
        assert_eq!(
            keys,
            vec!["priority/level".to_string(), "status".to_string()]
        );

        // Cursor at "priority/level" → only "status" remains
        let keys = repo
            .list_distinct_keys(Some("priority/level"), 50)
            .await
            .unwrap();
        assert_eq!(keys, vec!["status".to_string()]);
    }
}
