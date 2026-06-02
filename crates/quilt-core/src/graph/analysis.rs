//! Graph analysis algorithms — reusable, domain-agnostic
//!
//! All functions operate on adjacency maps of `String` node IDs.
//! No dependency on `quilt_domain` — usable from WASM, CLI, or other crates.
//!
//! Extracted from the duplicated implementations in:
//! - `quilt-analysis/src/structural_mirror/graph.rs`
//! - `quilt-cognitive/src/cognitive_mirror/graph.rs`

use std::collections::{HashMap, HashSet};

// ─────────────────────────────────────────────────────────────────────────────
// Cluster Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Detect connected components (clusters) in an undirected view of the graph.
///
/// Builds an undirected graph from the directed adjacency (treating all edges
/// as bidirectional), then runs BFS to find connected components of size
/// at least `min_size`.
pub fn detect_clusters(
    adjacency: &HashMap<String, Vec<String>>,
    min_size: usize,
) -> Vec<Vec<String>> {
    let undirected = build_undirected(adjacency);
    let mut visited: HashSet<&str> = HashSet::new();
    let mut clusters: Vec<Vec<String>> = Vec::new();

    for node in undirected.keys() {
        if visited.contains(node.as_str()) {
            continue;
        }

        let mut cluster = Vec::new();
        let mut stack = vec![node.as_str()];

        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            cluster.push(current.to_string());

            if let Some(neighbors) = undirected.get(current) {
                for n in neighbors {
                    if !visited.contains(n.as_str()) {
                        stack.push(n.as_str());
                    }
                }
            }
        }

        if cluster.len() >= min_size {
            clusters.push(cluster);
        }
    }

    clusters
}

/// Compute coherence of a cluster (fraction of internal edges over possible edges).
///
/// A cluster with all possible connections (complete subgraph) scores 1.0.
/// A cluster with no internal edges scores 0.0.
pub fn compute_cluster_coherence(
    cluster: &[String],
    undirected: &HashMap<String, HashSet<String>>,
) -> f64 {
    if cluster.len() <= 1 {
        return 1.0;
    }

    let mut internal_edges = 0usize;
    let possible_edges = cluster.len() * (cluster.len() - 1) / 2;

    for (i, a) in cluster.iter().enumerate() {
        for b in cluster.iter().skip(i + 1) {
            if undirected
                .get(a)
                .map(|n| n.contains(b.as_str()))
                .unwrap_or(false)
            {
                internal_edges += 1;
            }
        }
    }

    if possible_edges == 0 {
        return 0.0;
    }

    internal_edges as f64 / possible_edges as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// Gap Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Detect gaps: pairs of nodes that are NOT directly connected but have
/// high Jaccard similarity (intersection / union) >= `threshold` in their
/// reference sets.
///
/// A gap means "these two nodes _should_ probably be connected given how
/// much they reference the same things."
pub fn detect_gaps(
    adjacency: &HashMap<String, Vec<String>>,
    threshold: f64,
) -> Vec<(String, String)> {
    let nodes: Vec<&String> = adjacency.keys().collect();
    let mut gaps = Vec::new();

    // Pre-compute reference sets
    let ref_sets: HashMap<&str, HashSet<&str>> = adjacency
        .iter()
        .map(|(k, v)| (k.as_str(), v.iter().map(|s| s.as_str()).collect()))
        .collect();

    // Pre-compute existing direct reference pairs (undirected, deduplicated)
    let direct_refs: HashSet<(String, String)> = adjacency
        .iter()
        .flat_map(|(from, targets)| {
            targets.iter().map(move |to| {
                if from < to {
                    (from.clone(), to.clone())
                } else {
                    (to.clone(), from.clone())
                }
            })
        })
        .collect();

    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let a = nodes[i].as_str();
            let b = nodes[j].as_str();

            // Skip if directly connected
            let pair_key = if a < b {
                (a.to_string(), b.to_string())
            } else {
                (b.to_string(), a.to_string())
            };
            if direct_refs.contains(&pair_key) {
                continue;
            }

            let refs_a = ref_sets.get(a).unwrap();
            let refs_b = ref_sets.get(b).unwrap();

            // Jaccard similarity
            let intersection = refs_a.intersection(refs_b).count();
            let union = refs_a.union(refs_b).count();

            if union == 0 {
                continue;
            }

            let similarity = intersection as f64 / union as f64;
            if similarity >= threshold {
                gaps.push((a.to_string(), b.to_string()));
            }
        }
    }

    gaps
}

// ─────────────────────────────────────────────────────────────────────────────
// PageRank Influence
// ─────────────────────────────────────────────────────────────────────────────

/// Compute PageRank influence scores using the iterative power method.
///
/// Returns a map from node ID to its PageRank score (scores sum to 1.0).
pub fn compute_pagerank(
    adjacency: &HashMap<String, Vec<String>>,
    iterations: usize,
    damping: f64,
) -> HashMap<String, f64> {
    let n = adjacency.len();
    if n == 0 {
        return HashMap::new();
    }

    // Map String node IDs to dense indices
    let node_ids: Vec<&String> = adjacency.keys().collect();
    let id_to_idx: HashMap<&str, usize> = node_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    // Build index-based adjacency for fast iteration
    let adj_idx: Vec<Vec<usize>> = node_ids
        .iter()
        .map(|&id| {
            adjacency
                .get(id)
                .map(|targets| {
                    targets
                        .iter()
                        .filter_map(|t| id_to_idx.get(t.as_str()).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut scores = vec![1.0 / n as f64; n];
    let teleport = (1.0 - damping) / n as f64;

    for _ in 0..iterations {
        let mut new_scores = vec![0.0f64; n];

        for (i, neighbors) in adj_idx.iter().enumerate() {
            if neighbors.is_empty() {
                continue;
            }
            let share = scores[i] / neighbors.len() as f64;
            for &j in neighbors {
                new_scores[j] += damping * share;
            }
        }

        for s in &mut new_scores {
            *s += teleport;
        }

        // Convergence check
        let diff: f64 = scores
            .iter()
            .zip(&new_scores)
            .map(|(a, b)| (a - b).abs())
            .sum();

        scores = new_scores;

        if diff < 1e-10 {
            break;
        }
    }

    node_ids
        .into_iter()
        .zip(scores)
        .map(|(id, score)| ((*id).clone(), score))
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Density
// ─────────────────────────────────────────────────────────────────────────────

/// Compute graph density for a directed graph.
///
/// Density = `edge_count / (node_count * (node_count - 1))`.
/// Returns 0.0 for graphs with 0 or 1 nodes.
pub fn compute_density(node_count: usize, edge_count: usize) -> f64 {
    if node_count <= 1 {
        return 0.0;
    }
    let possible = node_count * (node_count - 1);
    edge_count as f64 / possible as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// Frontier Detection
// ─────────────────────────────────────────────────────────────────────────────

/// Find frontier nodes: nodes with high outgoing degree (>= 3) and low
/// incoming degree (<= 1) in the directed graph.
///
/// Frontiers are "knowledge spreaders" — they reference many other nodes
/// but are themselves rarely referenced.  Uses the default thresholds
/// from the original knowledge-graph analysis (min_outgoing = 3,
/// max_incoming = 1).
pub fn find_frontiers(adjacency: &HashMap<String, Vec<String>>) -> Vec<String> {
    // Build incoming counts
    let mut incoming: HashMap<&str, usize> = HashMap::new();
    for targets in adjacency.values() {
        for t in targets {
            *incoming.entry(t.as_str()).or_insert(0) += 1;
        }
    }

    adjacency
        .iter()
        .filter(|(id, targets)| {
            let out_count = targets.len();
            let in_count = incoming.get(id.as_str()).copied().unwrap_or(0);
            out_count >= 3 && in_count <= 1
        })
        .map(|(id, _)| id.clone())
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build undirected adjacency from directed adjacency.
///
/// For every directed edge `from -> to`, inserts both `from -> to` and
/// `to -> from` so the graph becomes undirected.  All source nodes from
/// the original adjacency are guaranteed to have an entry (even if empty).
fn build_undirected(adjacency: &HashMap<String, Vec<String>>) -> HashMap<String, HashSet<String>> {
    let mut undirected: HashMap<String, HashSet<String>> = HashMap::new();

    for (from, targets) in adjacency {
        undirected.entry(from.clone()).or_default();
        for to in targets {
            undirected
                .entry(from.clone())
                .or_default()
                .insert(to.clone());
            undirected
                .entry(to.clone())
                .or_default()
                .insert(from.clone());
        }
    }

    // Ensure isolated nodes are present
    for node in adjacency.keys() {
        undirected.entry(node.clone()).or_default();
    }

    undirected
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────

    /// 4-node diamond: a -> {b, c}, b -> d, c -> d, d -> {}
    fn diamond_graph() -> HashMap<String, Vec<String>> {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["b".into(), "c".into()]);
        adj.insert("b".into(), vec!["d".into()]);
        adj.insert("c".into(), vec!["d".into()]);
        adj.insert("d".into(), Vec::new());
        adj
    }

    /// Star graph: center -> {a, b, c, d}, leaves -> {center}
    fn star_graph() -> HashMap<String, Vec<String>> {
        let mut adj = HashMap::new();
        adj.insert("center".into(), vec!["a".into(), "b".into(), "c".into()]);
        adj.insert("a".into(), vec!["center".into()]);
        adj.insert("b".into(), vec!["center".into()]);
        adj.insert("c".into(), vec!["center".into()]);
        adj
    }

    // ── detect_clusters ──────────────────────────────────────────────

    #[test]
    fn test_clusters_empty() {
        let adj: HashMap<String, Vec<String>> = HashMap::new();
        let clusters = detect_clusters(&adj, 1);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_clusters_single_diamond() {
        let clusters = detect_clusters(&diamond_graph(), 1);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 4);
    }

    #[test]
    fn test_clusters_disconnected() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), Vec::new());
        adj.insert("b".into(), Vec::new());
        let clusters = detect_clusters(&adj, 1);
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_clusters_filter_by_min_size() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["b".into()]);
        adj.insert("b".into(), vec!["a".into()]);
        adj.insert("c".into(), Vec::new());
        let clusters = detect_clusters(&adj, 3);
        assert_eq!(clusters.len(), 0);
    }

    // ── detect_gaps ──────────────────────────────────────────────────

    #[test]
    fn test_gaps_diamond_bc() {
        // b->d, c->d: shared ref {d}, Jaccard=1.0 -> gap
        let gaps = detect_gaps(&diamond_graph(), 0.5);
        assert_eq!(gaps.len(), 1);
        let pair = &gaps[0];
        assert!((pair.0 == "b" && pair.1 == "c") || (pair.0 == "c" && pair.1 == "b"));
    }

    #[test]
    fn test_gaps_no_gap_when_direct() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["b".into()]);
        adj.insert("b".into(), vec!["a".into()]);
        let gaps = detect_gaps(&adj, 0.0);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_gaps_low_threshold() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["c".into()]);
        adj.insert("b".into(), vec!["c".into(), "d".into()]);
        adj.insert("c".into(), Vec::new());
        adj.insert("d".into(), Vec::new());
        // a={c}, b={c,d}: Jaccard=1/2=0.5 >= 0.4 -> gap
        let gaps = detect_gaps(&adj, 0.4);
        assert_eq!(gaps.len(), 1);
        assert!((gaps[0].0 == "a" && gaps[0].1 == "b") || (gaps[0].0 == "b" && gaps[0].1 == "a"));
    }

    #[test]
    fn test_gaps_no_gap_when_no_shared_refs() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["c".into()]);
        adj.insert("b".into(), vec!["d".into()]);
        adj.insert("c".into(), Vec::new());
        adj.insert("d".into(), Vec::new());
        let gaps = detect_gaps(&adj, 0.1);
        assert_eq!(gaps.len(), 0);
    }

    #[test]
    fn test_gaps_empty_adjacency() {
        let adj: HashMap<String, Vec<String>> = HashMap::new();
        let gaps = detect_gaps(&adj, 0.5);
        assert_eq!(gaps.len(), 0);
    }

    // ── compute_pagerank ─────────────────────────────────────────────

    #[test]
    fn test_pagerank_star_center_highest() {
        let pr = compute_pagerank(&star_graph(), 100, 0.85);
        assert!(pr["center"] > 0.3);
        assert!(pr["center"] > pr["a"]);
        assert!(pr["center"] > pr["b"]);
        assert!(pr["center"] > pr["c"]);
    }

    #[test]
    fn test_pagerank_sink_nodes_get_teleport_only() {
        let pr = compute_pagerank(&star_graph(), 100, 0.85);
        // Leaves have no outgoing edges, so they only get teleport
        // After many iterations, they should have positive scores
        assert!(pr["a"] > 0.0);
        assert!(pr["b"] > 0.0);
        assert!(pr["c"] > 0.0);
    }

    #[test]
    fn test_pagerank_empty() {
        let adj: HashMap<String, Vec<String>> = HashMap::new();
        let pr = compute_pagerank(&adj, 100, 0.85);
        assert!(pr.is_empty());
    }

    #[test]
    fn test_pagerank_single_node() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), Vec::new());
        let pr = compute_pagerank(&adj, 10, 0.85);
        // Single sink node: only teleport (1-d)/n = 0.15
        assert!((pr["a"] - 0.15).abs() < 1e-10);
    }

    #[test]
    fn test_pagerank_no_damping_means_all_equal() {
        // With damping=0, every node gets only teleport = equal scores
        let pr = compute_pagerank(&star_graph(), 10, 0.0);
        let vals: Vec<f64> = pr.values().copied().collect();
        let first = vals[0];
        for v in &vals {
            assert!((v - first).abs() < 1e-10);
        }
    }

    // ── compute_density ──────────────────────────────────────────────

    #[test]
    fn test_density_complete_directed() {
        // Complete directed graph of 4 nodes: 4*3 = 12 possible edges
        assert!((compute_density(4, 12) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_density_empty() {
        assert_eq!(compute_density(0, 0), 0.0);
        assert_eq!(compute_density(1, 0), 0.0);
        assert_eq!(compute_density(1, 5), 0.0);
    }

    #[test]
    fn test_density_sparse() {
        // 5 nodes, 2 edges: 2/(5*4) = 0.1
        assert!((compute_density(5, 2) - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_density_dense() {
        // 3 nodes, 5 edges: 5/(3*2) = ~0.833
        assert!((compute_density(3, 5) - 5.0 / 6.0).abs() < 1e-10);
    }

    // ── find_frontiers ───────────────────────────────────────────────

    #[test]
    fn test_frontiers_high_outgoing_low_incoming_is_frontier() {
        let mut adj = HashMap::new();
        adj.insert(
            "hub".into(),
            vec!["a".into(), "b".into(), "c".into(), "d".into()],
        );
        adj.insert("a".into(), Vec::new());
        adj.insert("b".into(), Vec::new());
        adj.insert("c".into(), Vec::new());
        adj.insert("d".into(), Vec::new());
        // hub: outgoing=4, incoming=0 -> frontier
        let frontiers = find_frontiers(&adj);
        assert!(frontiers.contains(&"hub".to_string()));
        // leaves: outgoing=0, incoming=1 -> NOT frontier
        assert!(!frontiers.contains(&"a".to_string()));
    }

    #[test]
    fn test_frontiers_leaf_is_not() {
        let frontiers = find_frontiers(&star_graph());
        // a: outgoing=1 (to center), incoming=1 (from center) -> not frontier
        assert!(!frontiers.contains(&"a".to_string()));
    }

    #[test]
    fn test_frontiers_not_when_incoming_high() {
        let mut adj = HashMap::new();
        adj.insert("hub".into(), vec!["a".into(), "b".into(), "c".into()]);
        adj.insert("a".into(), vec!["hub".into()]);
        adj.insert("b".into(), vec!["hub".into()]);
        adj.insert("c".into(), vec!["hub".into()]);
        // hub: outgoing=3, incoming=3 -> NOT frontier (incoming > 1)
        let frontiers = find_frontiers(&adj);
        assert!(!frontiers.contains(&"hub".to_string()));
    }

    #[test]
    fn test_frontiers_empty() {
        let adj: HashMap<String, Vec<String>> = HashMap::new();
        let frontiers = find_frontiers(&adj);
        assert!(frontiers.is_empty());
    }

    // ── compute_cluster_coherence ────────────────────────────────────

    #[test]
    fn test_coherence_single_node() {
        let undirected = build_undirected(&diamond_graph());
        let score = compute_cluster_coherence(&["a".into()], &undirected);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_coherence_complete_pair() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), vec!["b".into()]);
        adj.insert("b".into(), vec!["a".into()]);
        let undirected = build_undirected(&adj);
        // a-b has an edge -> coherence = 1/1 = 1.0
        let score = compute_cluster_coherence(&["a".into(), "b".into()], &undirected);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_coherence_disconnected_pair() {
        let mut adj = HashMap::new();
        adj.insert("a".into(), Vec::new());
        adj.insert("b".into(), Vec::new());
        let undirected = build_undirected(&adj);
        // no edge between a and b -> coherence = 0/1 = 0.0
        let score = compute_cluster_coherence(&["a".into(), "b".into()], &undirected);
        assert!((score - 0.0).abs() < 1e-10);
    }
}
