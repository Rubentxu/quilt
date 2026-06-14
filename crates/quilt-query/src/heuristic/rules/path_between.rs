use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::QueryAst;

pub struct PathBetweenRule;

impl PathBetweenRule {
    pub(crate) fn extract_two_targets(input: &str) -> Option<(String, String)> {
        // Look for "path between X and Y" or "camino entre X e Y"
        let lower = input.to_lowercase();

        // Try "between X and Y" pattern
        if let Some(idx) = lower.find(" between ") {
            let after = &lower[idx + " between ".len()..];
            if let Some(and_idx) = after.find(" and ") {
                let from = after[..and_idx].trim().to_string();
                let to = after[and_idx + " and ".len()..].trim().to_string();
                if !from.is_empty() && !to.is_empty() {
                    return Some((from, to));
                }
            }
        }

        // Try "camino entre X e Y" pattern
        if let Some(idx) = lower.find(" entre ") {
            let after = &lower[idx + " entre ".len()..];
            if let Some(e_idx) = after.find(" e ") {
                let from = after[..e_idx].trim().to_string();
                let to = after[e_idx + " e ".len()..].trim().to_string();
                if !from.is_empty() && !to.is_empty() {
                    return Some((from, to));
                }
            }
        }

        None
    }
}

impl IntentRule for PathBetweenRule {
    fn name(&self) -> &str {
        "path_between"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        let has_path_pattern = contains_any(&lower, &[
            "path between", "camino entre", "camino más corto entre",
            "shortest path between", "ruta entre",
        ]);

        if !has_path_pattern {
            return None;
        }

        if Self::extract_two_targets(input).is_none() {
            return None;
        }

        Some((QueryAst::And(vec![]), 0.9))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// HeuristicEngine
// ─────────────────────────────────────────────────────────────────────────────
