use crate::ast::QueryAst;
use crate::heuristic::types::{GraphResult, GraphResultType, IntentResult, IntentRule};
use crate::heuristic::rules::*;
use crate::heuristic::shared::*;
use petgraph::algo::{dijkstra, tarjan_scc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use quilt_domain::value_objects::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// Engine that runs all intent rules and picks the best match.
pub struct HeuristicEngine {
    text_rules: Vec<Arc<dyn IntentRule>>,
    graph_rules: Vec<Arc<dyn IntentRule>>,
}

impl HeuristicEngine {
    /// Create a new engine with all default rules.
    pub fn new() -> Self {
        Self {
            text_rules: vec![
                Arc::new(CombinedRule),
                Arc::new(TaskStatusRule),
                Arc::new(TemporalRule),
                Arc::new(GroupByRule),
                Arc::new(ProjectFilterRule),
                Arc::new(ReviewStatusRule),
                Arc::new(CreatedByRule),
                Arc::new(MostCentralRule),
            ],
            graph_rules: vec![
                Arc::new(RelatedToRule),
                Arc::new(ConnectedToRule),
                Arc::new(PathBetweenRule),
            ],
        }
    }

    /// Parse natural language input into an IntentResult.
    ///
    /// Graph-aware rules (`RelatedTo`, `ConnectedTo`, `MostCentral`, `PathBetween`)
    /// require `parse_with_graph` to be called instead, otherwise they silently
    /// produce no match.
    pub fn parse(&self, input: &str) -> Option<IntentResult> {
        self.parse_with_graph_impl(input, None, &HashMap::new())
    }

    /// Parse natural language input into an IntentResult with optional graph access.
    ///
    /// When `graph` is `Some`, graph-aware rules are evaluated and can produce
    /// enriched results with `graph_result` field populated.
    ///
    /// `node_map` is required when `graph` is `Some` — it maps block UUIDs to
    /// petgraph node indices. The caller builds this during graph construction.
    #[instrument(skip(self, graph, node_map))]
    pub fn parse_with_graph(
        &self,
        input: &str,
        graph: Option<&DiGraph<Uuid, f32>>,
        node_map: &HashMap<Uuid, NodeIndex>,
    ) -> Option<IntentResult> {
        self.parse_with_graph_impl(input, graph, node_map)
    }

    #[instrument(skip_all)]
    fn parse_with_graph_impl(
        &self,
        input: &str,
        graph: Option<&DiGraph<Uuid, f32>>,
        node_map: &HashMap<Uuid, NodeIndex>,
    ) -> Option<IntentResult> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        let mut best: Option<(QueryAst, f32, &str, Option<GraphResult>)> = None;

        // Evaluate text rules first
        for rule in &self.text_rules {
            if let Some((ast, confidence)) = rule.matches(input) {
                if best.as_ref().map_or(true, |(_, c, _, _)| confidence > *c) {
                    best = Some((ast, confidence, rule.name(), None));
                }
            }
        }

        // Evaluate graph rules if graph is available
        if let (Some(graph), true) = (graph, graph.is_some()) {
            for rule in &self.graph_rules {
                if let Some((ast, confidence, graph_result)) = self.evaluate_graph_rule(
                    rule.as_ref(),
                    input,
                    graph,
                    node_map,
                ) {
                    if best.as_ref().map_or(true, |(_, c, _, _)| confidence > *c) {
                        best = Some((ast, confidence, rule.name(), graph_result));
                    }
                }
            }
        }

        best.map(|(ast, confidence, rule_name, graph_result)| {
            let dsl = if let Some(ref gr) = graph_result {
                // For graph results, generate DSL using the block IDs
                let ids = &gr.block_ids;
                if !ids.is_empty() {
                    let id_strs: Vec<String> = ids.iter().map(|u| format!("\"{}\"", u)).collect();
                    format!("(id:: [{}])", id_strs.join(" "))
                } else {
                    ast_to_dsl(&ast)
                }
            } else {
                ast_to_dsl(&ast)
            };

            let explanation = if let Some(ref gr) = graph_result {
                match gr.result_type {
                    GraphResultType::Neighbors => {
                        format!(
                            "Matched rule '{}' — found {} neighbor blocks",
                            rule_name,
                            gr.block_ids.len()
                        )
                    }
                    GraphResultType::ConnectedComponent => {
                        format!(
                            "Matched rule '{}' — found component with {} blocks",
                            rule_name,
                            gr.block_ids.len()
                        )
                    }
                    GraphResultType::MostCentral => {
                        format!(
                            "Matched rule '{}' — top {} central blocks",
                            rule_name,
                            gr.block_ids.len()
                        )
                    }
                    GraphResultType::PathBetween => {
                        format!(
                            "Matched rule '{}' — shortest path with {} hops",
                            rule_name,
                            gr.distance.unwrap_or(-1)
                        )
                    }
                }
            } else {
                format!("Matched rule '{}' with confidence {:.0}%", rule_name, confidence * 100.0)
            };

            IntentResult {
                ast,
                dsl,
                confidence,
                explanation,
                graph_result,
            }
        })
    }

    #[instrument(skip_all)]
    fn evaluate_graph_rule(
        &self,
        rule: &dyn IntentRule,
        input: &str,
        graph: &DiGraph<Uuid, f32>,
        node_map: &HashMap<Uuid, NodeIndex>,
    ) -> Option<(QueryAst, f32, Option<GraphResult>)> {
        let ast_and_conf = rule.matches(input)?;

        // RelatedTo rule
        if rule.name() == "related_to" {
            let target = RelatedToRule::find_target_block(input, graph, node_map)?;
            let hops = if input.contains("2-hop") || input.contains("2 hops") { 2 } else { 1 };
            let neighbors = RelatedToRule::get_neighbors(target, graph, node_map, hops);
            let graph_result = GraphResult {
                result_type: GraphResultType::Neighbors,
                center_block_id: Some(target),
                block_ids: neighbors.clone(),
                block_names: vec![], // Names not available in graph context
                path: None,
                distance: None,
            };
            // Generate DSL with neighbor IDs
            let dsl = if !neighbors.is_empty() {
                QueryAst::Ids(neighbors.iter().map(|u| u.to_string()).collect())
            } else {
                ast_and_conf.0.clone()
            };
            return Some((dsl, ast_and_conf.1, Some(graph_result)));
        }

        // ConnectedTo rule
        if rule.name() == "connected_to" {
            let target = ConnectedToRule::find_target_block(input, node_map)?;
            let Some(&target_idx) = node_map.get(&target) else {
                return None;
            };

            // Run tarjan_scc to find connected components
            let sccs: Vec<Vec<NodeIndex>> = tarjan_scc(graph);

            // Find which component contains the target
            let component: Vec<Uuid> = sccs
                .into_iter()
                .find(|comp| comp.contains(&target_idx))
                .map(|nodes| nodes.into_iter().map(|idx| graph[idx]).collect())
                .unwrap_or_default();

            let graph_result = GraphResult {
                result_type: GraphResultType::ConnectedComponent,
                center_block_id: Some(target),
                block_ids: component.clone(),
                block_names: vec![],
                path: None,
                distance: None,
            };
            let dsl = if !component.is_empty() {
                QueryAst::Ids(component.iter().map(|u| u.to_string()).collect())
            } else {
                ast_and_conf.0.clone()
            };
            return Some((dsl, ast_and_conf.1, Some(graph_result)));
        }

        // MostCentral rule
        if rule.name() == "most_central" {
            let top_n = extract_top_n(input).unwrap_or(10);

            let ids: Vec<Uuid> = graph
                .node_indices()
                .map(|idx| graph[idx])
                .collect();
            let n = ids.len();

            if n == 0 {
                return Some((ast_and_conf.0, ast_and_conf.1, None));
            }

            let id_to_idx: HashMap<Uuid, usize> =
                ids.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();

            let adj: Vec<Vec<usize>> = ids
                .iter()
                .map(|id| {
                    let idx = id_to_idx[id];
                    graph
                        .neighbors(NodeIndex::new(idx))
                        .map(|neighbor_idx| neighbor_idx.index())
                        .collect()
                })
                .collect();

            // Power iteration for eigenvector centrality
            let damping = 0.85f32;
            let mut scores = vec![1.0 / n as f32; n];
            let max_iter = 100;
            let threshold = 1e-6f32;

            for _ in 0..max_iter {
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
                let teleport = (1.0 - damping) / n as f32;
                for s in &mut new_scores {
                    *s += teleport;
                }

                let diff: f32 = scores
                    .iter()
                    .zip(&new_scores)
                    .map(|(a, b)| (a - b).abs())
                    .sum();
                scores = new_scores;

                if diff < threshold {
                    break;
                }
            }

            let mut result: Vec<(Uuid, f32)> = ids
                .into_iter()
                .zip(scores)
                .collect();
            result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

            let top_ids: Vec<Uuid> = result.into_iter().take(top_n).map(|(id, _)| id).collect();

            let graph_result = GraphResult {
                result_type: GraphResultType::MostCentral,
                center_block_id: None,
                block_ids: top_ids.clone(),
                block_names: vec![],
                path: None,
                distance: None,
            };
            let dsl = QueryAst::Ids(top_ids.iter().map(|u| u.to_string()).collect());
            return Some((dsl, ast_and_conf.1, Some(graph_result)));
        }

        // PathBetween rule
        if rule.name() == "path_between" {
            let (from_name, to_name) = PathBetweenRule::extract_two_targets(input)?;

            // Find the source and target nodes
            let from_id = node_map
                .keys()
                .find(|u| u.to_string().to_lowercase().contains(&from_name.to_lowercase()))
                .copied();
            let to_id = node_map
                .keys()
                .find(|u| u.to_string().to_lowercase().contains(&to_name.to_lowercase()))
                .copied();

            let (Some(from_id), Some(to_id)) = (from_id, to_id) else {
                return None;
            };

            let Some(&from_idx) = node_map.get(&from_id) else {
                return None;
            };
            let Some(&to_idx) = node_map.get(&to_id) else {
                return None;
            };

            // Dijkstra shortest path
            let dists: HashMap<NodeIndex, f32> =
                dijkstra(graph, from_idx, Some(to_idx), |e| *e.weight())
                    .into_iter()
                    .collect();

            let distance = dists.get(&to_idx).copied();

            // Reconstruct path using BFS
            let path_ids: Vec<Uuid> = if distance.is_some() {
                let mut queue = vec![from_idx];
                let mut came_from: HashMap<NodeIndex, NodeIndex> = HashMap::new();
                came_from.insert(from_idx, from_idx);

                while let Some(node) = queue.pop() {
                    if node == to_idx {
                        break;
                    }
                    for edge_ref in graph.edges(node) {
                        let neighbor = edge_ref.target();
                        if !came_from.contains_key(&neighbor) {
                            came_from.insert(neighbor, node);
                            queue.push(neighbor);
                        }
                    }
                }

                let mut path = Vec::new();
                if came_from.contains_key(&to_idx) {
                    let mut curr = to_idx;
                    path.push(graph[curr]);
                    while curr != from_idx {
                        if let Some(&prev) = came_from.get(&curr) {
                            curr = prev;
                            path.push(graph[curr]);
                        } else {
                            break;
                        }
                    }
                    path.reverse();
                }
                path
            } else {
                vec![]
            };

            let dist = distance.map(|d| d as i32).unwrap_or(-1);

            let graph_result = GraphResult {
                result_type: GraphResultType::PathBetween,
                center_block_id: Some(from_id),
                block_ids: path_ids.clone(),
                block_names: vec![],
                path: Some(path_ids),
                distance: Some(dist),
            };

            // PathBetween can't be expressed as DSL, so we use a placeholder
            return Some((ast_and_conf.0, ast_and_conf.1, Some(graph_result)));
        }

        Some((ast_and_conf.0, ast_and_conf.1, None))
    }
}

impl Default for HeuristicEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

