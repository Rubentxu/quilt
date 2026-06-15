use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::{QueryAst, QueryValue, TemporalRange};

pub struct CombinedRule;

impl IntentRule for CombinedRule {
    fn name(&self) -> &str {
        "combined"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        // Check for combined patterns: author + status + temporal
        let has_author = contains_any(&lower, &["my ", "mi ", "mis ", "creado por mi", "created by me"]);
        let has_status = contains_any(&lower, &["done", "completed", "finished", "open", "pending",
                                                   "terminadas", "completadas", "hechas", "abiertas", "pendientes"]);
        let has_temporal = contains_any(&lower, &["today", "hoy", "this week", "esta semana",
                                                    "yesterday", "ayer", "last week", "semana pasada",
                                                    "this month", "este mes", "last month", "mes pasado"]);

        // Must have at least 2 of 3 signals for combined rule
        let signal_count = [has_author, has_status, has_temporal].iter().filter(|&&x| x).count();
        if signal_count < 2 {
            return None;
        }

        let mut parts = Vec::new();

        // Author filter
        if has_author {
            parts.push(QueryAst::Property {
                key: "created_by".to_string(),
                op: crate::property_op::PropertyOp::Equals,
                value: QueryValue::String("current_user".to_string()),
                value2: None,
            });
        }

        // Status filter
        if has_status {
            if contains_any(&lower, &["done", "completed", "finished", "terminadas", "completadas", "hechas"]) {
                parts.push(QueryAst::Task(vec!["done".into()]));
            } else if contains_any(&lower, &["open", "pending", "abiertas", "pendientes"]) {
                parts.push(QueryAst::Task(vec!["todo".into(), "doing".into()]));
            }
        }

        // Temporal filter
        if has_temporal {
            let range = if contains_any(&lower, &["today", "hoy"]) {
                TemporalRange::Today
            } else if contains_any(&lower, &["yesterday", "ayer"]) {
                TemporalRange::Yesterday
            } else if contains_any(&lower, &["this week", "esta semana"]) {
                TemporalRange::ThisWeek
            } else if contains_any(&lower, &["last week", "semana pasada"]) {
                TemporalRange::LastWeek
            } else if contains_any(&lower, &["this month", "este mes"]) {
                TemporalRange::ThisMonth
            } else {
                TemporalRange::LastMonth
            };

            // Wrap existing filters in temporal
            let inner = if parts.len() == 1 {
                parts.pop().unwrap()
            } else {
                QueryAst::And(parts)
            };

            return Some((
                QueryAst::Temporal {
                    range,
                    inner: Box::new(inner),
                },
                0.95,
            ));
        }

        // No temporal — just AND the parts
        if parts.len() == 1 {
            Some((parts.pop().unwrap(), 0.85))
        } else {
            Some((QueryAst::And(parts), 0.85))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph-Aware Rules (F3)
// ─────────────────────────────────────────────────────────────────────────────
