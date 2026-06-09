//! Tests for `TemplateContract` and `TemplateLayout` value objects.
//!
//! TDD: written first. These tests will fail until the types are
//! implemented in `entities/template_contract.rs` and re-exported
//! from the domain layer.
//!
//! Covers (Q030 ROADMAP):
//! - serialization/deserialization round-trip
//! - `TemplateLayout` discriminator tag (`inline` / `panel` / `locked`)
//! - `PropertyKey` validation rules
//! - `Version` comparison and serialization
//! - validation: required properties present, locked properties
//!   cannot be modified
//! - version mismatch detection
//! - helper constructors (default layout, missing required detection)

use quilt_domain::entities::{PropertyKey, TemplateContract, TemplateLayout, Version};
use std::collections::HashMap;

// ── PropertyKey tests ─────────────────────────────────────────────

#[test]
fn property_key_normalizes_lowercase() {
    let k = PropertyKey::new("Title").unwrap();
    assert_eq!(k.as_str(), "title");
}

#[test]
fn property_key_normalizes_separators() {
    // Per `normalize_property_name` in the domain: `/`, ` `, `_` all → `-`
    assert_eq!(PropertyKey::new("foo/bar").unwrap().as_str(), "foo-bar");
    assert_eq!(PropertyKey::new("foo bar").unwrap().as_str(), "foo-bar");
    assert_eq!(PropertyKey::new("foo_bar").unwrap().as_str(), "foo-bar");
}

#[test]
fn property_key_trims_whitespace() {
    let k = PropertyKey::new("  status  ").unwrap();
    assert_eq!(k.as_str(), "status");
}

#[test]
fn property_key_rejects_empty() {
    let result = PropertyKey::new("");
    assert!(result.is_err(), "empty key must be rejected");
}

#[test]
fn property_key_rejects_internal_whitespace() {
    // After trim, internal whitespace should be normalized to `-`
    let k = PropertyKey::new("hello world").unwrap();
    assert_eq!(k.as_str(), "hello-world");
}

#[test]
fn property_key_display() {
    let k = PropertyKey::new("status").unwrap();
    assert_eq!(format!("{}", k), "status");
}

#[test]
fn property_key_equality_ignores_normalization() {
    let a = PropertyKey::new("Status").unwrap();
    let b = PropertyKey::new("status").unwrap();
    assert_eq!(a, b, "normalized keys compare equal");
}

// ── Version tests ─────────────────────────────────────────────────

#[test]
fn version_new_starts_at_1() {
    let v = Version::new();
    assert_eq!(v.as_u32(), 1);
}

#[test]
fn version_bump_increments() {
    let v = Version::new();
    let v2 = v.bump();
    assert_eq!(v.as_u32(), 1);
    assert_eq!(v2.as_u32(), 2);
}

#[test]
fn version_serde_roundtrip() {
    let v = Version::new();
    let json = serde_json::to_string(&v).expect("serialize");
    let parsed: Version = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(v, parsed);
}

#[test]
fn version_serializes_as_number() {
    // Versions should serialize as plain integers for clean MCP JSON.
    let v = Version::new();
    let json = serde_json::to_string(&v).expect("serialize");
    assert_eq!(json, "1", "Version should serialize as bare integer");
}

#[test]
fn version_deserializes_from_number() {
    let v: Version = serde_json::from_str("42").expect("deserialize");
    assert_eq!(v.as_u32(), 42);
}

#[test]
fn version_comparison_works() {
    let v1 = Version::new();
    let v2 = v1.bump();
    assert!(v2 > v1);
    assert!(v1 < v2);
    assert_eq!(v1, Version::new());
}

// ── TemplateLayout tests ──────────────────────────────────────────

#[test]
fn template_layout_inline_serializes_with_tag() {
    let layout = TemplateLayout::Inline(PropertyKey::new("title").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    // Externally-tagged representation
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["tag"], "inline");
    assert_eq!(v["property"], "title");
}

#[test]
fn template_layout_panel_serializes_with_tag() {
    let layout = TemplateLayout::Panel(PropertyKey::new("notes").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["tag"], "panel");
    assert_eq!(v["property"], "notes");
}

#[test]
fn template_layout_locked_serializes_with_tag() {
    let layout = TemplateLayout::Locked(PropertyKey::new("type").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert_eq!(v["tag"], "locked");
    assert_eq!(v["property"], "type");
}

#[test]
fn template_layout_roundtrip_inline() {
    let layout = TemplateLayout::Inline(PropertyKey::new("status").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, layout);
}

#[test]
fn template_layout_roundtrip_panel() {
    let layout = TemplateLayout::Panel(PropertyKey::new("summary").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, layout);
}

#[test]
fn template_layout_roundtrip_locked() {
    let layout = TemplateLayout::Locked(PropertyKey::new("id").unwrap());
    let json = serde_json::to_string(&layout).expect("serialize");
    let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, layout);
}

#[test]
fn template_layout_property_returns_key() {
    let k = PropertyKey::new("title").unwrap();
    assert_eq!(TemplateLayout::Inline(k.clone()).property(), &k);
    assert_eq!(TemplateLayout::Panel(k.clone()).property(), &k);
    assert_eq!(TemplateLayout::Locked(k.clone()).property(), &k);
}

#[test]
fn template_layout_is_locked_predicate() {
    assert!(TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_locked());
    assert!(!TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_locked());
    assert!(!TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_locked());
}

#[test]
fn template_layout_is_inline_predicate() {
    assert!(TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_inline());
    assert!(!TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_inline());
    assert!(!TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_inline());
}

#[test]
fn template_layout_is_panel_predicate() {
    assert!(TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_panel());
    assert!(!TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_panel());
    assert!(!TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_panel());
}

// ── TemplateContract tests ────────────────────────────────────────

fn sample_contract() -> TemplateContract {
    TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .required_property("title")
        .required_property("status")
        .required_property("template")
        .inline_layout("title")
        .panel_layout("status")
        .locked_layout("template")
        .version(Version::new())
        .build()
        .expect("sample contract should build")
}

#[test]
fn template_contract_builds_with_all_fields() {
    let contract = sample_contract();
    assert_eq!(contract.required_properties().len(), 3);
    assert_eq!(contract.layout().len(), 3);
    assert_eq!(contract.locked_properties().len(), 1);
    assert_eq!(contract.version().as_u32(), 1);
}

#[test]
fn template_contract_serializes_roundtrip() {
    let contract = sample_contract();
    let json = serde_json::to_string(&contract).expect("serialize");
    let parsed: TemplateContract = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, contract);
}

#[test]
fn template_contract_serializes_with_expected_fields() {
    let contract = sample_contract();
    let json = serde_json::to_string(&contract).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
    assert!(v.get("template_id").is_some(), "template_id field present");
    assert!(
        v.get("required_properties").is_some(),
        "required_properties field present"
    );
    assert!(v.get("layout").is_some(), "layout field present");
    assert!(
        v.get("locked_properties").is_some(),
        "locked_properties field present"
    );
    assert!(v.get("version").is_some(), "version field present");
}

#[test]
fn template_contract_locked_set_includes_all_locked_layouts() {
    let contract = sample_contract();
    let locked = contract.locked_properties();
    // The "template" key is in the Locked layout → must appear here too.
    assert!(locked.iter().any(|k| k.as_str() == "template"));
}

#[test]
fn template_contract_empty_build_works() {
    let contract = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .build()
        .expect("empty contract should build");
    assert!(contract.required_properties().is_empty());
    assert!(contract.layout().is_empty());
    assert!(contract.locked_properties().is_empty());
    assert_eq!(contract.version().as_u32(), 1);
}

#[test]
fn template_contract_builder_fails_without_template_id() {
    let result = TemplateContract::builder()
        .required_property("title")
        .build();
    assert!(result.is_err(), "template_id is required");
}

#[test]
fn template_contract_validation_layout_keys_must_be_unique() {
    // "title" is registered as both Inline and Panel — should fail validation.
    let result = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .inline_layout("title")
        .panel_layout("title")
        .build();
    assert!(result.is_err(), "duplicate layout keys must be rejected");
}

#[test]
fn template_contract_validation_required_must_have_layout() {
    // A required property with no layout is ambiguous — should fail.
    let result = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .required_property("title")
        .build();
    assert!(
        result.is_err(),
        "required property without layout must be rejected"
    );
}

#[test]
fn template_contract_layout_must_have_required_for_non_locked() {
    // Inline/panel layouts must be declared as required too.
    let result = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .inline_layout("title")
        .build();
    assert!(
        result.is_err(),
        "non-locked layout must also be required (inversion of above)"
    );
}

#[test]
fn template_contract_validation_passes_when_consistent() {
    let contract = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .required_property("title")
        .required_property("status")
        .inline_layout("title")
        .panel_layout("status")
        .build();
    assert!(contract.is_ok(), "consistent contract should build");
}

#[test]
fn template_contract_validation_locked_must_be_required() {
    // "id" is locked but not in required_properties → error.
    let result = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .required_property("title")
        .locked_layout("id")
        .build();
    assert!(
        result.is_err(),
        "locked property must be declared as required"
    );
}

#[test]
fn template_contract_validation_locked_also_required_works() {
    let result = TemplateContract::builder()
        .template_id(quilt_domain::value_objects::Uuid::new_v4())
        .required_property("id")
        .locked_layout("id")
        .build();
    assert!(
        result.is_ok(),
        "locked+required declaration should be valid: {result:?}"
    );
}

// ── Contract validation logic tests ───────────────────────────────

fn kv(props: &[(&str, &str)]) -> HashMap<String, String> {
    props
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn contract_validates_required_properties_present() {
    let contract = sample_contract();
    // Has all 3 required keys — valid.
    let props = kv(&[
        ("title", "My Title"),
        ("status", "todo"),
        ("template", "ref"),
    ]);
    assert!(contract.validate_application(&props).is_ok());

    // Missing "status" — should fail.
    let props = kv(&[("title", "My Title"), ("template", "ref")]);
    let result = contract.validate_application(&props);
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("status"),
        "error should mention missing key: {err}"
    );
}

#[test]
fn contract_validates_locked_property_not_overridden() {
    let contract = sample_contract();
    // The template's canonical value for "template" is "ref-template".
    let template_values = kv(&[("title", ""), ("status", ""), ("template", "ref-template")]);

    // "template" is locked; if the user changes it to something
    // different, the strict mutation check must reject it.
    let proposed = kv(&[
        ("title", "x"),
        ("status", "y"),
        ("template", "changed-by-user"),
    ]);

    let result = contract.check_locked_against_template(&proposed, &template_values);
    assert!(
        result.is_err(),
        "changing a locked property must be rejected: {result:?}"
    );
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.contains("template"),
        "error should mention the property: {err}"
    );
}

#[test]
fn contract_strict_locked_check_passes_when_values_match() {
    let contract = sample_contract();
    let template_values = kv(&[
        ("title", "Original Title"),
        ("status", "todo"),
        ("template", "ref-template"),
    ]);
    // User keeps "template" at the template's value — fine.
    let proposed = kv(&[
        ("title", "User's Title"),
        ("status", "in-progress"),
        ("template", "ref-template"),
    ]);
    assert!(
        contract
            .check_locked_against_template(&proposed, &template_values)
            .is_ok()
    );
}

#[test]
fn contract_version_mismatch_returns_error() {
    let contract_v1 = sample_contract();
    let expected_v2 = contract_v1.version().bump();

    let props = kv(&[("title", "x"), ("status", "y"), ("template", "ref")]);
    let result = contract_v1.validate_application_with_version(&props, expected_v2);
    assert!(result.is_err(), "version mismatch must be detected");
    let err = format!("{}", result.unwrap_err());
    assert!(
        err.to_lowercase().contains("version"),
        "error must mention 'version': {err}"
    );
}

#[test]
fn contract_version_match_passes() {
    let contract = sample_contract();
    let props = kv(&[("title", "x"), ("status", "y"), ("template", "ref")]);
    let result = contract.validate_application_with_version(&props, *contract.version());
    assert!(result.is_ok(), "matching version should pass: {result:?}");
}

#[test]
fn contract_check_locked_passes_when_locked_unchanged() {
    let contract = sample_contract();
    let props = kv(&[
        ("title", "x"),
        ("status", "y"),
        ("template", "the-template"),
    ]);
    let result = contract.check_locked_mutations(&props);
    assert!(result.is_ok());
}

#[test]
fn contract_extra_properties_are_allowed() {
    // Extra user-added properties (not declared in contract) should be
    // allowed — contracts restrict what the TEMPLATE requires, not what
    // the user can add.
    let contract = sample_contract();
    let props = kv(&[
        ("title", "x"),
        ("status", "y"),
        ("template", "ref"),
        ("my-custom-prop", "user-value"),
    ]);
    assert!(contract.validate_application(&props).is_ok());
}
