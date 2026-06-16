//! Heading projection contract (V1, WASM mirror).
//!
//! Matches blocks where `block-role:: heading` AND `heading-level::`
//! is 1, 2, or 3 (tightened from server's `IsSet` — see module-level
//! docs for the rationale).
//!
//! Mirrors `quilt_application::services::projection::contracts::heading`
//! (slice #4).

use crate::projection::resolver::WasmContract;
use crate::projection::view::{WasmDecoration, WasmDecorationKind};
use crate::types::BlockDto;
use serde_json::json;

/// V1 heading levels — the canonical H1–H3 set produced by the
/// V1 canonicalizer. (Server uses `IsSet` which matches any
/// level; the WASM mirror tightens to `IsOneOf(1, 2, 3)`.)
const V1_HEADING_LEVELS: &[i64] = &[1, 2, 3];

/// HeadingProjection — produces a heading-anchor decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadingContract;

impl WasmContract for HeadingContract {
    fn id(&self) -> &'static str {
        "heading"
    }

    fn priority(&self) -> u32 {
        150
    }

    fn matches(&self, block: &BlockDto) -> bool {
        let Some(props) = block.properties.as_object() else {
            return false;
        };
        let role_matches = props
            .get("block-role")
            .map_or(false, |v| v == &json!("heading"));
        let level_in_set = props
            .get("heading-level")
            .and_then(|v| v.as_i64())
            .map_or(false, |n| V1_HEADING_LEVELS.contains(&n));
        role_matches && level_in_set
    }

    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration> {
        let level_value = block
            .properties
            .as_object()
            .and_then(|p| p.get("heading-level").cloned())
            .unwrap_or_else(|| json!(1));

        let weight = match level_value.as_i64() {
            Some(1) => 100,
            Some(2) => 80,
            Some(3) => 60,
            _ => 40,
        };

        vec![WasmDecoration {
            kind: WasmDecorationKind::HeadingAnchor,
            target: "heading-level".to_string(),
            value: level_value,
            weight,
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
            content: "Test heading".to_string(),
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
    fn heading_matches_h1() {
        let block = make_block(json!({"block-role": "heading", "heading-level": 1}));
        assert!(HeadingContract.matches(&block));
    }

    #[test]
    fn heading_matches_h2() {
        let block = make_block(json!({"block-role": "heading", "heading-level": 2}));
        assert!(HeadingContract.matches(&block));
    }

    #[test]
    fn heading_matches_h3() {
        let block = make_block(json!({"block-role": "heading", "heading-level": 3}));
        assert!(HeadingContract.matches(&block));
    }

    #[test]
    fn heading_rejects_h4() {
        let block = make_block(json!({"block-role": "heading", "heading-level": 4}));
        assert!(!HeadingContract.matches(&block));
    }

    #[test]
    fn heading_rejects_non_heading_role() {
        let block = make_block(json!({"block-role": "paragraph", "heading-level": 1}));
        assert!(!HeadingContract.matches(&block));
    }

    #[test]
    fn heading_rejects_block_without_heading_level() {
        let block = make_block(json!({"block-role": "heading"}));
        assert!(!HeadingContract.matches(&block));
    }

    // ── apply (decoration + weight mapping) ───────────────────────

    #[test]
    fn heading_apply_emits_heading_anchor_with_level_weight() {
        let block = make_block(json!({"block-role": "heading", "heading-level": 1}));
        let decs = HeadingContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].kind, WasmDecorationKind::HeadingAnchor);
        assert_eq!(decs[0].target, "heading-level");
        assert_eq!(decs[0].value, json!(1));
        assert_eq!(decs[0].weight, 100);
    }

    #[test]
    fn heading_apply_weight_mapping_table() {
        let cases = vec![(1i64, 100u8), (2, 80), (3, 60), (99, 40)];
        for (level, expected_weight) in cases {
            let block = make_block(json!({"block-role": "heading", "heading-level": level}));
            let decs = HeadingContract.apply(&block);
            assert_eq!(
                decs[0].weight, expected_weight,
                "level={level} should map to weight {expected_weight}"
            );
        }
    }

    // ── identity / metadata ───────────────────────────────────────

    #[test]
    fn heading_contract_id_and_priority() {
        assert_eq!(HeadingContract.id(), "heading");
        assert_eq!(HeadingContract.priority(), 150);
    }
}
