//! Date projection contract (V1, WASM mirror).
//!
//! Matches blocks where `scheduled::` OR `deadline::` is set. Produces
//! a `DateIndicator` decoration — `deadline` is preferred over
//! `scheduled` and has higher weight (95 vs 75).
//!
//! Mirrors `quilt_application::services::projection::contracts::date`
//! (slice #4).

use crate::projection::resolver::WasmContract;
use crate::projection::view::{WasmDecoration, WasmDecorationKind};
use crate::types::BlockDto;

/// DateProjection — produces a date-indicator decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct DateContract;

impl WasmContract for DateContract {
    fn id(&self) -> &'static str {
        "date"
    }

    fn priority(&self) -> u32 {
        250
    }

    fn matches(&self, block: &BlockDto) -> bool {
        let Some(props) = block.properties.as_object() else {
            return false;
        };
        props.contains_key("scheduled") || props.contains_key("deadline")
    }

    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration> {
        let props = block.properties.as_object();

        // Prefer deadline over scheduled — higher weight
        let (target_key, date_value, weight) =
            if let Some(v) = props.and_then(|p| p.get("deadline").cloned()) {
                ("deadline".to_string(), v, 95u8)
            } else if let Some(v) = props.and_then(|p| p.get("scheduled").cloned()) {
                ("scheduled".to_string(), v, 75u8)
            } else {
                // Should not be reached — contract ensures one of these is set.
                return Vec::new();
            };

        vec![WasmDecoration {
            kind: WasmDecorationKind::DateIndicator,
            target: target_key,
            value: date_value,
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
            content: "Test date".to_string(),
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
    fn date_matches_deadline() {
        let block = make_block(json!({"deadline": "2026-12-31T00:00:00Z"}));
        assert!(DateContract.matches(&block));
    }

    #[test]
    fn date_matches_scheduled() {
        let block = make_block(json!({"scheduled": "2026-12-25T00:00:00Z"}));
        assert!(DateContract.matches(&block));
    }

    #[test]
    fn date_matches_both() {
        let block = make_block(json!({
            "deadline": "2026-12-31T00:00:00Z",
            "scheduled": "2026-12-25T00:00:00Z"
        }));
        assert!(DateContract.matches(&block));
    }

    #[test]
    fn date_rejects_neither() {
        let block = make_block(json!({}));
        assert!(!DateContract.matches(&block));
    }

    #[test]
    fn date_apply_deadline_weight_95() {
        let block = make_block(json!({"deadline": "2026-12-31T00:00:00Z"}));
        let decs = DateContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].kind, WasmDecorationKind::DateIndicator);
        assert_eq!(decs[0].target, "deadline");
        assert_eq!(decs[0].weight, 95);
    }

    #[test]
    fn date_apply_scheduled_weight_75() {
        let block = make_block(json!({"scheduled": "2026-12-25T00:00:00Z"}));
        let decs = DateContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].target, "scheduled");
        assert_eq!(decs[0].weight, 75);
    }

    #[test]
    fn date_apply_prefers_deadline_over_scheduled() {
        let block = make_block(json!({
            "deadline": "2026-12-31T00:00:00Z",
            "scheduled": "2026-12-25T00:00:00Z"
        }));
        let decs = DateContract.apply(&block);
        assert_eq!(decs[0].target, "deadline");
        assert_eq!(decs[0].weight, 95);
    }

    #[test]
    fn date_contract_id_and_priority() {
        assert_eq!(DateContract.id(), "date");
        assert_eq!(DateContract.priority(), 250);
    }
}
