use crate::heuristic::types::IntentRule;
use quilt_domain::value_objects::Uuid;
use crate::heuristic::shared::{contains_any, extract_after_pattern};

use crate::ast::QueryAst;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

pub struct ConnectedToRule;

impl ConnectedToRule {
    pub(crate) fn find_target_block<'a>(input: &str, node_map: &HashMap<Uuid, NodeIndex>) -> Option<Uuid> {
        let target = extract_after_pattern(input, &[
            "same cluster as", "mismo cluster que", "misma cluster que",
            "mismo componente que", "misma componente que",
            "bloques en el mismo cluster", "bloque en el mismo cluster",
        ])?;

        let target_lower = target.to_lowercase();
        for (uuid, _) in node_map {
            if uuid.to_string().to_lowercase().contains(&target_lower) {
                return Some(*uuid);
            }
        }
        None
    }
}

impl IntentRule for ConnectedToRule {
    fn name(&self) -> &str {
        "connected_to"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        let has_connected_pattern = contains_any(&lower, &[
            "same cluster as", "mismo cluster que", "misma cluster que",
            "mismo componente que", "misma componente que",
            "bloques en el mismo cluster", "bloque en el mismo cluster",
        ]);

        if !has_connected_pattern {
            return None;
        }

        Some((QueryAst::And(vec![]), 0.85))
    }
}
