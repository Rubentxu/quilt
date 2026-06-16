//! Link projection contract (V1, WASM mirror).
//!
//! Matches blocks where `link::` is set. Produces a `LinkAffordance`
//! decoration with weight 70.
//!
//! **Deviation from server**: uses `IsSet("link")` (URL presence)
//! instead of `IsSet("link-kind")`. The BlockRow needs the URL to
//! render the affordance. See `wasm-projection-contracts/spec.md`
//! for the full rationale.

use crate::projection::resolver::WasmContract;
use crate::projection::view::{WasmDecoration, WasmDecorationKind};
use crate::types::BlockDto;
use serde_json::json;

/// LinkProjection — produces a link-affordance decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct LinkContract;

impl WasmContract for LinkContract {
    fn id(&self) -> &'static str {
        "link"
    }

    fn priority(&self) -> u32 {
        300
    }

    fn matches(&self, block: &BlockDto) -> bool {
        block
            .properties
            .as_object()
            .map_or(false, |p| p.contains_key("link"))
    }

    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration> {
        let link_value = block
            .properties
            .as_object()
            .and_then(|p| p.get("link").cloned())
            .unwrap_or_else(|| json!(""));

        vec![WasmDecoration {
            kind: WasmDecorationKind::LinkAffordance,
            target: "link".to_string(),
            value: link_value,
            weight: 70,
        }]
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
            content: "Test link".to_string(),
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
    fn link_matches_with_url() {
        let block = make_block(json!({"link": "https://example.com"}));
        assert!(LinkContract.matches(&block));
    }

    #[test]
    fn link_matches_with_empty_string() {
        let block = make_block(json!({"link": ""}));
        assert!(LinkContract.matches(&block));
    }

    #[test]
    fn link_rejects_without_link() {
        let block = make_block(json!({}));
        assert!(!LinkContract.matches(&block));
    }

    #[test]
    fn link_apply_emits_link_affordance_weight_70() {
        let block = make_block(json!({"link": "https://example.com"}));
        let decs = LinkContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].kind, WasmDecorationKind::LinkAffordance);
        assert_eq!(decs[0].target, "link");
        assert_eq!(decs[0].value, json!("https://example.com"));
        assert_eq!(decs[0].weight, 70);
    }

    #[test]
    fn link_contract_id_and_priority() {
        assert_eq!(LinkContract.id(), "link");
        assert_eq!(LinkContract.priority(), 300);
    }
}
