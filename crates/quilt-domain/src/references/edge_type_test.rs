//! Tests for EdgeType enum and TypedEdge

use crate::references::{EdgeType, TypedEdge};
use crate::value_objects::Uuid;
use proptest::prelude::*;

/// Round-trip test for unit variants
#[test]
fn test_edge_type_page_ref_roundtrip() {
    let original = EdgeType::PageRef;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_block_ref_roundtrip() {
    let original = EdgeType::BlockRef;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_tag_roundtrip() {
    let original = EdgeType::Tag;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_backlink_roundtrip() {
    let original = EdgeType::Backlink;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_namespace_roundtrip() {
    let original = EdgeType::Namespace;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_parent_roundtrip() {
    let original = EdgeType::Parent;
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_custom_roundtrip() {
    let original = EdgeType::Custom("evidence-link".to_string());
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, parsed);
}

#[test]
fn test_edge_type_custom_preserves_string() {
    let original = EdgeType::Custom("my-custom-type".to_string());
    let json = serde_json::to_string(&original).unwrap();
    let parsed: EdgeType = serde_json::from_str(&json).unwrap();
    match parsed {
        EdgeType::Custom(s) => assert_eq!(s, "my-custom-type"),
        _ => panic!("Expected Custom variant"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypedEdge serde roundtrip + proptest
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_typed_edge_roundtrip() {
    let edge = TypedEdge::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        EdgeType::PageRef,
        0.5,
        1700000000,
    );
    let json = serde_json::to_string(&edge).unwrap();
    let parsed: TypedEdge = serde_json::from_str(&json).unwrap();
    assert_eq!(edge.from, parsed.from);
    assert_eq!(edge.to, parsed.to);
    assert_eq!(edge.edge_type, parsed.edge_type);
    assert!((edge.weight - parsed.weight).abs() < f32::EPSILON);
    assert_eq!(edge.created_at, parsed.created_at);
}

#[test]
fn test_typed_edge_custom_type_roundtrip() {
    let edge = TypedEdge::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        EdgeType::Custom("my-edge".to_string()),
        0.99,
        1700000000,
    );
    let json = serde_json::to_string(&edge).unwrap();
    let parsed: TypedEdge = serde_json::from_str(&json).unwrap();
    assert_eq!(edge.from, parsed.from);
    assert_eq!(edge.to, parsed.to);
    match (&edge.edge_type, &parsed.edge_type) {
        (EdgeType::Custom(a), EdgeType::Custom(b)) => assert_eq!(a, b),
        _ => panic!("Expected Custom edge type"),
    }
}

// Proptest: 100 random TypedEdge instances round-trip correctly
proptest! {
    #[test]
    fn test_typed_edge_proptest_roundtrip(
        from_bytes in any::<[u8; 16]>(),
        to_bytes in any::<[u8; 16]>(),
        edge_type in prop_oneof![
            Just(EdgeType::PageRef),
            Just(EdgeType::BlockRef),
            Just(EdgeType::Tag),
            Just(EdgeType::Backlink),
            Just(EdgeType::Namespace),
            Just(EdgeType::Parent),
            any::<String>().prop_map(EdgeType::Custom),
        ],
        // Generate raw weight in range [-2.0, 2.0] to test clamping
        raw_weight in -2.0f32..2.0f32,
        created_at in 0i64..i64::MAX,
    ) {
        let from = Uuid::from_bytes(from_bytes);
        let to = Uuid::from_bytes(to_bytes);
        let edge = TypedEdge::new(from, to, edge_type.clone(), raw_weight, created_at);

        // Verify weight is always clamped
        prop_assert!(edge.weight >= 0.0 && edge.weight <= 1.0);

        let json = serde_json::to_string(&edge).unwrap();
        let parsed: TypedEdge = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(edge.from, parsed.from);
        prop_assert_eq!(edge.to, parsed.to);
        prop_assert_eq!(edge.edge_type, parsed.edge_type);
        prop_assert!((edge.weight - parsed.weight).abs() < f32::EPSILON);
        prop_assert_eq!(edge.created_at, parsed.created_at);
    }
}
