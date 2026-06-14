use crate::heuristic::types::IntentRule;
use quilt_domain::value_objects::Uuid;
use crate::heuristic::shared::{contains_any, extract_after_pattern};
use petgraph::visit::EdgeRef;

use crate::ast::QueryAst;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};

/// RelatedTo rule: "tareas relacionadas con X", "bloques vinculados a Y"
///
/// Finds the target block by name/content, then returns its 1-hop or 2-hop neighbors.
pub struct RelatedToRule;

impl RelatedToRule {
    pub(crate) fn find_target_block<'a>(input: &str, _graph: &DiGraph<Uuid, f32>, node_map: &HashMap<Uuid, NodeIndex>) -> Option<Uuid> {
        // Extract the target name from patterns like "related to X" or "vinculado a Y"
        let target = extract_after_pattern(input, &[
            "related to ", "related to:", "vinculado a ", "vinculados a ",
            "bloques relacionados con ", "bloque relacionado con ",
            "tareas relacionadas con ", "tarea relacionada con ",
        ])?;

        let target_lower = target.to_lowercase();
        // Search for a node whose content matches the target (we use node ID as proxy
        // since we don't have block content in the graph). In practice, the caller
        // resolves the name to a block ID before building the graph.
        // For heuristic matching, we look for any node whose debug representation
        // contains the target string.
        for (uuid, _) in node_map {
            if uuid.to_string().to_lowercase().contains(&target_lower) {
                return Some(*uuid);
            }
        }
        None
    }

    pub(crate) fn get_neighbors(uuid: Uuid, graph: &DiGraph<Uuid, f32>, node_map: &HashMap<Uuid, NodeIndex>, hops: usize) -> Vec<Uuid> {
        let Some(&center_idx) = node_map.get(&uuid) else {
            return vec![];
        };

        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut queue: Vec<(NodeIndex, usize)> = vec![(center_idx, 0)];
        visited.insert(center_idx);

        while let Some((node, depth)) = queue.pop() {
            if depth >= hops {
                continue;
            }
            for edge in graph.edges(node) {
                let neighbor = edge.target();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push((neighbor, depth + 1));
                }
            }
            // Also traverse incoming edges (backlinks) for undirected neighbor search
            for edge in graph.edges(node) {
                let neighbor = edge.source();
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push((neighbor, depth + 1));
                }
            }
        }

        visited.into_iter().filter(|&idx| idx != center_idx).map(|idx| graph[idx]).collect()
    }
}

impl IntentRule for RelatedToRule {
    fn name(&self) -> &str {
        "related_to"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        // Check for graph-aware patterns
        let has_related_pattern = contains_any(&lower, &[
            "related to", "vinculado a", "vinculados a",
            "bloques relacionados con", "bloque relacionado con",
            "tareas relacionadas con", "tarea relacionada con",
        ]);

        if !has_related_pattern {
            return None;
        }

        // This rule always matches if the pattern is present; actual graph lookup
        // happens in parse_with_graph. Return a placeholder DSL.
        Some((QueryAst::And(vec![]), 0.85))
    }
}
