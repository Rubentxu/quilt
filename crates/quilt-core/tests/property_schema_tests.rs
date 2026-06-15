//! WASM parity tests for ADR-0025 property configuration types.
//!
//! Verifies that `quilt-core` (WASM-compiled) serializes property types
//! with the same JSON wire format as `quilt-domain` (native).
//!
//! T8 — ADR-0025 WASM parity: byte-equal JSON for all 4 new enums and
//! the extended PropertyDefinition struct.

use quilt_core::schema::properties::{
    DerivedSource, MergePolicy, PropertyDefinition, PropertyMutability, PropertyType,
    PropertyVisibility, ViewContext,
};
use serde_json::json;

// ── PropertyVisibility serde ─────────────────────────────────────────

#[test]
fn test_property_visibility_serde_roundtrip() {
    for variant in [
        PropertyVisibility::Inline,
        PropertyVisibility::Panel,
        PropertyVisibility::System,
        PropertyVisibility::Hidden,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let restored: PropertyVisibility = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, restored, "roundtrip failed for {:?}", variant);
    }
}

#[test]
fn test_property_visibility_lowercase() {
    let json = serde_json::to_string(&PropertyVisibility::Panel).unwrap();
    assert_eq!(json, "\"panel\"");

    let json = serde_json::to_string(&PropertyVisibility::System).unwrap();
    assert_eq!(json, "\"system\"");
}

#[test]
fn test_property_visibility_default_is_inline() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "test", "Test", PropertyType::Text);
    assert_eq!(def.visibility, PropertyVisibility::Inline);
}

// ── PropertyMutability serde ────────────────────────────────────────

#[test]
fn test_property_mutability_serde_roundtrip() {
    for variant in [PropertyMutability::Mutable, PropertyMutability::Immutable] {
        let json = serde_json::to_string(&variant).unwrap();
        let restored: PropertyMutability = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, restored);
    }
}

#[test]
fn test_property_mutability_from_read_only() {
    assert_eq!(
        PropertyMutability::from_read_only(false),
        PropertyMutability::Mutable
    );
    assert_eq!(
        PropertyMutability::from_read_only(true),
        PropertyMutability::Immutable
    );
    assert!(!PropertyMutability::Mutable.to_read_only());
    assert!(PropertyMutability::Immutable.to_read_only());
}

// ── DerivedSource serde ──────────────────────────────────────────────

#[test]
fn test_derived_source_unit_variants() {
    for variant in [
        DerivedSource::BlockContent,
        DerivedSource::Markdown,
        DerivedSource::Canonicalization,
        DerivedSource::Importer,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let restored: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, restored);
    }
}

#[test]
fn test_derived_source_other_property() {
    let variant = DerivedSource::OtherProperty("foo.bar".into());
    let json = serde_json::to_string(&variant).unwrap();
    let restored: DerivedSource = serde_json::from_str(&json).unwrap();
    assert_eq!(variant, restored);

    // Wire format must be {"type":"other_property","value":"foo.bar"}
    assert!(json.contains("\"type\":\"other_property\""));
    assert!(json.contains("\"value\":\"foo.bar\""));
}

// ── MergePolicy serde ───────────────────────────────────────────────

#[test]
fn test_merge_policy_serde_roundtrip() {
    for variant in [
        MergePolicy::SetIfMissing,
        MergePolicy::Overwrite,
        MergePolicy::Append,
        MergePolicy::Union,
        MergePolicy::RejectOnConflict,
        MergePolicy::AskOnConflict,
    ] {
        let json = serde_json::to_string(&variant).unwrap();
        let restored: MergePolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, restored);
    }
}

#[test]
fn test_merge_policy_adr_0025_v1_table() {
    // Verify the ADR-0025 V1 table is populated
    let table = MergePolicy::ADR_0025_V1_TABLE;
    assert!(!table.is_empty());
    // tags → Union
    assert_eq!(
        table.iter().find(|(k, _)| *k == "tags").map(|(_, v)| *v),
        Some(MergePolicy::Union)
    );
}

// ── PropertyDefinition extended fields ───────────────────────────────

#[test]
fn test_property_definition_all_adr_0025_fields_default() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "test", "Test", PropertyType::Text);

    assert_eq!(def.visibility, PropertyVisibility::Inline);
    assert_eq!(def.mutability, PropertyMutability::Mutable);
    assert_eq!(def.derived_from, None);
    assert_eq!(def.merge_policy, MergePolicy::SetIfMissing);
}

#[test]
fn test_property_definition_with_visibility_builder() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "t", "T", PropertyType::Text)
        .with_visibility(PropertyVisibility::Panel);
    assert_eq!(def.visibility, PropertyVisibility::Panel);
}

#[test]
fn test_property_definition_with_mutability_builder() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "t", "T", PropertyType::Text)
        .with_mutability(PropertyMutability::Immutable);
    assert_eq!(def.mutability, PropertyMutability::Immutable);
}

#[test]
fn test_property_definition_with_derived_from_sets_immutable() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "t", "T", PropertyType::Text)
        .with_derived_from(DerivedSource::Markdown);
    assert_eq!(def.derived_from, Some(DerivedSource::Markdown));
    assert_eq!(def.mutability, PropertyMutability::Immutable); // invariant enforced at builder level
}

#[test]
fn test_property_definition_with_merge_policy() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "t", "T", PropertyType::Text)
        .with_merge_policy(MergePolicy::Union);
    assert_eq!(def.merge_policy, MergePolicy::Union);
}

#[test]
fn test_property_definition_is_queryable() {
    let base = PropertyDefinition::new(uuid::Uuid::nil(), "t", "T", PropertyType::Text);

    // Inline and Panel are queryable; System is not; Hidden IS queryable
    assert!(
        base.clone()
            .with_visibility(PropertyVisibility::Inline)
            .is_queryable()
    );
    assert!(
        base.clone()
            .with_visibility(PropertyVisibility::Panel)
            .is_queryable()
    );
    assert!(
        base.clone()
            .with_visibility(PropertyVisibility::Hidden)
            .is_queryable()
    );
    assert!(
        !base
            .clone()
            .with_visibility(PropertyVisibility::System)
            .is_queryable()
    );
}

#[test]
fn test_property_definition_from_legacy_fields() {
    let def = PropertyDefinition::from_legacy_fields(
        uuid::Uuid::nil(),
        "test",
        "Test",
        PropertyType::Text,
        ViewContext::Page,
        true,
        true,
        false,
        false,
    );

    assert_eq!(def.visibility, PropertyVisibility::Panel);
    assert_eq!(def.mutability, PropertyMutability::Mutable);
    assert_eq!(def.derived_from, None);
    assert_eq!(def.merge_policy, MergePolicy::SetIfMissing);

    // read_only=true → Immutable; hidden=true → Hidden (not System, even with ViewContext::Never)
    let def_ro = PropertyDefinition::from_legacy_fields(
        uuid::Uuid::nil(),
        "test2",
        "Test2",
        PropertyType::Text,
        ViewContext::Never,
        false,
        false,
        true,
        true,
    );
    assert_eq!(def_ro.visibility, PropertyVisibility::Hidden);
    assert_eq!(def_ro.mutability, PropertyMutability::Immutable);
}

// ── ADR-0025 JSON wire-format parity ─────────────────────────────────

/// ADR-0025 T8: Wire-format parity for PropertyVisibility.
/// Must serialize as lowercase string (not tagged object).
#[test]
fn test_wire_format_property_visibility() {
    let json = serde_json::to_string(&PropertyVisibility::System).unwrap();
    assert_eq!(json, "\"system\"");
}

/// ADR-0025 T8: Wire-format parity for DerivedSource tagged variant.
/// Unit variants serialize as plain strings; OtherProperty as tagged object.
#[test]
fn test_wire_format_derived_source() {
    let block_content_json = serde_json::to_string(&DerivedSource::BlockContent).unwrap();
    assert_eq!(block_content_json, "\"block_content\"");

    let other_json = serde_json::to_string(&DerivedSource::OtherProperty("foo".into())).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&other_json).unwrap();
    assert_eq!(parsed["type"].as_str().unwrap(), "other_property");
    assert_eq!(parsed["value"].as_str().unwrap(), "foo");
}

/// ADR-0025 T8: Wire-format parity for MergePolicy.
/// Must use snake_case enum variant names.
#[test]
fn test_wire_format_merge_policy() {
    assert_eq!(
        serde_json::to_string(&MergePolicy::SetIfMissing).unwrap(),
        "\"set_if_missing\""
    );
    assert_eq!(
        serde_json::to_string(&MergePolicy::AskOnConflict).unwrap(),
        "\"ask_on_conflict\""
    );
}

/// ADR-0025 T8: PropertyDefinition with all ADR-0025 fields omitted
/// (default values) must deserialize from legacy JSON without the new fields.
#[test]
fn test_backward_compat_deser_legacy_property_def() {
    // Legacy JSON — no visibility, mutability, derived_from, merge_policy.
    // All enum variants use derived serde (capitalized form).
    let legacy_json = json!({
        "id": "00000000-0000-0000-0000-000000000001",
        "db_ident": "test/prop",
        "title": "Test Property",
        "property_type": "Text",
        "cardinality": "One",
        "closed_values": [],
        "view_context": "Page",
        "public": true,
        "queryable": true,
        "hidden": false,
        "attribute": null
    });

    let def: PropertyDefinition = serde_json::from_value(legacy_json).unwrap();
    assert_eq!(def.visibility, PropertyVisibility::Inline); // default
    assert_eq!(def.mutability, PropertyMutability::Mutable); // default
    assert_eq!(def.derived_from, None); // default
    assert_eq!(def.merge_policy, MergePolicy::SetIfMissing); // default
}

/// ADR-0025 T8: PropertyDefinition with full ADR-0025 fields must serialize
/// to JSON containing all new fields with correct wire format.
#[test]
fn test_wire_format_full_property_definition() {
    let def = PropertyDefinition::new(uuid::Uuid::nil(), "test", "Test", PropertyType::Text)
        .with_visibility(PropertyVisibility::System)
        .with_mutability(PropertyMutability::Immutable)
        .with_derived_from(DerivedSource::BlockContent)
        .with_merge_policy(MergePolicy::Union);

    let json = serde_json::to_string(&def).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["visibility"].as_str().unwrap(), "system");
    assert_eq!(parsed["mutability"].as_str().unwrap(), "immutable");
    assert_eq!(parsed["derived_from"].as_str().unwrap(), "block_content");
    assert_eq!(parsed["merge_policy"].as_str().unwrap(), "union");
}
