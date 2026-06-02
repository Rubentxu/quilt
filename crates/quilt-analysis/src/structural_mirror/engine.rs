use crate::structural_mirror::graph::build_structure_map;
use crate::structural_mirror::types::StructureMap;
use crate::AnalysisError;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

pub type StructuralError = AnalysisError;

#[derive(Clone)]
pub struct StructuralMirror {
    block_repo: Arc<dyn BlockRepository>,
}

impl std::fmt::Debug for StructuralMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructuralMirror")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .finish()
    }
}

impl StructuralMirror {
    pub fn new(block_repo: Arc<dyn BlockRepository>) -> Self {
        Self { block_repo }
    }

    #[instrument(skip(self))]
    pub async fn analyze(&self, page_id: Uuid) -> Result<StructureMap, StructuralError> {
        let blocks = self.block_repo.get_by_page(page_id).await?;
        Ok(build_structure_map(&blocks))
    }

    #[instrument(skip(self, blocks))]
    pub async fn analyze_blocks(&self, blocks: &[quilt_domain::entities::Block]) -> StructureMap {
        build_structure_map(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structural_mirror::LightweightGraph;
    use async_trait::async_trait;
    use quilt_domain::entities::Block;
    use quilt_domain::errors::DomainError;
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, Uuid};
    use std::collections::HashMap;

    fn make_block(id: Uuid, refs: Vec<Uuid>, page_id: Uuid) -> Block {
        Block {
            id,
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: format!("Block {}", id),
            properties: HashMap::new(),
            refs,
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn uuid_from_u8(i: u8) -> Uuid {
        let mut b = [0u8; 16];
        b[0] = i;
        Uuid::from_bytes(b)
    }

    #[derive(Debug, Clone, Default)]
    struct MockBlockRepo {
        pages: HashMap<Uuid, Vec<Block>>,
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<Block>, DomainError> {
            Ok(None)
        }
        async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(self.pages.get(&page_id).cloned().unwrap_or_default())
        }
        async fn get_children(&self, _parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(&self, _id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            Err(DomainError::NotImplemented(
                "get_with_refs not implemented in mock",
            ))
        }
        async fn insert(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn count_all(&self) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn query_dsl(
            &self,
            _sql: &str,
            _params: &[String],
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage(
                "query_dsl not supported in mock".to_string(),
            ))
        }
        async fn list_by_property(
            &self,
            _key: &str,
            _value: &str,
            _limit: usize,
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage(
                "list_by_property not supported in mock".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn test_analyze_empty_page() {
        let page_id = uuid_from_u8(1);
        let repo = Arc::new(MockBlockRepo::default());
        let mirror = StructuralMirror::new(repo);
        let map = mirror.analyze(page_id).await.unwrap();
        assert!(map.clusters.is_empty());
        assert!(map.density.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_single_block() {
        let page_id = uuid_from_u8(1);
        let block = make_block(uuid_from_u8(10), vec![], page_id);
        let repo = Arc::new(MockBlockRepo {
            pages: vec![(page_id, vec![block])].into_iter().collect(),
        });
        let mirror = StructuralMirror::new(repo);
        let map = mirror.analyze(page_id).await.unwrap();
        assert_eq!(map.clusters.len(), 1);
        assert_eq!(map.clusters[0].block_ids.len(), 1);
        assert_eq!(map.frontiers.len(), 0);
    }

    #[tokio::test]
    async fn test_analyze_cyclic_refs() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);
        let blocks = vec![
            make_block(a, vec![b], page_id),
            make_block(b, vec![c], page_id),
            make_block(c, vec![a], page_id),
        ];
        let repo = Arc::new(MockBlockRepo {
            pages: vec![(page_id, blocks)].into_iter().collect(),
        });
        let mirror = StructuralMirror::new(repo);
        let map = mirror.analyze(page_id).await.unwrap();
        assert_eq!(map.clusters.len(), 1);
        assert_eq!(map.clusters[0].block_ids.len(), 3);
        assert!(map.frontiers.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_star_graph() {
        let page_id = uuid_from_u8(1);
        let center = uuid_from_u8(10);
        let leaves: Vec<Uuid> = (11..16).map(uuid_from_u8).collect();
        let mut blocks = vec![make_block(center, leaves.clone(), page_id)];
        for &leaf in &leaves {
            blocks.push(make_block(leaf, vec![], page_id));
        }
        let blocks_clone = blocks.clone();
        let repo = Arc::new(MockBlockRepo {
            pages: vec![(page_id, blocks)].into_iter().collect(),
        });
        let mirror = StructuralMirror::new(repo);
        let map = mirror.analyze(page_id).await.unwrap();
        assert_eq!(map.clusters.len(), 1);
        let graph = LightweightGraph::from_blocks(&blocks_clone);
        assert_eq!(graph.out_degree(&center), 5);
    }

    #[tokio::test]
    async fn test_analyze_blocks_direct() {
        let page_id = uuid_from_u8(1);
        let blocks = vec![
            make_block(uuid_from_u8(10), vec![], page_id),
            make_block(uuid_from_u8(11), vec![], page_id),
        ];
        let repo = Arc::new(MockBlockRepo::default());
        let mirror = StructuralMirror::new(repo);
        let map = mirror.analyze_blocks(&blocks).await;
        assert_eq!(map.clusters.len(), 2);
    }
}
