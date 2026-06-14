use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::{extract_after_pattern, capitalize_first};

use crate::ast::{QueryAst, QueryValue};

pub struct ProjectFilterRule;

impl IntentRule for ProjectFilterRule {
    fn name(&self) -> &str {
        "project_filter"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        let project_name = extract_after_pattern(&lower, &["project ", "proyecto "]);

        let name = project_name?;

        if name.is_empty() {
            return None;
        }

        // Capitalize first letter for the property value
        let formatted = capitalize_first(name);

        Some((
            QueryAst::Property {
                key: "project".to_string(),
                op: crate::property_op::PropertyOp::Equals,
                value: QueryValue::String(formatted),
                value2: None,
            },
            0.85,
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Review Status (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────
