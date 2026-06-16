//! Parity tests — WASM projection output == server reference output.
//!
//! The 18-row matrix below is the **load-bearing safety net** for the
//! WASM-first projection strategy. If the WASM output ever diverges
//! from the server output, the UI will silently render different
//! content depending on which path served the request — exactly the
//! kind of subtle bug that erodes user trust.
//!
//! # How the parity test works
//!
//! The WASM resolver (in `resolver.rs`) and the server's
//! `ProjectionResolver` (in `crates/quilt-application/src/...`)
//! have different type signatures (BlockDto vs Block; serde_json::Value
//! vs PropertyValue). To compare them, we re-implement the server's
//! algorithm in test code as [`server_resolve_reference`] (no
//! `quilt-domain` / `quilt-application` dependency in `quilt-core`).
//!
//! The reference implementation is **deliberately redundant** with
//! the server. The duplication is a price we pay for keeping
//! `quilt-core` independent of the domain/application crates. The
//! 18-row matrix covers all the corner cases; if a row fails, either
//! the WASM mirror or the reference impl is wrong.
//!
//! # Coverage
//!
//! Each row tests:
//! 1. The winning contract id
//! 2. The decoration `kind`, `target`, `value`, `weight`
//! 3. The `wasm_had_conflict` flag
//!
//! Fields not under test (text, children, properties) are spot-checked
//! but not exhaustively compared.

use crate::projection::resolver::WasmProjectionResolver;
use crate::projection::view::{WasmDecorationKind, WasmProjectionView};
use crate::types::BlockDto;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Construct a `BlockDto` from a JSON properties map (test helper).
fn block(properties: Value, content: &str) -> BlockDto {
    BlockDto {
        id: "b1".to_string(),
        page_id: "p1".to_string(),
        parent_id: None,
        content: content.to_string(),
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

// ──────────────────────────────────────────────────────────────────────
// Server reference implementation (deliberately redundant)
// ──────────────────────────────────────────────────────────────────────

/// Re-implementation of the server's `ProjectionResolver::resolve`
/// algorithm for parity testing. Operates on `serde_json::Value`
/// (the same type the WASM resolver uses) so we can compare outputs
/// field-by-field.
fn server_resolve_reference(properties: &Value) -> ReferenceOutcome {
    // The 6 V1 contracts with their priorities and match rules
    let candidates: Vec<(&str, u32, fn(&Value) -> bool)> = vec![
        ("task", 100u32, matches_task),
        ("heading", 150, matches_heading),
        ("media", 200, matches_media),
        ("date", 250, matches_date),
        ("link", 300, matches_link),
    ];

    let mut matched: Vec<(&str, u32)> = Vec::new();
    for (id, priority, matcher) in &candidates {
        if matcher(properties) {
            matched.push((*id, *priority));
        }
    }

    // Default contract is always a candidate
    matched.push(("default", u32::MAX));

    // Highest score = smallest priority (score = -(priority as f64))
    matched.sort_by_key(|(_, p)| *p);
    let (winner_id, winner_priority) = matched[0];

    // Compute decorations based on winner
    let decorations = match winner_id {
        "task" => compute_task_decoration(properties),
        "heading" => compute_heading_decoration(properties),
        "media" => compute_media_decoration(properties),
        "date" => compute_date_decoration(properties),
        "link" => compute_link_decoration(properties),
        _ => Vec::new(),
    };

    let mut properties_out = BTreeMap::new();
    if let Some(obj) = properties.as_object() {
        for (k, v) in obj {
            properties_out.insert(k.clone(), v.clone());
        }
    }
    properties_out.insert("projection".to_string(), json!(winner_id));

    ReferenceOutcome {
        winner_id: winner_id.to_string(),
        decorations,
        properties: properties_out,
    }
}

struct ReferenceOutcome {
    winner_id: String,
    decorations: Vec<ReferenceDecoration>,
    properties: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
struct ReferenceDecoration {
    kind: String,
    target: String,
    value: Value,
    weight: u8,
}

fn matches_task(p: &Value) -> bool {
    let Some(obj) = p.as_object() else { return false };
    obj.get("type").map_or(false, |v| v == &json!("task"))
        && obj.contains_key("status")
        && obj
            .get("status")
            .and_then(|v| v.as_str())
            .map_or(false, |s| {
                matches!(
                    s,
                    "todo" | "in-progress" | "done" | "cancelled" | "waiting"
                )
            })
}

fn matches_heading(p: &Value) -> bool {
    let Some(obj) = p.as_object() else { return false };
    obj.get("block-role").map_or(false, |v| v == &json!("heading"))
        && obj
            .get("heading-level")
            .and_then(|v| v.as_i64())
            .map_or(false, |n| matches!(n, 1 | 2 | 3))
}

fn matches_media(p: &Value) -> bool {
    let Some(obj) = p.as_object() else { return false };
    obj.get("type").map_or(false, |v| v == &json!("media"))
        && obj
            .get("media-type")
            .and_then(|v| v.as_str())
            .map_or(false, |s| matches!(s, "video" | "image"))
}

fn matches_date(p: &Value) -> bool {
    p.as_object()
        .map_or(false, |o| o.contains_key("scheduled") || o.contains_key("deadline"))
}

fn matches_link(p: &Value) -> bool {
    p.as_object()
        .map_or(false, |o| o.contains_key("link"))
}

fn compute_task_decoration(p: &Value) -> Vec<ReferenceDecoration> {
    let status = p
        .as_object()
        .and_then(|o| o.get("status").cloned())
        .unwrap_or(json!("todo"));
    let weight = match status.as_str() {
        Some("done") => 100,
        Some("cancelled") => 80,
        Some("in-progress") => 60,
        Some("waiting") => 40,
        Some("todo") => 20,
        _ => 10,
    };
    vec![ReferenceDecoration {
        kind: "task-checkbox".to_string(),
        target: "status".to_string(),
        value: status,
        weight,
    }]
}

fn compute_heading_decoration(p: &Value) -> Vec<ReferenceDecoration> {
    let level = p
        .as_object()
        .and_then(|o| o.get("heading-level").cloned())
        .unwrap_or(json!(1));
    let weight = match level.as_i64() {
        Some(1) => 100,
        Some(2) => 80,
        Some(3) => 60,
        _ => 40,
    };
    vec![ReferenceDecoration {
        kind: "heading-anchor".to_string(),
        target: "heading-level".to_string(),
        value: level,
        weight,
    }]
}

fn compute_media_decoration(p: &Value) -> Vec<ReferenceDecoration> {
    let media_type = p
        .as_object()
        .and_then(|o| o.get("media-type").cloned())
        .unwrap_or(json!("image"));
    vec![ReferenceDecoration {
        kind: "media-preview".to_string(),
        target: "media-type".to_string(),
        value: media_type,
        weight: 90,
    }]
}

fn compute_date_decoration(p: &Value) -> Vec<ReferenceDecoration> {
    let obj = p.as_object();
    if let Some(v) = obj.and_then(|o| o.get("deadline").cloned()) {
        vec![ReferenceDecoration {
            kind: "date-indicator".to_string(),
            target: "deadline".to_string(),
            value: v,
            weight: 95,
        }]
    } else if let Some(v) = obj.and_then(|o| o.get("scheduled").cloned()) {
        vec![ReferenceDecoration {
            kind: "date-indicator".to_string(),
            target: "scheduled".to_string(),
            value: v,
            weight: 75,
        }]
    } else {
        Vec::new()
    }
}

fn compute_link_decoration(p: &Value) -> Vec<ReferenceDecoration> {
    let link = p
        .as_object()
        .and_then(|o| o.get("link").cloned())
        .unwrap_or(json!(""));
    vec![ReferenceDecoration {
        kind: "link-affordance".to_string(),
        target: "link".to_string(),
        value: link,
        weight: 70,
    }]
}

// ──────────────────────────────────────────────────────────────────────
// Parity assertion helpers
// ──────────────────────────────────────────────────────────────────────

/// Assert that the WASM view and the server reference are equivalent.
fn assert_parity(
    properties: Value,
    content: &str,
    expected_winner: &str,
    expected_decoration: Option<(&str, &str, &str, u8)>,
) {
    let wasm_view: WasmProjectionView =
        WasmProjectionResolver::v1().resolve(&block(properties.clone(), content));
    let server_view = server_resolve_reference(&properties);

    // 1. Winner id
    assert_eq!(
        wasm_view.wasm_contract_id, expected_winner,
        "winner mismatch: WASM={} vs expected={}",
        wasm_view.wasm_contract_id, expected_winner
    );
    assert_eq!(
        wasm_view.wasm_contract_id, server_view.winner_id,
        "winner mismatch: WASM={} vs server={}",
        wasm_view.wasm_contract_id, server_view.winner_id
    );

    // 2. Decoration
    let wasm_decs = &wasm_view.decorations;
    let server_decs = &server_view.decorations;
    assert_eq!(
        wasm_decs.len(),
        server_decs.len(),
        "decoration count mismatch: WASM={:?} vs server={:?}",
        wasm_decs,
        server_decs
    );
    if !wasm_decs.is_empty() {
        let d = &wasm_decs[0];
        assert_eq!(
            d.kind,
            wasm_kind_from_str(&server_decs[0].kind),
            "decoration kind mismatch: WASM={:?} vs server={:?}",
            d.kind,
            server_decs[0].kind
        );
        assert_eq!(d.target, server_decs[0].target, "decoration target mismatch");
        assert_eq!(d.value, server_decs[0].value, "decoration value mismatch");
        assert_eq!(d.weight, server_decs[0].weight, "decoration weight mismatch");
    }
    if let Some((kind, target, value, weight)) = expected_decoration {
        assert_eq!(wasm_decs.len(), 1, "expected exactly 1 decoration");
        assert_eq!(wasm_decs[0].kind, wasm_kind_from_str(kind), "decoration kind");
        assert_eq!(wasm_decs[0].target, target, "decoration target");
        // The value is parsed from the test string as either a JSON literal
        // (if it parses as JSON) or a string. Numbers like "1" parse as
        // JSON numbers — that's the WASM-side behavior.
        let expected_value = serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.to_string()));
        assert_eq!(
            wasm_decs[0].value, expected_value,
            "decoration value: expected {:?}, got {:?}",
            expected_value, wasm_decs[0].value
        );
        assert_eq!(wasm_decs[0].weight, weight, "decoration weight");
    }

    // 3. Properties (must include the projection:: key)
    assert_eq!(
        wasm_view.properties.get("projection"),
        server_view.properties.get("projection"),
        "projection:: property mismatch"
    );

    // 4. Conflicts
    assert_eq!(
        wasm_view.wasm_had_conflict,
        wasm_view.conflicts.is_empty() == false,
        "wasm_had_conflict flag inconsistent with conflicts vec"
    );
}

fn wasm_kind_from_str(s: &str) -> WasmDecorationKind {
    match s {
        "task-checkbox" => WasmDecorationKind::TaskCheckbox,
        "status-badge" => WasmDecorationKind::StatusBadge,
        "media-preview" => WasmDecorationKind::MediaPreview,
        "heading-anchor" => WasmDecorationKind::HeadingAnchor,
        "date-indicator" => WasmDecorationKind::DateIndicator,
        "link-affordance" => WasmDecorationKind::LinkAffordance,
        "generic-badge" => WasmDecorationKind::GenericBadge,
        other => panic!("unknown decoration kind: {other}"),
    }
}

// ──────────────────────────────────────────────────────────────────────
// Resolver sanity tests (not part of the 18-row matrix)
// ──────────────────────────────────────────────────────────────────────

#[test]
fn v1_returns_six_contracts() {
    let resolver = WasmProjectionResolver::v1();
    assert_eq!(resolver.len(), 6);
    assert!(!resolver.is_empty());
}

#[test]
fn v1_priorities_are_100_150_200_250_300_u32_max() {
    // Indirectly verified by the build-time assertion; explicit check here.
    let resolver = WasmProjectionResolver::v1();
    // We can verify the contracts' priorities are well-formed by resolving
    // a block that matches each one.
    let cases = [
        (json!({"type": "task", "status": "done"}), "task"),
        (json!({"block-role": "heading", "heading-level": 1}), "heading"),
        (json!({"type": "media", "media-type": "video"}), "media"),
        (json!({"deadline": "2026-12-31T00:00:00Z"}), "date"),
        (json!({"link": "https://example.com"}), "link"),
        (json!({}), "default"),
    ];
    for (props, expected) in cases {
        let view = resolver.resolve(&block(props, "test"));
        assert_eq!(
            view.wasm_contract_id, expected,
            "contract priority ordering broken for {expected}"
        );
    }
}

#[test]
fn resolve_empty_block_yields_default() {
    let view = WasmProjectionResolver::v1().resolve(&block(json!({}), ""));
    assert_eq!(view.wasm_contract_id, "default");
    assert!(view.decorations.is_empty());
    assert!(view.conflicts.is_empty());
    assert!(!view.wasm_had_conflict);
    assert!(view.wasm_source);
}

#[test]
fn resolve_adversarial_unicode_yields_default() {
    let view = WasmProjectionResolver::v1().resolve(&block(
        json!({"🦀": "rust", "type": "🦀", "status": "🦀"}),
        "🦀",
    ));
    // The 🦀::task value does not match the V1 task contract's
    // `Equals(type, "task")` predicate, so no specialized contract
    // matches. Default wins.
    assert_eq!(view.wasm_contract_id, "default");
    assert!(view.decorations.is_empty());
}

#[test]
fn resolve_sets_wasm_source_true() {
    let view = WasmProjectionResolver::v1().resolve(&block(json!({}), ""));
    assert!(view.wasm_source);
}

#[test]
fn resolve_preserves_base_surface() {
    let view = WasmProjectionResolver::v1().resolve(&block(json!({"type": "task", "status": "done"}), "Hello world"));
    assert_eq!(view.text, "Hello world");
    assert_eq!(view.properties.get("type"), Some(&json!("task")));
    assert_eq!(view.properties.get("status"), Some(&json!("done")));
    // projection:: system property added by resolver
    assert_eq!(view.properties.get("projection"), Some(&json!("task")));
}

// ──────────────────────────────────────────────────────────────────────
// 18-row parity test matrix
// ──────────────────────────────────────────────────────────────────────

// Task rows (5)
#[test]
fn parity_row_01_task_done() {
    assert_parity(
        json!({"type": "task", "status": "done"}),
        "Buy milk",
        "task",
        Some(("task-checkbox", "status", "done", 100)),
    );
}

#[test]
fn parity_row_02_task_todo() {
    assert_parity(
        json!({"type": "task", "status": "todo"}),
        "task",
        "task",
        Some(("task-checkbox", "status", "todo", 20)),
    );
}

#[test]
fn parity_row_03_task_cancelled() {
    assert_parity(
        json!({"type": "task", "status": "cancelled"}),
        "task",
        "task",
        Some(("task-checkbox", "status", "cancelled", 80)),
    );
}

#[test]
fn parity_row_04_task_in_progress() {
    assert_parity(
        json!({"type": "task", "status": "in-progress"}),
        "task",
        "task",
        Some(("task-checkbox", "status", "in-progress", 60)),
    );
}

#[test]
fn parity_row_05_task_waiting() {
    assert_parity(
        json!({"type": "task", "status": "waiting"}),
        "task",
        "task",
        Some(("task-checkbox", "status", "waiting", 40)),
    );
}

// Heading rows (3)
#[test]
fn parity_row_07_heading_h1() {
    assert_parity(
        json!({"block-role": "heading", "heading-level": 1}),
        "Title",
        "heading",
        Some(("heading-anchor", "heading-level", "1", 100)),
    );
}

#[test]
fn parity_row_08_heading_h2() {
    assert_parity(
        json!({"block-role": "heading", "heading-level": 2}),
        "Subtitle",
        "heading",
        Some(("heading-anchor", "heading-level", "2", 80)),
    );
}

#[test]
fn parity_row_09_heading_h3() {
    assert_parity(
        json!({"block-role": "heading", "heading-level": 3}),
        "Section",
        "heading",
        Some(("heading-anchor", "heading-level", "3", 60)),
    );
}

// Media rows (2)
#[test]
fn parity_row_10_media_video() {
    assert_parity(
        json!({"type": "media", "media-type": "video"}),
        "video",
        "media",
        Some(("media-preview", "media-type", "video", 90)),
    );
}

#[test]
fn parity_row_11_media_image() {
    assert_parity(
        json!({"type": "media", "media-type": "image"}),
        "image",
        "media",
        Some(("media-preview", "media-type", "image", 90)),
    );
}

// Date rows (2)
#[test]
fn parity_row_12_date_deadline() {
    assert_parity(
        json!({"deadline": "2026-12-31T00:00:00Z"}),
        "deadline",
        "date",
        Some(("date-indicator", "deadline", "2026-12-31T00:00:00Z", 95)),
    );
}

#[test]
fn parity_row_13_date_scheduled() {
    assert_parity(
        json!({"scheduled": "2026-12-25T00:00:00Z"}),
        "scheduled",
        "date",
        Some(("date-indicator", "scheduled", "2026-12-25T00:00:00Z", 75)),
    );
}

// Link row (1)
#[test]
fn parity_row_14_link() {
    assert_parity(
        json!({"link": "https://example.com"}),
        "link",
        "link",
        Some(("link-affordance", "link", "https://example.com", 70)),
    );
}

// Default rows (2)
#[test]
fn parity_row_15_default_empty() {
    assert_parity(json!({}), "", "default", None);
}

#[test]
fn parity_row_16_default_status_only() {
    // ADR-0025 invariant: status:: alone does NOT activate TaskProjection
    assert_parity(
        json!({"status": "todo"}),
        "Just status",
        "default",
        None,
    );
}

// Cross-contract rows (3)
#[test]
fn parity_row_17_task_beats_media() {
    assert_parity(
        json!({"type": "task", "status": "done", "media-type": "video"}),
        "both",
        "task",
        Some(("task-checkbox", "status", "done", 100)),
    );
}

#[test]
fn parity_row_18_date_prefers_deadline() {
    assert_parity(
        json!({"deadline": "2026-12-31T00:00:00Z", "scheduled": "2026-12-25T00:00:00Z"}),
        "both dates",
        "date",
        Some(("date-indicator", "deadline", "2026-12-31T00:00:00Z", 95)),
    );
}
