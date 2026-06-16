//! Default projection contract (V1, WASM mirror).
//!
//! Universal fallback — matches every block, produces no decorations.
//! Priority is `u32::MAX` so it never wins a tie against a specialized
//! contract.

use crate::projection::resolver::WasmContract;
use crate::projection::view::WasmDecoration;
use crate::types::BlockDto;

/// DefaultProjection — universal fallback (no decorations).
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultContract;

impl WasmContract for DefaultContract {
    fn id(&self) -> &'static str {
        "default"
    }

    fn priority(&self) -> u32 {
        u32::MAX
    }

    fn matches(&self, _block: &BlockDto) -> bool {
        true // wildcard
    }

    fn apply(&self, _block: &BlockDto) -> Vec<WasmDecoration> {
        Vec::new()
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
            content: "Test".to_string(),
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

    #[test]
    fn default_matches_empty_block() {
        let block = make_block(json!({}));
        assert!(DefaultContract.matches(&block));
    }

    #[test]
    fn default_matches_fully_populated_block() {
        let block = make_block(json!({
            "type": "task", "status": "done", "priority": "A", "tags": ["a", "b"]
        }));
        assert!(DefaultContract.matches(&block));
    }

    #[test]
    fn default_matches_adversarial_unicode_block() {
        let block = make_block(json!({"🦀": "rust", "type": "🦀"}));
        assert!(DefaultContract.matches(&block));
    }

    #[test]
    fn default_apply_emits_no_decorations() {
        let block = make_block(json!({"type": "task"}));
        assert!(DefaultContract.apply(&block).is_empty());
    }

    #[test]
    fn default_contract_id_and_priority() {
        assert_eq!(DefaultContract.id(), "default");
        assert_eq!(DefaultContract.priority(), u32::MAX);
    }
}
