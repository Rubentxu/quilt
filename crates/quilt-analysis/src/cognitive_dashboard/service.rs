//! Cognitive Dashboard service
//!
//! Builds a global (cross-page) knowledge graph by iterating all pages
//! and running the structural mirror analysis on the aggregated block set.

use crate::cognitive_dashboard::types::{CognitiveGraphDto, GraphCluster, GraphEdge, GraphNode};
use crate::structural_mirror::{LightweightGraph, build_structure_map};
use chrono::Utc;
use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::instrument;

/// Service that produces the cognitive graph snapshot.
#[derive(Clone)]
pub struct CognitiveDashboardService {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
}

impl std::fmt::Debug for CognitiveDashboardService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CognitiveDashboardService")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("page_repo", &"Arc<dyn PageRepository>")
            .finish()
    }
}

impl CognitiveDashboardService {
    pub fn new(block_repo: Arc<dyn BlockRepository>, page_repo: Arc<dyn PageRepository>) -> Self {
        Self {
            block_repo,
            page_repo,
        }
    }

    /// Build the cognitive graph for the entire knowledge graph.
    #[instrument(skip(self))]
    pub async fn build_graph(&self) -> CognitiveGraphDto {
        let now = Utc::now();

        // Collect all blocks across all pages
        let all_blocks = self.collect_all_blocks().await;
        if all_blocks.is_empty() {
            return CognitiveGraphDto {
                nodes: Vec::new(),
                edges: Vec::new(),
                clusters: Vec::new(),
                frontier_nodes: Vec::new(),
                gap_nodes: Vec::new(),
                generated_at: now.to_rfc3339(),
            };
        }

        // Build a page_name lookup
        let page_name_map = self.build_page_name_map().await;

        // Run structural mirror analysis on all blocks
        let structure_map = build_structure_map(&all_blocks);

        // Build node info: block_id -> (page_name, content_preview, influence)
        let block_influence: HashMap<_, _> = structure_map
            .influences
            .iter()
            .map(|s| (s.block_id, s.influence_score))
            .collect();

        let frontier_set: HashSet<_> = structure_map.frontiers.iter().collect();
        let gap_set: HashSet<_> = structure_map
            .gaps
            .iter()
            .flat_map(|g| [g.from, g.to])
            .collect();

        // Build nodes
        let mut nodes = Vec::with_capacity(all_blocks.len());
        for block in &all_blocks {
            let page_name = page_name_map
                .get(&block.page_id)
                .cloned()
                .unwrap_or_else(|| format!("page:{}", block.page_id));

            let content_preview = if block.content.len() > 120 {
                block.content[..120].to_string()
            } else {
                block.content.clone()
            };

            let is_frontier = frontier_set.contains(&block.id);
            let is_gap = gap_set.contains(&block.id);

            nodes.push(GraphNode {
                id: block.id.to_string(),
                block_id: block.id.to_string(),
                page_id: block.page_id.to_string(),
                page_name,
                content_preview,
                influence_score: *block_influence.get(&block.id).unwrap_or(&0.0),
                is_frontier,
                is_gap,
                cluster_id: None, // filled below
            });
        }

        // Build edges from the lightweight graph
        let graph = LightweightGraph::from_blocks(&all_blocks);
        let edges: Vec<GraphEdge> = graph
            .edges()
            .into_iter()
            .map(|(from, to)| GraphEdge {
                from: from.to_string(),
                to: to.to_string(),
            })
            .collect();

        // Build clusters
        let cluster_id_map: HashMap<_, _> = structure_map
            .clusters
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let id = format!("cluster-{}", i);
                (c.block_ids.as_slice(), id)
            })
            .collect();

        // Assign cluster IDs to nodes
        for node in &mut nodes {
            for (block_ids, cluster_id) in &cluster_id_map {
                if let Ok(node_uuid) = quilt_domain::value_objects::Uuid::parse_str(&node.block_id)
                {
                    if block_ids.contains(&node_uuid) {
                        node.cluster_id = Some(cluster_id.clone());
                        break;
                    }
                }
            }
        }

        let clusters: Vec<GraphCluster> = structure_map
            .clusters
            .iter()
            .enumerate()
            .map(|(i, c)| GraphCluster {
                id: format!("cluster-{}", i),
                block_ids: c.block_ids.iter().map(|u| u.to_string()).collect(),
                theme: c.theme.clone(),
                coherence_score: c.coherence_score,
            })
            .collect();

        let frontier_nodes: Vec<String> = structure_map
            .frontiers
            .iter()
            .map(|u| u.to_string())
            .collect();

        let gap_nodes: Vec<String> = structure_map
            .gaps
            .iter()
            .flat_map(|g| [g.from, g.to])
            .collect::<HashSet<_>>()
            .iter()
            .map(|u| u.to_string())
            .collect();

        CognitiveGraphDto {
            nodes,
            edges,
            clusters,
            frontier_nodes,
            gap_nodes,
            generated_at: now.to_rfc3339(),
        }
    }

    /// Collect all blocks from all pages.
    async fn collect_all_blocks(&self) -> Vec<Block> {
        let pages = match self.page_repo.get_all().await {
            Ok(pages) => pages,
            Err(_) => return Vec::new(),
        };

        let mut all_blocks = Vec::new();
        for page in pages {
            match self.block_repo.get_by_page(page.id).await {
                Ok(blocks) => all_blocks.extend(blocks),
                Err(_) => continue,
            }
        }
        all_blocks
    }

    /// Build a page_id -> page_name map for all pages.
    async fn build_page_name_map(&self) -> HashMap<quilt_domain::value_objects::Uuid, String> {
        let pages = match self.page_repo.get_all().await {
            Ok(pages) => pages,
            Err(_) => return HashMap::new(),
        };

        pages.into_iter().map(|p| (p.id, p.name)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::entities::Block;
    use quilt_domain::errors::DomainError;
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, BlockType, Uuid};
    use std::collections::HashMap;

    fn make_block(id: Uuid, refs: Vec<Uuid>, page_id: Uuid, content: &str) -> Block {
        Block {
            id,
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            marker: None,
            priority: None,
            content: content.to_string(),
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
            ..Default::default()
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

    #[derive(Debug, Clone, Default)]
    struct MockPageRepo {
        pages: Vec<quilt_domain::entities::Page>,
    }

    impl MockBlockRepo {
        fn add_page(&mut self, page_id: Uuid, blocks: Vec<Block>) {
            self.pages.insert(page_id, blocks);
        }
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
            Err(DomainError::NotImplemented("mock"))
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
        async fn move_block(&self, _: Uuid, _: Option<Uuid>, _: f64) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn search(&self, _: &str, _: usize) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn count_by_page(&self, _: Uuid) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn count_all(&self) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn query_dsl(&self, _: &str, _: &[String]) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage("mock".to_string()))
        }
        async fn list_by_property(
            &self,
            _: &str,
            _: &str,
            _: usize,
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage("mock".to_string()))
        }
        async fn list_distinct_keys(
            &self,
            _: Option<&str>,
            _: u32,
        ) -> Result<Vec<String>, DomainError> {
            Err(DomainError::Storage("mock".to_string()))
        }
        async fn list_by_property_key(&self, _: &str, _: u32) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage("mock".to_string()))
        }
        async fn list_distinct_authors(&self, _: Option<&str>) -> Result<Vec<String>, DomainError> {
            Ok(vec![])
        }
    }

    impl MockPageRepo {
        fn add_pages(&mut self, pages: Vec<quilt_domain::entities::Page>) {
            self.pages = pages;
        }
    }

    #[async_trait]
    impl PageRepository for MockPageRepo {
        async fn get_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<quilt_domain::entities::Page>, DomainError> {
            Ok(None)
        }
        async fn get_by_name(
            &self,
            _name: &str,
        ) -> Result<Option<quilt_domain::entities::Page>, DomainError> {
            Ok(None)
        }
        async fn get_journal(
            &self,
            _day: quilt_domain::value_objects::JournalDay,
        ) -> Result<Option<quilt_domain::entities::Page>, DomainError> {
            Ok(None)
        }
        async fn get_all(&self) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(self.pages.clone())
        }
        async fn get_namespace_pages(
            &self,
            _namespace_id: Uuid,
        ) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(vec![])
        }
        async fn insert(&self, _page: &quilt_domain::entities::Page) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update(&self, _page: &quilt_domain::entities::Page) -> Result<(), DomainError> {
            Ok(())
        }
        async fn rename(&self, _id: Uuid, _new_name: &str) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_updated_since(
            &self,
            _: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(vec![])
        }
        async fn get_recent(
            &self,
            _limit: usize,
        ) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(vec![])
        }
        async fn count(&self) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn search(
            &self,
            _: &str,
            _: usize,
        ) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(vec![])
        }
        async fn search_by_name_or_title(
            &self,
            _: &str,
            _: usize,
        ) -> Result<Vec<quilt_domain::entities::Page>, DomainError> {
            Ok(vec![])
        }
        async fn update_properties(
            &self,
            _page_id: Uuid,
            _props: std::collections::HashMap<String, DefaultPropertyEntry<PropertyValue>>,
        ) -> Result<quilt_domain::entities::Page, DomainError> {
            Err(DomainError::NotImplemented("mock"))
        }
        async fn get_by_source_path(
            &self,
            _source_path: &str,
        ) -> Result<Option<quilt_domain::entities::Page>, DomainError> {
            Ok(None)
        }
        async fn update_source_mtime_cas(
            &self,
            _page_id: Uuid,
            _expected_mtime: chrono::DateTime<chrono::Utc>,
            _new_mtime: chrono::DateTime<chrono::Utc>,
        ) -> Result<bool, DomainError> {
            Ok(true)
        }
    }

    fn make_page(id: Uuid, name: &str) -> quilt_domain::entities::Page {
        quilt_domain::entities::Page {
            id,
            name: name.to_string(),
            title: Some(name.to_string()),
            journal: false,
            journal_day: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_build_graph_empty() {
        let block_repo = Arc::new(MockBlockRepo::default());
        let page_repo = Arc::new(MockPageRepo::default());
        let service = CognitiveDashboardService::new(block_repo, page_repo);
        let graph = service.build_graph().await;
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.clusters.is_empty());
    }

    #[tokio::test]
    async fn test_build_graph_single_page() {
        let page_id = uuid_from_u8(1);
        let block_a = uuid_from_u8(10);
        let block_b = uuid_from_u8(11);
        let block_c = uuid_from_u8(12);

        let blocks = vec![
            make_block(block_a, vec![block_b], page_id, "Rust is great"),
            make_block(block_b, vec![block_c], page_id, "Because of memory safety"),
            make_block(block_c, vec![], page_id, "Study shows this"),
        ];

        let pages = vec![make_page(page_id, "Test Page")];

        let mut mock_block_repo = MockBlockRepo::default();
        mock_block_repo.add_page(page_id, blocks);

        let mut mock_page_repo = MockPageRepo::default();
        mock_page_repo.add_pages(pages);

        let service =
            CognitiveDashboardService::new(Arc::new(mock_block_repo), Arc::new(mock_page_repo));
        let graph = service.build_graph().await;

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.clusters.len(), 1);
        assert!(!graph.generated_at.is_empty());
    }

    #[tokio::test]
    async fn test_build_graph_frontier_detection() {
        let page_id = uuid_from_u8(1);
        let center = uuid_from_u8(10);
        let leaves: Vec<Uuid> = (11..16).map(uuid_from_u8).collect();

        let mut blocks = vec![make_block(center, leaves.clone(), page_id, "Hub block")];
        for &leaf in &leaves {
            blocks.push(make_block(leaf, vec![], page_id, &format!("Leaf {}", leaf)));
        }

        let pages = vec![make_page(page_id, "Test Page")];

        let mut mock_block_repo = MockBlockRepo::default();
        mock_block_repo.add_page(page_id, blocks);

        let mut mock_page_repo = MockPageRepo::default();
        mock_page_repo.add_pages(pages);

        let service =
            CognitiveDashboardService::new(Arc::new(mock_block_repo), Arc::new(mock_page_repo));
        let graph = service.build_graph().await;

        // The center node should be flagged as frontier
        let center_node = graph
            .nodes
            .iter()
            .find(|n| n.block_id == center.to_string());
        assert!(
            center_node.map(|n| n.is_frontier).unwrap_or(false),
            "center should be frontier"
        );
    }

    #[tokio::test]
    async fn test_build_graph_gap_detection() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);
        let d = uuid_from_u8(13);

        // a and b both ref c and d but don't ref each other — gap
        let blocks = vec![
            make_block(a, vec![c, d], page_id, "Block A"),
            make_block(b, vec![c, d], page_id, "Block B"),
            make_block(c, vec![], page_id, "Block C"),
            make_block(d, vec![], page_id, "Block D"),
        ];

        let pages = vec![make_page(page_id, "Test Page")];

        let mut mock_block_repo = MockBlockRepo::default();
        mock_block_repo.add_page(page_id, blocks);

        let mut mock_page_repo = MockPageRepo::default();
        mock_page_repo.add_pages(pages);

        let service =
            CognitiveDashboardService::new(Arc::new(mock_block_repo), Arc::new(mock_page_repo));
        let graph = service.build_graph().await;

        // Gap nodes should be identified (a and b, which don't ref each other but share c, d)
        assert!(
            !graph.gap_nodes.is_empty(),
            "gap nodes should be detected: {:?}",
            graph.gap_nodes
        );
    }
}
