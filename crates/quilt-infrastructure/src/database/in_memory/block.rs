//! In-memory BlockRepository implementation for testing.

use async_trait::async_trait;
use parking_lot::RwLock;
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;
use std::collections::HashMap;

/// In-memory BlockRepository using HashMap storage.
#[deprecated(
    since = "0.1.0",
    note = "Use `quilt_test_helpers::InMemoryBlockRepo` instead"
)]
#[derive(Debug, Default)]
pub struct InMemoryBlockRepository {
    blocks: RwLock<HashMap<Uuid, Block>>,
}

impl InMemoryBlockRepository {
    /// Create a new empty in-memory block repository.
    pub fn new() -> Self {
        Self {
            blocks: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl BlockRepository for InMemoryBlockRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks.get(&id).cloned())
    }

    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks
            .values()
            .filter(|b| b.page_id == page_id)
            .cloned()
            .collect())
    }

    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks
            .values()
            .filter(|b| b.parent_id == Some(parent_id))
            .cloned()
            .collect())
    }

    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
        let blocks = self.blocks.read();
        blocks
            .get(&id)
            .map(|b| (b.clone(), b.refs.clone()))
            .ok_or_else(|| DomainError::BlockNotFound(id))
            .map(|(block, refs)| (block, refs))
    }

    async fn insert(&self, block: &Block) -> Result<(), DomainError> {
        let mut blocks = self.blocks.write();
        blocks.insert(block.id, block.clone());
        Ok(())
    }

    async fn update(&self, block: &Block) -> Result<(), DomainError> {
        let mut blocks = self.blocks.write();
        if blocks.contains_key(&block.id) {
            blocks.insert(block.id, block.clone());
            Ok(())
        } else {
            Err(DomainError::BlockNotFound(block.id))
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let mut blocks = self.blocks.write();
        blocks.remove(&id);
        Ok(())
    }

    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError> {
        let mut blocks = self.blocks.write();
        if let Some(block) = blocks.get_mut(&id) {
            block.parent_id = new_parent;
            block.order = new_order;
            block.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(DomainError::BlockNotFound(id))
        }
    }

    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks
            .values()
            .filter(|b| b.refs.contains(&block_id))
            .cloned()
            .collect())
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError> {
        let query_lower = query.to_lowercase();
        let blocks = self.blocks.read();
        Ok(blocks
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
        let blocks = self.blocks.read();
        Ok(blocks
            .values()
            .filter(|b| b.updated_at >= since)
            .cloned()
            .collect())
    }

    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks.values().filter(|b| b.page_id == page_id).count())
    }

    async fn count_all(&self) -> Result<usize, DomainError> {
        let blocks = self.blocks.read();
        Ok(blocks.len())
    }

    async fn query_dsl(&self, _sql: &str, _params: &[String]) -> Result<Vec<Block>, DomainError> {
        // In-memory repository doesn't support raw SQL execution
        Err(DomainError::Storage(
            "query_dsl not supported by in-memory repository".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::BlockCreate;
    use quilt_domain::value_objects::BlockFormat;
    use std::collections::HashMap;

    fn create_test_block(content: &str, page_id: Uuid) -> Block {
        Block::new(BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        })
        .unwrap()
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();
        let block = create_test_block("Hello world", page_id);
        let block_id = block.id;

        repo.insert(&block).await.unwrap();
        let retrieved = repo.get_by_id(block_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Hello world");
    }

    #[tokio::test]
    async fn test_get_by_page() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();

        let block1 = create_test_block("Block 1", page_id);
        let block2 = create_test_block("Block 2", page_id);
        let block3 = create_test_block("Block 3", Uuid::new_v4()); // Different page

        repo.insert(&block1).await.unwrap();
        repo.insert(&block2).await.unwrap();
        repo.insert(&block3).await.unwrap();

        let blocks = repo.get_by_page(page_id).await.unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[tokio::test]
    async fn test_delete() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();
        let block = create_test_block("To delete", page_id);
        let block_id = block.id;

        repo.insert(&block).await.unwrap();
        repo.delete(block_id).await.unwrap();

        let retrieved = repo.get_by_id(block_id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_get_children() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();

        let parent_block = create_test_block("Parent", page_id);
        let parent_id = parent_block.id;

        let child_block = Block::new(BlockCreate {
            page_id,
            content: "Child".to_string(),
            parent_id: Some(parent_id),
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        })
        .unwrap();

        repo.insert(&parent_block).await.unwrap();
        repo.insert(&child_block).await.unwrap();

        let children = repo.get_children(parent_id).await.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].content, "Child");
    }

    #[tokio::test]
    async fn test_search() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();

        let block1 = create_test_block("Hello world", page_id);
        let block2 = create_test_block("Goodbye world", page_id);
        let block3 = create_test_block("Hello again", page_id);

        repo.insert(&block1).await.unwrap();
        repo.insert(&block2).await.unwrap();
        repo.insert(&block3).await.unwrap();

        let results = repo.search("hello", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_backlinks() {
        let repo = InMemoryBlockRepository::new();
        let page_id = Uuid::new_v4();

        let block1 = create_test_block("Block 1", page_id);
        let block1_id = block1.id;

        let mut block2 = create_test_block("Block 2", page_id);
        block2.refs.push(block1_id);

        repo.insert(&block1).await.unwrap();
        repo.insert(&block2).await.unwrap();

        let backlinks = repo.get_backlinks(block1_id).await.unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].content, "Block 2");
    }
}
