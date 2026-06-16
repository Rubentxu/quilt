//! WASM projection view — composable block visualization surface (port).
//!
//! Mirrors `quilt_domain::projection::view` (slice #4) but operates on
//! `serde_json::Value` instead of `PropertyValue`. The view is the
//! JSON wire contract with the React UI: every field is serializable.
//!
//! # Base Block Surface
//!
//! Every block always starts from its **Base Block Surface**: the raw
//! content, links, children, and properties already present on the block.
//! Decorations from active contracts are **composed** on top of this
//! base — they never replace it.
//!
//! # WASM-specific metadata
//!
//! The view carries three WASM-specific fields (`wasm_source`,
//! `wasm_contract_id`, `wasm_had_conflict`) that the server path does
//! not emit. The UI hook adds them to enable telemetry and to prove
//! to debug consumers that the WASM path produced the view.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Kind of link — determines how the UI renders the link affordance.
///
/// Mirrors the server's `LinkKind` enum (slice #4) but serializes as
/// kebab-case lowercase (matching the `link-kind::` property values
/// produced by the V1 canonicalizer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WasmLinkKind {
    /// External URL (web link)
    External,
    /// Media asset (image, video, audio)
    Media,
    /// Reference to another page
    PageRef,
    /// Reference to another block
    BlockRef,
}

impl Default for WasmLinkKind {
    fn default() -> Self {
        WasmLinkKind::External
    }
}

/// A link extracted or derived from a block property.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WasmLinkView {
    /// URL or identifier of the link.
    pub url: String,
    /// Human-readable label (may be empty).
    pub label: String,
    /// Kind of link.
    #[serde(default)]
    pub kind: WasmLinkKind,
}

/// Kind of decoration — visual annotation applied by a projection contract.
///
/// Mirrors the server's `DecorationKind` enum (slice #4). Serializes as
/// kebab-case lowercase (e.g., `task-checkbox`, `media-preview`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WasmDecorationKind {
    /// Task checkbox (from `status` property)
    TaskCheckbox,
    /// Status badge (colored label)
    StatusBadge,
    /// Media embed preview (thumbnail, play button, etc.)
    MediaPreview,
    /// Heading anchor (e.g., `#` with level number)
    HeadingAnchor,
    /// Date indicator (scheduled, deadline, etc.)
    DateIndicator,
    /// Link affordance (external link icon)
    LinkAffordance,
    /// Generic badge (custom label + color)
    GenericBadge,
}

/// A visual decoration produced by a projection contract.
///
/// Decorations are additive — multiple contracts can each add their
/// own decorations. The UI decides how to render them based on `kind`
/// and `weight`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmDecoration {
    /// What kind of decoration this is.
    pub kind: WasmDecorationKind,
    /// Property key this decoration targets (e.g., `"status"`, `"deadline"`).
    pub target: String,
    /// The property value driving this decoration.
    pub value: serde_json::Value,
    /// Higher weight = rendered more prominently. Range 0–255.
    pub weight: u8,
}

/// A conflict arising from the projection resolution algorithm.
///
/// Mirrors the server's `ProjectionConflict` (slice #4) but uses
/// `String` for ids and `String` for block_id (the WASM layer does
/// not have access to `Uuid` from `quilt-domain`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmProjectionConflict {
    /// Human-readable reason for the conflict.
    pub reason: String,
    /// IDs of all contracts that tied in score/priority.
    #[serde(default)]
    pub candidates: Vec<String>,
    /// The winning contract ID, if one could be determined.
    #[serde(default)]
    pub winner: Option<String>,
    /// The block ID this conflict pertains to (as string).
    pub block_id: String,
}

/// The complete visual projection of a block.
///
/// Produced by [`resolver::WasmProjectionResolver::resolve`]. The shape
/// is byte-equal to the server's `ProjectionView` for the first six
/// fields; the last three fields are WASM-specific metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WasmProjectionView {
    /// Raw text content from the block.
    pub text: String,
    /// Links extracted or derived from block properties.
    #[serde(default)]
    pub links: Vec<WasmLinkView>,
    /// Child block IDs (preserved in order).
    #[serde(default)]
    pub children: Vec<String>,
    /// Visual decorations from active contracts.
    #[serde(default)]
    pub decorations: Vec<WasmDecoration>,
    /// Conflicts from ambiguous resolution (empty when unambiguous).
    #[serde(default)]
    pub conflicts: Vec<WasmProjectionConflict>,
    /// Effective properties for the view (base + derived).
    /// BTreeMap is used for deterministic iteration order.
    #[serde(default)]
    pub properties: BTreeMap<String, serde_json::Value>,
    /// WASM-specific: always `true` (proves the WASM path produced the view).
    pub wasm_source: bool,
    /// WASM-specific: winning contract id (e.g., `"task"`, `"default"`).
    pub wasm_contract_id: String,
    /// WASM-specific: `true` if a tied-score conflict was detected.
    pub wasm_had_conflict: bool,
}

impl WasmProjectionView {
    /// Construct a default (empty) view for the given block.
    ///
    /// Used by the resolver's fallback paths (no match, internal error,
    /// panicked contract).
    pub fn default_for_block(_block_id: &str) -> Self {
        Self {
            text: String::new(),
            links: Vec::new(),
            children: Vec::new(),
            decorations: Vec::new(),
            conflicts: Vec::new(),
            properties: BTreeMap::new(),
            wasm_source: true,
            wasm_contract_id: "default".to_string(),
            wasm_had_conflict: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── LinkKind ──────────────────────────────────────────────────

    #[test]
    fn link_kind_default_is_external() {
        assert_eq!(WasmLinkKind::default(), WasmLinkKind::External);
    }

    #[test]
    fn link_kind_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&WasmLinkKind::PageRef).unwrap(),
            "\"page-ref\""
        );
        assert_eq!(
            serde_json::to_string(&WasmLinkKind::BlockRef).unwrap(),
            "\"block-ref\""
        );
        assert_eq!(
            serde_json::to_string(&WasmLinkKind::External).unwrap(),
            "\"external\""
        );
        assert_eq!(
            serde_json::to_string(&WasmLinkKind::Media).unwrap(),
            "\"media\""
        );
    }

    #[test]
    fn link_kind_rejects_unknown_on_deserialize() {
        let result: Result<WasmLinkKind, _> = serde_json::from_str("\"made-up\"");
        assert!(result.is_err(), "Unknown link kind should be rejected");
    }

    // ── DecorationKind ───────────────────────────────────────────

    #[test]
    fn decoration_kind_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&WasmDecorationKind::TaskCheckbox).unwrap(),
            "\"task-checkbox\""
        );
        assert_eq!(
            serde_json::to_string(&WasmDecorationKind::MediaPreview).unwrap(),
            "\"media-preview\""
        );
        assert_eq!(
            serde_json::to_string(&WasmDecorationKind::DateIndicator).unwrap(),
            "\"date-indicator\""
        );
        assert_eq!(
            serde_json::to_string(&WasmDecorationKind::LinkAffordance).unwrap(),
            "\"link-affordance\""
        );
    }

    // ── Decoration ───────────────────────────────────────────────

    #[test]
    fn decoration_value_preserves_json_types() {
        let cases = vec![
            (json!("hello"), "\"hello\""),
            (json!(42), "42"),
            (json!(true), "true"),
            (json!(null), "null"),
        ];
        for (value, expected_substr) in cases {
            let dec = WasmDecoration {
                kind: WasmDecorationKind::TaskCheckbox,
                target: "status".to_string(),
                value: value.clone(),
                weight: 100,
            };
            let json = serde_json::to_string(&dec).unwrap();
            assert!(
                json.contains(&format!("\"value\":{}", expected_substr)),
                "value type {:?} should serialize as {} in {}",
                value,
                expected_substr,
                json
            );
        }
    }

    // ── WasmProjectionView ───────────────────────────────────────

    #[test]
    fn view_default_for_block_has_wasm_source_true() {
        let v = WasmProjectionView::default_for_block("b1");
        assert!(v.wasm_source);
        assert_eq!(v.wasm_contract_id, "default");
        assert!(!v.wasm_had_conflict);
        assert!(v.decorations.is_empty());
        assert!(v.conflicts.is_empty());
        assert!(v.properties.is_empty());
        assert_eq!(v.text, "");
    }

    #[test]
    fn view_serializes_losslessly() {
        let mut properties = BTreeMap::new();
        properties.insert("type".to_string(), json!("task"));
        properties.insert("status".to_string(), json!("done"));
        let view = WasmProjectionView {
            text: "Hello".to_string(),
            links: vec![WasmLinkView {
                url: "https://x.com".to_string(),
                label: "x".to_string(),
                kind: WasmLinkKind::External,
            }],
            children: vec!["b2".to_string()],
            decorations: vec![WasmDecoration {
                kind: WasmDecorationKind::TaskCheckbox,
                target: "status".to_string(),
                value: json!("done"),
                weight: 100,
            }],
            conflicts: vec![],
            properties,
            wasm_source: true,
            wasm_contract_id: "task".to_string(),
            wasm_had_conflict: false,
        };
        let json_str = serde_json::to_string(&view).unwrap();
        let parsed: WasmProjectionView = serde_json::from_str(&json_str).unwrap();
        assert_eq!(view, parsed);
    }

    #[test]
    fn view_properties_use_btree_for_deterministic_iteration() {
        let mut properties = BTreeMap::new();
        properties.insert("z".to_string(), json!("z"));
        properties.insert("a".to_string(), json!("a"));
        properties.insert("m".to_string(), json!("m"));
        let view = WasmProjectionView {
            text: String::new(),
            links: Vec::new(),
            children: Vec::new(),
            decorations: Vec::new(),
            conflicts: Vec::new(),
            properties,
            wasm_source: true,
            wasm_contract_id: "default".to_string(),
            wasm_had_conflict: false,
        };
        let json_str = serde_json::to_string(&view).unwrap();
        // BTreeMap iterates in sorted key order, so the JSON output
        // has "a" before "m" before "z" regardless of insertion order.
        let a_pos = json_str.find("\"a\"").unwrap();
        let m_pos = json_str.find("\"m\"").unwrap();
        let z_pos = json_str.find("\"z\"").unwrap();
        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }

    // ── LinkView ─────────────────────────────────────────────────

    #[test]
    fn link_view_serializes_with_kebab_kind() {
        let link = WasmLinkView {
            url: "[[my-page]]".to_string(),
            label: "My Page".to_string(),
            kind: WasmLinkKind::PageRef,
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("\"kind\":\"page-ref\""), "got: {json}");
    }
}
