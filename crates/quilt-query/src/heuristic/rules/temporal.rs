use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::{QueryAst, TemporalRange};

pub struct TemporalRule;

impl IntentRule for TemporalRule {
    fn name(&self) -> &str {
        "temporal"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        // Determine temporal range
        let range = if contains_any(&lower, &["today", "hoy"]) {
            TemporalRange::Today
        } else if contains_any(&lower, &["yesterday", "ayer"]) {
            TemporalRange::Yesterday
        } else if contains_any(&lower, &["this week", "esta semana"]) {
            TemporalRange::ThisWeek
        } else if contains_any(&lower, &["last week", "semana pasada", "la semana pasada"]) {
            TemporalRange::LastWeek
        } else if contains_any(&lower, &["this month", "este mes"]) {
            TemporalRange::ThisMonth
        } else if contains_any(&lower, &["last month", "mes pasado", "el mes pasado"]) {
            TemporalRange::LastMonth
        } else {
            return None;
        };

        // Determine inner filter
        let inner = if contains_any(&lower, &["tasks", "task", "tareas", "tarea"]) {
            if contains_any(&lower, &["done", "completed", "finished", "terminadas", "completadas", "hechas"]) {
                QueryAst::Task(vec!["done".into()])
            } else if contains_any(&lower, &["open", "pending", "abiertas", "pendientes"]) {
                QueryAst::Task(vec!["todo".into(), "in-progress".into()])
            } else {
                // Generic "tasks this week" — all tasks in time range
                QueryAst::And(vec![])
            }
        } else if contains_any(&lower, &["created", "creado", "creada", "creados", "creadas"]) {
            // "created this week" — temporal on all blocks
            QueryAst::And(vec![])
        } else {
            QueryAst::And(vec![])
        };

        let confidence = if inner == QueryAst::And(vec![]) { 0.7 } else { 0.85 };

        Some((
            QueryAst::Temporal {
                range,
                inner: Box::new(inner),
            },
            confidence,
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Property Group By (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────
