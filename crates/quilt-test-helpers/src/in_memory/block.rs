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
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{BlockCreate, PageCreate};
    use quilt_domain::value_objects::BlockFormat;

    fn make_block(page_id: Uuid, content: &str) -> Block {
        Block::new(BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
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
            name: "Chained".to_string(),
            title: Some("Chained".to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
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
        })
        .unwrap();

        // Create a block with a different page_id
        let other_page_id = Uuid::new_v4();
        let blocks = vec![make_block(other_page_id, "Block 1")];

        let result = InMemoryBlockRepo::new().with_page(page.clone(), blocks);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DomainError::InvalidData(_)));
    }
}
