use crate::heuristic::types::IntentRule;
use crate::heuristic::shared::contains_any;

use crate::ast::QueryAst;

pub struct TaskStatusRule;

impl IntentRule for TaskStatusRule {
    fn name(&self) -> &str {
        "task_status"
    }

    fn matches(&self, input: &str) -> Option<(QueryAst, f32)> {
        let lower = input.to_lowercase();

        // English patterns
        if contains_any(&lower, &["open tasks", "open task", "pending tasks", "pending task"]) {
            return Some((QueryAst::Task(vec!["todo".into(), "in-progress".into()]), 0.9));
        }
        if contains_any(&lower, &["done tasks", "completed tasks", "finished tasks"]) {
            return Some((QueryAst::Task(vec!["done".into()]), 0.9));
        }
        if contains_any(&lower, &["tasks in progress", "active tasks", "working on"]) {
            return Some((QueryAst::Task(vec!["in-progress".into()]), 0.85));
        }
        if contains_any(&lower, &["cancelled tasks", "canceled tasks"]) {
            return Some((QueryAst::Task(vec!["cancelled".into()]), 0.9));
        }

        // Spanish patterns
        if contains_any(&lower, &["tareas abiertas", "tareas pendientes", "tarea abierta", "tarea pendiente"]) {
            return Some((QueryAst::Task(vec!["todo".into(), "in-progress".into()]), 0.9));
        }
        if contains_any(&lower, &["tareas terminadas", "tareas completadas", "tareas hechas", "tarea terminada"]) {
            return Some((QueryAst::Task(vec!["done".into()]), 0.9));
        }
        if contains_any(&lower, &["tareas en progreso", "tareas activas", "tarea en progreso"]) {
            return Some((QueryAst::Task(vec!["in-progress".into()]), 0.85));
        }
        if contains_any(&lower, &["tareas canceladas", "tarea cancelada"]) {
            return Some((QueryAst::Task(vec!["cancelled".into()]), 0.9));
        }

        None
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Temporal (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────
