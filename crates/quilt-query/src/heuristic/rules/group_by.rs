use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::{contains_any, extract_after_pattern};

use crate::ast::QueryAst;

pub struct GroupByRule;

impl IntentRule for GroupByRule {
    fn name(&self) -> &str {
        "group_by"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        // English: "by {property}" or "group by {property}"
        // Spanish: "por {property}" or "agrupar por {property}"
        let property = extract_after_pattern(&lower, &["by ", "group by ", "por ", "agrupar por "]);

        let prop = match property? {
            p if contains_any(p, &["project", "proyecto"]) => "project",
            p if contains_any(p, &["status", "estado"]) => "status",
            p if contains_any(p, &["priority", "prioridad"]) => "priority",
            p if contains_any(p, &["author", "autor", "created by", "creado por"]) => "author",
            p if contains_any(p, &["tag", "tags", "etiqueta", "etiquetas"]) => "tags",
            _ => return None,
        };

        Some((
            QueryAst::GroupBy {
                inner: Box::new(QueryAst::And(vec![])),
                property: prop.to_string(),
            },
            0.8,
        ))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Project Filter (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────
