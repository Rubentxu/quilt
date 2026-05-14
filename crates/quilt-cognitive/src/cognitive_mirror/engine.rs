//! CognitiveMirror Engine
//!
//! Main entry point for cognitive analysis of a page's knowledge graph.

use crate::ai_client::AIClient;
use crate::cognitive_mirror::graph::build_cognitive_map;
use crate::cognitive_mirror::types::{CognitiveMap, KnowledgeCluster};
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::{JournalDay, Uuid};
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

/// Errors that can occur during cognitive analysis
#[derive(Debug, Error)]
pub enum CognitiveError {
    #[error("Block not found: {0}")]
    BlockNotFound(Uuid),
    #[error("AI client error: {0}")]
    AI(#[from] crate::ai_client::AIClientError),
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
}

/// CognitiveMirror analyzes a page's block reference graph.
#[derive(Clone)]
pub struct CognitiveMirror {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
}

impl std::fmt::Debug for CognitiveMirror {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CognitiveMirror")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("ai_client", &"Arc<dyn AIClient>")
            .finish()
    }
}

impl CognitiveMirror {
    /// Create a new CognitiveMirror
    pub fn new(block_repo: Arc<dyn BlockRepository>, ai_client: Arc<dyn AIClient>) -> Self {
        Self {
            block_repo,
            ai_client,
        }
    }

    /// Analyze a page's blocks and produce a full CognitiveMap.
    #[instrument(skip(self))]
    pub async fn analyze(&self, page_id: Uuid) -> Result<CognitiveMap, CognitiveError> {
        let blocks = self.block_repo.get_by_page(page_id).await?;
        let mut map = build_cognitive_map(&blocks);

        // Use AI to detect cluster themes if clusters exist
        if !map.clusters.is_empty() {
            map.clusters = self.detect_cluster_themes(&blocks, map.clusters).await;
        }

        Ok(map)
    }

    /// Analyze arbitrary blocks (useful for testing or cross-page analysis).
    #[instrument(skip(self, blocks))]
    pub async fn analyze_blocks(&self, blocks: &[quilt_domain::entities::Block]) -> CognitiveMap {
        let mut map = build_cognitive_map(blocks);

        // Use AI to detect cluster themes if clusters exist
        if !map.clusters.is_empty() {
            map.clusters = self.detect_cluster_themes(blocks, map.clusters).await;
        }

        map
    }

    /// Use AI to detect themes for each cluster based on block contents.
    async fn detect_cluster_themes(
        &self,
        blocks: &[Block],
        mut clusters: Vec<KnowledgeCluster>,
    ) -> Vec<KnowledgeCluster> {
        let block_content: std::collections::HashMap<Uuid, &str> =
            blocks.iter().map(|b| (b.id, b.content.as_str())).collect();

        for cluster in &mut clusters {
            if cluster.block_ids.len() > 1 {
                let contents: String = cluster
                    .block_ids
                    .iter()
                    .filter_map(|id| block_content.get(id).map(|s| (*s).to_string()))
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(" ");

                if contents.len() > 20 {
                    let system_prompt = "You are a knowledge graph analyzer. Given a list of related text blocks, identify the common theme or topic in 1-3 words. Respond only with the theme.";
                    if let Ok(theme) = self.ai_client.chat(system_prompt, &contents).await {
                        let clean_theme = theme.trim().lines().next().unwrap_or(&theme).trim();
                        if !clean_theme.is_empty() && clean_theme.len() < 50 {
                            cluster.theme = Some(clean_theme.to_string());
                        }
                    }
                }
            }
        }

        clusters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_client::MockAIClient;
    use crate::cognitive_mirror::LightweightGraph;
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
            journal_day: None,
            updated_journal_day: None,
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

        async fn hard_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
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
        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_analyze_empty_page() {
        let page_id = uuid_from_u8(1);
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let mirror = CognitiveMirror::new(repo, ai);
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
        let ai = Arc::new(MockAIClient::new());
        let mirror = CognitiveMirror::new(repo, ai);
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
        let ai = Arc::new(MockAIClient::new());
        let mirror = CognitiveMirror::new(repo, ai);
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
        let ai = Arc::new(MockAIClient::new());
        let mirror = CognitiveMirror::new(repo, ai);
        let map = mirror.analyze(page_id).await.unwrap();
        assert_eq!(map.clusters.len(), 1);
        // In a star graph the center has the most outgoing refs (5 vs 0 for leaves)
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
        let ai = Arc::new(MockAIClient::new());
        let mirror = CognitiveMirror::new(repo, ai);
        let map = mirror.analyze_blocks(&blocks).await;
        assert_eq!(map.clusters.len(), 2);
    }
}
