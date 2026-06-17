//! Task projection contract (V1, WASM mirror).
//!
//! Matches blocks where `type:: task` AND `status::` is set.
//! Produces a `TaskCheckbox` decoration with weight based on status.
//!
//! Mirrors `quilt_application::services::projection::contracts::task`
//! (slice #4). See module-level docs for the deliberate deviations
//! from server semantics.

use crate::projection::contracts::match_status_one_of;
use crate::projection::resolver::WasmContract;
use crate::projection::view::{WasmDecoration, WasmDecorationKind};
use crate::types::BlockDto;
use serde_json::json;

/// V1 task statuses — matches the server's `task_contract()` definition.
const V1_STATUSES: &[&str] = &["todo", "in-progress", "done", "cancelled", "waiting"];

/// TaskProjection — produces a task-checkbox decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskContract;

impl WasmContract for TaskContract {
    fn id(&self) -> &'static str {
        "task"
    }

    fn priority(&self) -> u32 {
        100
    }

    fn matches(&self, block: &BlockDto) -> bool {
        let Some(props) = block.properties.as_object() else {
            return false;
        };
        // type:: task AND status:: (any value) AND status:: in known statuses
        let type_matches = props.get("type").map_or(false, |v| v == &json!("task"));
        let status_set = props.contains_key("status");
        let status_known = match_status_one_of(props, "status", V1_STATUSES);
        type_matches && status_set && status_known
    }

    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration> {
        let status_value = block
            .properties
            .as_object()
            .and_then(|p| p.get("status").cloned())
            .unwrap_or_else(|| json!("todo"));

        let weight = status_weight(&status_value);
        vec![WasmDecoration {
            kind: WasmDecorationKind::TaskCheckbox,
            target: "status".to_string(),
            value: status_value,
            weight,
        }]
    }
}

/// Status-to-weight mapping — byte-equal to the server's
/// `TaskProjection::status_weight` (slice #4).
fn status_weight(status: &serde_json::Value) -> u8 {
    if let serde_json::Value::String(s) = status {
        match s.as_str() {
            "done" => 100,
            "cancelled" => 80,
            "in-progress" => 60,
            "waiting" => 40,
            "todo" => 20,
            _ => 10,
        }
    } else {
        10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_block(properties: serde_json::Value) -> BlockDto {
        BlockDto {
            id: "b1".to_string(),
            page_id: "p1".to_string(),
            parent_id: None,
            content: "Test task".to_string(),
            order: 0.0,
            level: 1,
            marker: None,
            priority: None,
            collapsed: false,
            properties,
            refs: vec![],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            created_by: None,
        }
    }

    // ── matches ───────────────────────────────────────────────────

    #[test]
    fn task_matches_block_with_type_and_known_status() {
        let block = make_block(json!({"type": "task", "status": "todo"}));
        assert!(TaskContract.matches(&block));
    }

    #[test]
    fn task_rejects_block_without_type() {
        let block = make_block(json!({"status": "todo"}));
        assert!(!TaskContract.matches(&block));
    }

    #[test]
    fn task_rejects_block_with_unknown_status() {
        let block = make_block(json!({"type": "task", "status": "maybe"}));
        assert!(!TaskContract.matches(&block));
    }

    #[test]
    fn task_rejects_block_without_status() {
        let block = make_block(json!({"type": "task"}));
        assert!(!TaskContract.matches(&block));
    }

    #[test]
    fn task_rejects_non_task_block() {
        let block = make_block(json!({"type": "paragraph", "status": "done"}));
        assert!(!TaskContract.matches(&block));
    }

    // ── apply (decoration + weight mapping) ───────────────────────

    #[test]
    fn task_apply_emits_task_checkbox_with_status_weight_done() {
        let block = make_block(json!({"type": "task", "status": "done"}));
        let decs = TaskContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].kind, WasmDecorationKind::TaskCheckbox);
        assert_eq!(decs[0].target, "status");
        assert_eq!(decs[0].value, json!("done"));
        assert_eq!(decs[0].weight, 100);
    }

    #[test]
    fn task_apply_status_weight_mapping_table() {
        let cases = vec![
            ("done", 100u8),
            ("cancelled", 80),
            ("in-progress", 60),
            ("waiting", 40),
            ("todo", 20),
            ("other", 10),
        ];
        for (status, expected_weight) in cases {
            let block = make_block(json!({"type": "task", "status": status}));
            let decs = TaskContract.apply(&block);
            assert_eq!(
                decs[0].weight, expected_weight,
                "status={status} should map to weight {expected_weight}"
            );
        }
    }

    // ── identity / metadata ───────────────────────────────────────

    #[test]
    fn task_contract_id_and_priority() {
        assert_eq!(TaskContract.id(), "task");
        assert_eq!(TaskContract.priority(), 100);
    }
}
