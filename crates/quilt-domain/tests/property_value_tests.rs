//! Integration tests for PropertyValue value object.
//!
//! Covers: constructors, type_name, to_json, from_json (all variants),
//! Display, Default, parse_properties (which internally normalizes names),
//! and edge cases (empty arrays, nested arrays, null, objects).

use chrono::{TimeZone, Utc};
use quilt_domain::value_objects::{parse_properties, PropertyValue};

// ── Constructors ──────────────────────────────────────────────

#[test]
fn test_constructors() {
    assert_eq!(PropertyValue::string("hello"), PropertyValue::String("hello".into()));
    assert_eq!(PropertyValue::boolean(true), PropertyValue::Boolean(true));
    assert_eq!(PropertyValue::integer(42), PropertyValue::Integer(42));
    assert_eq!(PropertyValue::float(3.14), PropertyValue::Float(3.14));
    assert_eq!(PropertyValue::reference("mypage"), PropertyValue::Ref("mypage".into()));
}

#[test]
fn test_string_constructor_accepts_string_types() {
    let s = PropertyValue::string("owned");
    assert_eq!(s, PropertyValue::String("owned".into()));

    let s = PropertyValue::string(String::from("owned"));
    assert_eq!(s, PropertyValue::String("owned".into()));
}

// ── type_name ─────────────────────────────────────────────────

#[test]
fn test_type_name_all_variants() {
    assert_eq!(PropertyValue::String("x".into()).type_name(), "string");
    assert_eq!(PropertyValue::Boolean(true).type_name(), "boolean");
    assert_eq!(PropertyValue::Integer(1).type_name(), "integer");
    assert_eq!(PropertyValue::Float(1.0).type_name(), "float");
    assert_eq!(
        PropertyValue::Date(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()).type_name(),
        "date"
    );
    assert_eq!(PropertyValue::Ref("p".into()).type_name(), "ref");
    assert_eq!(PropertyValue::Array(vec![]).type_name(), "array");
}

// ── to_json / from_json roundtrip ─────────────────────────────

#[test]
fn test_json_roundtrip_string() {
    let original = PropertyValue::String("hello".into());
    let json = original.to_json();
    assert_eq!(json, serde_json::json!("hello"));
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_boolean() {
    let original = PropertyValue::Boolean(true);
    let json = original.to_json();
    assert_eq!(json, serde_json::json!(true));
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_integer() {
    let original = PropertyValue::Integer(42);
    let json = original.to_json();
    assert_eq!(json, serde_json::json!(42));
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_integer_negative() {
    let original = PropertyValue::Integer(-7);
    let json = original.to_json();
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_float() {
    let original = PropertyValue::Float(3.14);
    let json = original.to_json();
    let restored = PropertyValue::from_json(&json).unwrap();
    match restored {
        PropertyValue::Float(f) => assert!((f - 3.14).abs() < f64::EPSILON),
        other => panic!("expected Float, got {:?}", other),
    }
}

#[test]
fn test_json_roundtrip_date() {
    let dt = Utc.with_ymd_and_hms(2026, 5, 15, 10, 30, 0).unwrap();
    let original = PropertyValue::Date(dt);
    let json = original.to_json();
    assert!(json.as_str().unwrap().starts_with("2026-05-15"));
    // from_json cannot reconstruct a Date from a JSON string in the current impl
    // (it becomes a String). Document this behavior.
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(restored, PropertyValue::String(dt.to_rfc3339()));
}

#[test]
fn test_json_roundtrip_ref() {
    let original = PropertyValue::Ref("mypage".into());
    let json = original.to_json();
    assert_eq!(json, serde_json::json!("mypage"));
    // from_json interprets as String (no way to distinguish from plain string)
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(restored, PropertyValue::String("mypage".into()));
}

#[test]
fn test_json_roundtrip_array() {
    let original = PropertyValue::Array(vec![
        PropertyValue::String("a".into()),
        PropertyValue::Integer(1),
    ]);
    let json = original.to_json();
    assert_eq!(json, serde_json::json!(["a", 1]));
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_nested_array() {
    let original = PropertyValue::Array(vec![
        PropertyValue::Array(vec![PropertyValue::Integer(1), PropertyValue::Integer(2)]),
        PropertyValue::String("outer".into()),
    ]);
    let json = original.to_json();
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_json_roundtrip_empty_array() {
    let original = PropertyValue::Array(vec![]);
    let json = original.to_json();
    assert_eq!(json, serde_json::json!([]));
    let restored = PropertyValue::from_json(&json).unwrap();
    assert_eq!(original, restored);
}

#[test]
fn test_from_json_null_returns_none() {
    assert_eq!(PropertyValue::from_json(&serde_json::Value::Null), None);
}

#[test]
fn test_from_json_object_returns_none() {
    let obj = serde_json::json!({"key": "value"});
    assert_eq!(PropertyValue::from_json(&obj), None);
}

// ── Display ───────────────────────────────────────────────────

#[test]
fn test_display_string() {
    assert_eq!(format!("{}", PropertyValue::String("hello".into())), "hello");
}

#[test]
fn test_display_boolean() {
    assert_eq!(format!("{}", PropertyValue::Boolean(true)), "true");
    assert_eq!(format!("{}", PropertyValue::Boolean(false)), "false");
}

#[test]
fn test_display_integer() {
    assert_eq!(format!("{}", PropertyValue::Integer(42)), "42");
    assert_eq!(format!("{}", PropertyValue::Integer(-5)), "-5");
}

#[test]
fn test_display_float() {
    let val = PropertyValue::Float(3.14);
    let s = format!("{}", val);
    assert!(s.starts_with("3.14"));
}

#[test]
fn test_display_date() {
    let dt = Utc.with_ymd_and_hms(2026, 6, 2, 0, 0, 0).unwrap();
    let val = PropertyValue::Date(dt);
    assert_eq!(format!("{}", val), "2026-06-02");
}

#[test]
fn test_display_ref() {
    let val = PropertyValue::Ref("mypage".into());
    assert_eq!(format!("{}", val), "[[mypage]]");
}

#[test]
fn test_display_array() {
    let val = PropertyValue::Array(vec![
        PropertyValue::String("a".into()),
        PropertyValue::Integer(1),
    ]);
    assert_eq!(format!("{}", val), "[a, 1]");
}

#[test]
fn test_display_empty_array() {
    let val = PropertyValue::Array(vec![]);
    assert_eq!(format!("{}", val), "[]");
}

// ── Default ───────────────────────────────────────────────────

#[test]
fn test_default_is_empty_string() {
    assert_eq!(PropertyValue::default(), PropertyValue::String(String::new()));
}

// ── parse_properties (exercises normalize_property_name internally) ──

#[test]
fn test_parse_properties_normalizes_keys() {
    let mut map = serde_json::Map::new();
    // Keys with mixed case, slashes, spaces, underscores — all get normalized
    map.insert("My Title".to_string(), serde_json::Value::String("hello".to_string()));
    map.insert("FOO/BAR".to_string(), serde_json::Value::String("baz".to_string()));
    map.insert("snake_case".to_string(), serde_json::Value::String("val".to_string()));
    let props = parse_properties(&map);

    assert_eq!(props.get("my-title"), Some(&PropertyValue::String("hello".into())));
    assert_eq!(props.get("foo-bar"), Some(&PropertyValue::String("baz".into())));
    assert_eq!(props.get("snake-case"), Some(&PropertyValue::String("val".into())));
}

#[test]
fn test_parse_properties_single() {
    let mut map = serde_json::Map::new();
    map.insert("status".to_string(), serde_json::Value::String("draft".to_string()));
    let props = parse_properties(&map);
    assert_eq!(props.len(), 1);
    assert_eq!(props.get("status"), Some(&PropertyValue::String("draft".into())));
}

#[test]
fn test_parse_properties_multiple() {
    let mut map = serde_json::Map::new();
    map.insert("count".to_string(), serde_json::json!(5));
    map.insert("active".to_string(), serde_json::json!(true));
    let props = parse_properties(&map);
    assert_eq!(props.len(), 2);
    assert_eq!(props.get("count"), Some(&PropertyValue::Integer(5)));
    assert_eq!(props.get("active"), Some(&PropertyValue::Boolean(true)));
}

#[test]
fn test_parse_properties_skips_invalid_values() {
    let mut map = serde_json::Map::new();
    map.insert("valid".to_string(), serde_json::Value::String("ok".to_string()));
    map.insert("invalid".to_string(), serde_json::Value::Null);
    let props = parse_properties(&map);
    assert_eq!(props.len(), 1);
    assert!(props.contains_key("valid"));
    assert!(!props.contains_key("invalid"));
}

#[test]
fn test_parse_properties_empty() {
    let map = serde_json::Map::new();
    let props = parse_properties(&map);
    assert!(props.is_empty());
}
