use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::{QueryAst, QueryValue};

pub struct ReviewStatusRule;

impl IntentRule for ReviewStatusRule {
    fn name(&self) -> &str {
        "review_status"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        if contains_any(&lower, &["unreviewed", "not reviewed", "sin revisar", "no revisados", "bloques sin revisar"]) {
            return Some((
                QueryAst::Property {
                    key: "reviewed".to_string(),
                    op: crate::property_op::PropertyOp::Equals,
                    value: QueryValue::Boolean(false),
                    value2: None,
                },
                0.85,
            ));
        }

        if contains_any(&lower, &["reviewed", "revisados", "revisadas"]) {
            return Some((
                QueryAst::Property {
                    key: "reviewed".to_string(),
                    op: crate::property_op::PropertyOp::Equals,
                    value: QueryValue::Boolean(true),
                    value2: None,
                },
                0.85,
            ));
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Created By (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────
