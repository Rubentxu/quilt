//! Media projection contract (V1, WASM mirror).
//!
//! Matches blocks where `type:: media` AND `media-type::` is `video`
//! or `image`. Produces a `MediaPreview` decoration with weight 90.
//!
//! Mirrors `quilt_application::services::projection::contracts::media`
//! (slice #4).

use crate::projection::resolver::WasmContract;
use crate::projection::view::{WasmDecoration, WasmDecorationKind};
use crate::types::BlockDto;
use serde_json::json;

/// V1 media types — image and video only (audio is out of V1 scope).
const V1_MEDIA_TYPES: &[&str] = &["video", "image"];

/// MediaProjection — produces a media-preview decoration.
#[derive(Debug, Clone, Copy, Default)]
pub struct MediaContract;

impl WasmContract for MediaContract {
    fn id(&self) -> &'static str {
        "media"
    }

    fn priority(&self) -> u32 {
        200
    }

    fn matches(&self, block: &BlockDto) -> bool {
        let Some(props) = block.properties.as_object() else {
            return false;
        };
        let type_matches = props
            .get("type")
            .map_or(false, |v| v == &json!("media"));
        let media_type_in_set = props
            .get("media-type")
            .and_then(|v| v.as_str())
            .map_or(false, |s| V1_MEDIA_TYPES.contains(&s));
        type_matches && media_type_in_set
    }

    fn apply(&self, block: &BlockDto) -> Vec<WasmDecoration> {
        let media_type = block
            .properties
            .as_object()
            .and_then(|p| p.get("media-type").cloned())
            .unwrap_or_else(|| json!("image"));

        vec![WasmDecoration {
            kind: WasmDecorationKind::MediaPreview,
            target: "media-type".to_string(),
            value: media_type,
            weight: 90,
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
            content: "Test media".to_string(),
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
    fn media_matches_video() {
        let block = make_block(json!({"type": "media", "media-type": "video"}));
        assert!(MediaContract.matches(&block));
    }

    #[test]
    fn media_matches_image() {
        let block = make_block(json!({"type": "media", "media-type": "image"}));
        assert!(MediaContract.matches(&block));
    }

    #[test]
    fn media_rejects_audio() {
        let block = make_block(json!({"type": "media", "media-type": "audio"}));
        assert!(!MediaContract.matches(&block));
    }

    #[test]
    fn media_rejects_non_media_block() {
        let block = make_block(json!({"type": "task", "media-type": "video"}));
        assert!(!MediaContract.matches(&block));
    }

    #[test]
    fn media_rejects_block_without_media_type() {
        let block = make_block(json!({"type": "media"}));
        assert!(!MediaContract.matches(&block));
    }

    #[test]
    fn media_apply_emits_media_preview_weight_90() {
        let block = make_block(json!({"type": "media", "media-type": "video"}));
        let decs = MediaContract.apply(&block);
        assert_eq!(decs.len(), 1);
        assert_eq!(decs[0].kind, WasmDecorationKind::MediaPreview);
        assert_eq!(decs[0].target, "media-type");
        assert_eq!(decs[0].value, json!("video"));
        assert_eq!(decs[0].weight, 90);
    }

    #[test]
    fn media_contract_id_and_priority() {
        assert_eq!(MediaContract.id(), "media");
        assert_eq!(MediaContract.priority(), 200);
    }
}
