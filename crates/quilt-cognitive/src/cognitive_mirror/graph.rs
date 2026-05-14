//! Lightweight Graph Index and Algorithms

use crate::cognitive_mirror::types::{
    CognitiveMap, InfluenceScore, KnowledgeCluster, KnowledgeGap,
};
use quilt_domain::entities::Block;
use quilt_domain::value_objects::Uuid;
use std::collections::{HashMap, HashSet};

/// Compare two Uuids by their inner bytes (lexicographic)
fn uuid_lt(a: &Uuid, b: &Uuid) -> bool {
    a.as_bytes() < b.as_bytes()
}

// ─────────────────────────────────────────────────────────────────────────────
// Lightweight Graph
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LightweightGraph {
    adj: HashMap<Uuid, Vec<Uuid>>,
    incoming: HashMap<Uuid, Vec<Uuid>>,
    nodes: HashSet<Uuid>,
}

impl LightweightGraph {
    pub fn from_blocks(blocks: &[Block]) -> Self {
        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut nodes: HashSet<Uuid> = HashSet::new();

        for block in blocks {
            nodes.insert(block.id);
            adj.entry(block.id).or_default();
            for &ref_id in &block.refs {
                adj.entry(block.id).or_default().push(ref_id);
                nodes.insert(ref_id);
            }
        }

        Self {
            adj,
            incoming: HashMap::new(),
            nodes,
        }
    }

    pub fn outgoing(&self, id: &Uuid) -> Vec<Uuid> {
        self.adj.get(id).cloned().unwrap_or_default()
    }

    pub fn incoming(&mut self, id: &Uuid) -> Vec<Uuid> {
        if self.incoming.is_empty() {
            self.build_incoming();
        }
        self.incoming.get(id).cloned().unwrap_or_default()
    }

    fn build_incoming(&mut self) {
        for (from, targets) in &self.adj {
            for &to in targets {
                self.incoming.entry(to).or_default().push(*from);
            }
        }
    }

    pub fn nodes(&self) -> &HashSet<Uuid> {
        &self.nodes
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn out_degree(&self, id: &Uuid) -> usize {
        self.adj.get(id).map(|v| v.len()).unwrap_or(0)
    }

    pub fn edges(&self) -> Vec<(Uuid, Uuid)> {
        self.adj
            .iter()
            .flat_map(|(&from, targets)| targets.iter().map(move |&to| (from, to)))
            .collect()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cluster Detection
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClusterConfig {
    pub min_cluster_size: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            min_cluster_size: 1,
        }
    }
}

pub fn detect_clusters(graph: &LightweightGraph, config: &ClusterConfig) -> Vec<KnowledgeCluster> {
    let mut visited: HashSet<Uuid> = HashSet::new();
    let mut clusters: Vec<KnowledgeCluster> = Vec::new();

    let undirected: HashMap<Uuid, HashSet<Uuid>> = graph
        .nodes()
        .iter()
        .map(|&id| {
            let mut neighbors = HashSet::new();
            neighbors.extend(graph.outgoing(&id));
            for (from, targets) in &graph.adj {
                if targets.contains(&id) {
                    neighbors.insert(*from);
                }
            }
            (id, neighbors)
        })
        .collect();

    for &node in graph.nodes() {
        if visited.contains(&node) {
            continue;
        }

        let mut cluster_ids = Vec::new();
        let mut queue = vec![node];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);
            cluster_ids.push(current);

            if let Some(neighbors) = undirected.get(&current) {
                for &n in neighbors {
                    if !visited.contains(&n) {
                        queue.push(n);
                    }
                }
            }
        }

        if cluster_ids.len() >= config.min_cluster_size {
            let coherence = compute_coherence(&cluster_ids, &undirected);
            clusters.push(KnowledgeCluster {
                block_ids: cluster_ids,
                theme: None,
                coherence_score: coherence,
            });
        }
    }

    clusters
}

fn compute_coherence(block_ids: &[Uuid], undirected: &HashMap<Uuid, HashSet<Uuid>>) -> f32 {
    if block_ids.len() <= 1 {
        return 1.0;
    }

    let mut internal_edges = 0usize;
    let mut possible_edges = 0usize;

    for (i, &a) in block_ids.iter().enumerate() {
        for &b in block_ids.iter().skip(i + 1) {
            possible_edges += 1;
            if undirected.get(&a).map(|n| n.contains(&b)).unwrap_or(false) {
                internal_edges += 1;
            }
        }
    }

    if possible_edges == 0 {
        return 0.0;
    }

    internal_edges as f32 / possible_edges as f32
}

// ─────────────────────────────────────────────────────────────────────────────
// Density Calculation
// ─────────────────────────────────────────────────────────────────────────────

pub fn compute_density(blocks: &[Block]) -> HashMap<Uuid, f32> {
    let total = blocks.len();
    let mut density = HashMap::new();

    for block in blocks {
        if total <= 1 {
            density.insert(block.id, 0.0);
        } else {
            let out_deg = block.refs.len() as f32;
            let max = (total - 1) as f32;
            density.insert(block.id, (out_deg / max).min(1.0));
        }
    }

    density
}

// ─────────────────────────────────────────────────────────────────────────────
// Frontier Detection
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FrontConfig {
    pub min_outgoing: usize,
    pub max_incoming: usize,
}

impl Default for FrontConfig {
    fn default() -> Self {
        Self {
            min_outgoing: 3,
            max_incoming: 1,
        }
    }
}

pub fn detect_frontiers(
    graph: &mut LightweightGraph,
    blocks: &[Block],
    config: &FrontConfig,
) -> Vec<Uuid> {
    blocks
        .iter()
        .filter(|block| {
            let out_count = block.refs.len();
            let in_count = graph.incoming(&block.id).len();
            out_count >= config.min_outgoing && in_count <= config.max_incoming
        })
        .map(|b| b.id)
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap Detection
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GapConfig {
    pub min_shared_refs: usize,
}

impl Default for GapConfig {
    fn default() -> Self {
        Self { min_shared_refs: 2 }
    }
}

pub fn detect_gaps(blocks: &[Block], config: &GapConfig) -> Vec<KnowledgeGap> {
    let block_ids: Vec<Uuid> = blocks.iter().map(|b| b.id).collect();
    let mut gaps = Vec::new();

    let ref_sets: HashMap<Uuid, HashSet<Uuid>> = blocks
        .iter()
        .map(|b| (b.id, b.refs.iter().copied().collect::<HashSet<_>>()))
        .collect();

    let has_direct_ref: HashSet<(Uuid, Uuid)> = blocks
        .iter()
        .flat_map(|b| {
            b.refs.iter().map(move |&r| {
                let (lo, hi) = if uuid_lt(&b.id, &r) {
                    (b.id, r)
                } else {
                    (r, b.id)
                };
                (lo, hi)
            })
        })
        .collect();

    for (i, &a_id) in block_ids.iter().enumerate() {
        for &b_id in block_ids.iter().skip(i + 1) {
            let (lo, hi) = if uuid_lt(&a_id, &b_id) {
                (a_id, b_id)
            } else {
                (b_id, a_id)
            };
            if has_direct_ref.contains(&(lo, hi)) {
                continue;
            }

            let refs_a = ref_sets.get(&a_id).unwrap();
            let refs_b = ref_sets.get(&b_id).unwrap();
            let shared: Vec<Uuid> = refs_a.intersection(refs_b).copied().collect();

            if shared.len() >= config.min_shared_refs {
                gaps.push(KnowledgeGap {
                    from: a_id,
                    to: b_id,
                    shared_refs: shared,
                });
            }
        }
    }

    gaps
}

// ─────────────────────────────────────────────────────────────────────────────
// PageRank-lite Influence Mapping
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct InfluenceConfig {
    pub damping: f32,
    pub max_iter: usize,
    pub threshold: f32,
}

impl Default for InfluenceConfig {
    fn default() -> Self {
        Self {
            damping: 0.85,
            max_iter: 100,
            threshold: 1e-6,
        }
    }
}

pub fn compute_influence(blocks: &[Block], config: &InfluenceConfig) -> Vec<InfluenceScore> {
    let ids: Vec<Uuid> = blocks.iter().map(|b| b.id).collect();
    let n = ids.len();

    if n == 0 {
        return Vec::new();
    }

    let id_to_idx: HashMap<Uuid, usize> = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    let adj: Vec<Vec<usize>> = ids
        .iter()
        .map(|&id| {
            blocks
                .iter()
                .find(|b| b.id == id)
                .map(|b| {
                    b.refs
                        .iter()
                        .filter_map(|&r| id_to_idx.get(&r).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut scores = vec![1.0 / n as f32; n];
    let damping = config.damping;
    let teleport = (1.0 - damping) / n as f32;

    for _ in 0..config.max_iter {
        let mut new_scores = vec![0.0f32; n];

        for (i, neighbors) in adj.iter().enumerate() {
            if neighbors.is_empty() {
                continue;
            }
            let share = scores[i] / neighbors.len() as f32;
            for &j in neighbors {
                new_scores[j] += damping * share;
            }
        }

        for s in &mut new_scores {
            *s += teleport;
        }

        let diff: f32 = scores
            .iter()
            .zip(&new_scores)
            .map(|(a, b)| (a - b).abs())
            .sum();

        scores = new_scores;

        if diff < config.threshold {
            break;
        }
    }

    let mut result: Vec<InfluenceScore> = ids
        .into_iter()
        .zip(scores)
        .map(|(id, score)| InfluenceScore {
            block_id: id,
            influence_score: score,
        })
        .collect();

    result.sort_by(|a, b| b.influence_score.partial_cmp(&a.influence_score).unwrap());
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Full CognitiveMap construction
// ─────────────────────────────────────────────────────────────────────────────

pub fn build_cognitive_map(blocks: &[Block]) -> CognitiveMap {
    let mut graph = LightweightGraph::from_blocks(blocks);

    let cluster_config = ClusterConfig::default();
    let clusters = detect_clusters(&graph, &cluster_config);

    let density = compute_density(blocks);

    let front_config = FrontConfig::default();
    let frontiers = detect_frontiers(&mut graph, blocks, &front_config);

    let gap_config = GapConfig::default();
    let gaps = detect_gaps(blocks, &gap_config);

    let influence_config = InfluenceConfig::default();
    let influences = compute_influence(blocks, &influence_config);

    CognitiveMap {
        clusters,
        density,
        frontiers,
        gaps,
        influences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::value_objects::BlockFormat;
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

    #[test]
    fn test_from_blocks_empty() {
        let blocks: Vec<Block> = Vec::new();
        let graph = LightweightGraph::from_blocks(&blocks);
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_cluster_two_connected() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let blocks = vec![
            make_block(a, vec![b], page_id),
            make_block(b, vec![a], page_id),
        ];
        let graph = LightweightGraph::from_blocks(&blocks);
        let clusters = detect_clusters(&graph, &ClusterConfig::default());
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].block_ids.len(), 2);
    }

    #[test]
    fn test_cluster_two_disconnected() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let blocks = vec![
            make_block(a, vec![], page_id),
            make_block(b, vec![], page_id),
        ];
        let graph = LightweightGraph::from_blocks(&blocks);
        let clusters = detect_clusters(&graph, &ClusterConfig::default());
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_density_all_connected() {
        let page_id = uuid_from_u8(1);
        let ids: Vec<Uuid> = (0..5).map(uuid_from_u8).collect();
        let blocks: Vec<Block> = ids
            .iter()
            .map(|&id| {
                let refs: Vec<Uuid> = ids.iter().filter(|&&x| x != id).cloned().collect();
                make_block(id, refs, page_id)
            })
            .collect();
        let density = compute_density(&blocks);
        for block in &blocks {
            assert_eq!(density[&block.id], 1.0);
        }
    }

    #[test]
    fn test_density_none() {
        let page_id = uuid_from_u8(1);
        let block = make_block(uuid_from_u8(10), vec![], page_id);
        let density = compute_density(&[block]);
        assert_eq!(density[&uuid_from_u8(10)], 0.0);
    }

    #[test]
    fn test_density_single_block() {
        let page_id = uuid_from_u8(1);
        let block = make_block(uuid_from_u8(10), vec![uuid_from_u8(11)], page_id);
        let density = compute_density(&[block]);
        assert_eq!(density[&uuid_from_u8(10)], 0.0);
    }

    #[test]
    fn test_frontier_detected() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        // a refs 5 others; leaves ref nothing (isolated knowledge spreading from a)
        let refs_to_a: Vec<Uuid> = (11..16).map(uuid_from_u8).collect();
        let blocks = vec![
            make_block(a, refs_to_a.clone(), page_id),
            make_block(uuid_from_u8(11), vec![], page_id),
            make_block(uuid_from_u8(12), vec![], page_id),
            make_block(uuid_from_u8(13), vec![], page_id),
            make_block(uuid_from_u8(14), vec![], page_id),
            make_block(uuid_from_u8(15), vec![], page_id),
        ];
        let mut graph = LightweightGraph::from_blocks(&blocks);
        let frontiers = detect_frontiers(&mut graph, &blocks, &FrontConfig::default());
        // a: outgoing=5, incoming=0 → frontier=true
        assert!(frontiers.contains(&a));
    }

    #[test]
    fn test_frontier_not_hub() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let refs_to_a: Vec<Uuid> = (11..16).map(uuid_from_u8).collect();
        let mut blocks = vec![make_block(a, refs_to_a.clone(), page_id)];
        for &ref_id in &refs_to_a {
            blocks.push(make_block(ref_id, vec![a], page_id));
        }
        let mut graph = LightweightGraph::from_blocks(&blocks);
        let frontiers = detect_frontiers(&mut graph, &blocks, &FrontConfig::default());
        assert!(!frontiers.contains(&a));
    }

    #[test]
    fn test_gap_detected() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);
        let d = uuid_from_u8(13);
        let blocks = vec![
            make_block(a, vec![c, d], page_id),
            make_block(b, vec![c, d], page_id),
            make_block(c, vec![], page_id),
            make_block(d, vec![], page_id),
        ];
        let gaps = detect_gaps(&blocks, &GapConfig::default());
        assert_eq!(gaps.len(), 1);
        assert!(gaps[0].shared_refs.contains(&c));
        assert!(gaps[0].shared_refs.contains(&d));
    }

    #[test]
    fn test_gap_not_when_direct_ref() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);
        let d = uuid_from_u8(13);
        let blocks = vec![
            make_block(a, vec![b, c, d], page_id),
            make_block(b, vec![c, d], page_id),
            make_block(c, vec![], page_id),
            make_block(d, vec![], page_id),
        ];
        let gaps = detect_gaps(&blocks, &GapConfig::default());
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_gap_not_when_insufficient() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let c = uuid_from_u8(12);
        let blocks = vec![
            make_block(a, vec![c], page_id),
            make_block(b, vec![c], page_id),
            make_block(c, vec![], page_id),
        ];
        let gaps = detect_gaps(&blocks, &GapConfig::default());
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_influence_star_graph() {
        let page_id = uuid_from_u8(1);
        let center = uuid_from_u8(10);
        let leaves: Vec<Uuid> = (11..16).map(uuid_from_u8).collect();
        let mut blocks = vec![make_block(center, leaves.clone(), page_id)];
        for &leaf in &leaves {
            blocks.push(make_block(leaf, vec![center], page_id));
        }
        let result = compute_influence(&blocks, &InfluenceConfig::default());
        let center_score = result.iter().find(|s| s.block_id == center).unwrap();
        assert!(center_score.influence_score > 0.3);
    }

    #[test]
    fn test_build_cognitive_map_empty() {
        let map = build_cognitive_map(&[]);
        assert!(map.clusters.is_empty());
        assert!(map.density.is_empty());
        assert!(map.frontiers.is_empty());
        assert!(map.gaps.is_empty());
        assert!(map.influences.is_empty());
    }

    #[test]
    fn test_build_cognitive_map_single_block() {
        let page_id = uuid_from_u8(1);
        let block = make_block(uuid_from_u8(10), vec![], page_id);
        let map = build_cognitive_map(&[block]);
        assert_eq!(map.clusters.len(), 1);
        assert_eq!(map.clusters[0].block_ids.len(), 1);
        assert_eq!(map.frontiers.len(), 0);
    }
}
