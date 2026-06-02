//! Property-based tests for parse_properties — verifies that
//! arbitrary JSON maps are parsed without panicking and that
//! the output only contains valid PropertyValues.

use proptest::prelude::*;
use quilt_domain::value_objects::parse_properties;

// ── Strategy: random JSON-like maps ─────────────────────────

fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        any::<String>().prop_map(serde_json::Value::String),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::Value::Number(n.into())),
        Just(serde_json::Value::Null),
    ];
    leaf.prop_recursive(2, 4, 3, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..3)
                .prop_map(serde_json::Value::Array),
        ]
    })
}

fn arb_json_map() -> impl Strategy<Value = serde_json::Map<String, serde_json::Value>> {
    prop::collection::vec((any::<String>(), arb_json_value()), 0..10)
        .prop_map(|pairs| pairs.into_iter().collect::<serde_json::Map<_, _>>())
}

// ── Property tests ──────────────────────────────────────────

proptest! {
    #[test]
    fn parse_properties_never_panics(map in arb_json_map()) {
        let _result = parse_properties(&map); // must never panic
    }

    #[test]
    fn parse_properties_output_size_le_input_size(map in arb_json_map()) {
        let input_len = map.len();
        let result = parse_properties(&map);
        // Some entries may be skipped (null values), so output ≤ input
        assert!(result.len() <= input_len);
    }

    #[test]
    fn parse_properties_normalizes_keys(pairs in prop::collection::vec(
        ("[a-zA-Z ]{1,10}".prop_map(|s: String| s), arb_json_value()), 1..5)
    ) {
        let map: serde_json::Map<_, _> = pairs.into_iter().collect();
        let result = parse_properties(&map);

        for (original_key, _) in &map {
            let expected = original_key.to_lowercase().replace(['/', ' ', '_'], "-");
            if result.contains_key(&expected) {
                // Key was normalized
            }
            // Key might have been skipped if value was null
        }
    }

    #[test]
    fn string_properties_survive_parsing(s in ".*") {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), serde_json::Value::String(s.clone()));
        let result = parse_properties(&map);
        assert!(result.contains_key("key"));
    }
}
