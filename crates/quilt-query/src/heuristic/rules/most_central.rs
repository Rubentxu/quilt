use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::QueryAst;

pub struct MostCentralRule;

impl IntentRule for MostCentralRule {
    fn name(&self) -> &str {
        "most_central"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        let has_central_pattern = contains_any(&lower, &[
            "most important", "most central", "most important blocks",
            "nodos centrales", "bloques más importantes",
            "bloque más importante", "centrales",
        ]);

        if !has_central_pattern {
            return None;
        }

        Some((QueryAst::And(vec![]), 0.8))
    }
}
