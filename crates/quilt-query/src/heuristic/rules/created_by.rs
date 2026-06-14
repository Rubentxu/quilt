use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::{QueryAst, QueryValue};

pub struct CreatedByRule;

impl IntentRule for CreatedByRule {
    fn name(&self) -> &str {
        "created_by"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        if contains_any(&lower, &["created by me", "my tasks", "my blocks", "creado por mi", "mis tareas", "mis bloques"]) {
            return Some((
                QueryAst::Property {
                    key: "created_by".to_string(),
                    op: crate::property_op::PropertyOp::Equals,
                    value: QueryValue::String("current_user".to_string()),
                    value2: None,
                },
                0.8,
            ));
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Combined Temporal + Status + Author
// ─────────────────────────────────────────────────────────────────────────────
