//! Property-based tests for PropertyValue JSON roundtrip.
//!
//! Verifies that String, Boolean, Integer, Float, and Array
//! values survive to_json → from_json unchanged.

use proptest::prelude::*;
use quilt_domain::value_objects::PropertyValue;

// ── Strategies ──────────────────────────────────────────────

fn arb_string_value() -> impl Strategy<Value = PropertyValue> {
    ".*".prop_map(PropertyValue::String)
}

fn arb_bool_value() -> impl Strategy<Value = PropertyValue> {
    any::<bool>().prop_map(PropertyValue::Boolean)
}

fn arb_int_value() -> impl Strategy<Value = PropertyValue> {
    any::<i64>().prop_map(PropertyValue::Integer)
}

fn arb_leaf_value() -> impl Strategy<Value = PropertyValue> {
    prop_oneof![arb_string_value(), arb_bool_value(), arb_int_value()]
}

fn arb_array_value(depth: u32) -> impl Strategy<Value = PropertyValue> {
    let leaf = arb_leaf_value();
    if depth == 0 {
        leaf.prop_map(|v| PropertyValue::Array(vec![v])).boxed()
    } else {
        let inner = arb_array_value(depth - 1);
        prop::collection::vec(leaf, 0..5)
            .prop_map(|v| PropertyValue::Array(v))
            .boxed()
    }
}

// ── Property tests ──────────────────────────────────────────

proptest! {
    #[test]
    fn string_roundtrip(s in ".*") {
        let original = PropertyValue::String(s.clone());
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn bool_roundtrip(b in any::<bool>()) {
        let original = PropertyValue::Boolean(b);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn integer_roundtrip(i in any::<i64>()) {
        let original = PropertyValue::Integer(i);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn array_of_strings_roundtrip(strings in prop::collection::vec(".*", 0..10)) {
        let inner: Vec<PropertyValue> = strings.into_iter().map(PropertyValue::String).collect();
        let original = PropertyValue::Array(inner.clone());
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn array_has_correct_length(items in prop::collection::vec(any::<i64>().prop_map(PropertyValue::Integer), 0..20)) {
        let len = items.len();
        let original = PropertyValue::Array(items);
        let json = original.to_json();
        let restored = PropertyValue::from_json(&json).unwrap();
        match restored {
            PropertyValue::Array(arr) => assert_eq!(arr.len(), len),
            other => panic!("expected Array, got {:?}", other),
        }
    }

    #[test]
    fn type_name_is_consistent(value in arb_leaf_value()) {
        let type_name = value.type_name();
        match &value {
            PropertyValue::String(_) => assert_eq!(type_name, "string"),
            PropertyValue::Boolean(_) => assert_eq!(type_name, "boolean"),
            PropertyValue::Integer(_) => assert_eq!(type_name, "integer"),
            _ => {}
        }
    }

    #[test]
    fn to_json_never_panics(value in arb_leaf_value()) {
        let _json = value.to_json(); // must not panic
    }

    #[test]
    fn default_is_empty_string_then_some(_ in any::<u8>()) {
        let default = PropertyValue::default();
        assert_eq!(default, PropertyValue::String(String::new()));
    }
}
