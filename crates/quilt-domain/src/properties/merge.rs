//! Pure key-level property merge (F8 reduced).
//!
//! Generic over the [`PropertyEntry`] trait so the same logic works for
//! `DefaultPropertyEntry<PropertyValue>` (Page), bare `PropertyValue` (Block),
//! or any future V2 entry type (`VersionedPropertyEntry`, `SourcedPropertyEntry`,
//! …) without modification. This is the OCP payoff of the trait hierarchy.
//!
//! ## Merge contract
//!
//! - **Distinct keys**: entries from both sides are preserved (additive).
//! - **Same key, both sides have a timestamp**: the value with the later
//!   `updated_at` wins.
//! - **Same key, exactly one side has a timestamp**: the timestamped side wins.
//! - **Same key, neither side has a timestamp, or equal timestamps**: the
//!   `existing` value wins (deterministic, no randomness).
//! - **Pure function**: the input maps are never mutated; the returned map is
//!   a fresh allocation.
//!
//! This is a pure module-level function — no traits, no async, no I/O. Easy
//! to test, easy to reason about, easy to extend (new merge strategies = new
//! types implementing `Mergeable`, not changes here).

use crate::properties::entry::PropertyEntry;
use std::collections::HashMap;

/// Merge `incoming` into `existing`, returning the merged map.
///
/// See module docs for the merge contract.
///
/// # Type parameter
///
/// - `E: PropertyEntry + Clone` — `Clone` is required because we read from
///   `incoming` to insert into the result. `PropertyEntry` is the OCP extension
///   point: this function does not change when new entry types are added.
pub fn merge_properties<E: PropertyEntry + Clone>(
    existing: &HashMap<String, E>,
    incoming: HashMap<String, E>,
) -> HashMap<String, E> {
    let mut result = existing.clone();
    for (k, v) in incoming {
        match result.get(&k) {
            None => {
                result.insert(k, v);
            }
            Some(existing_v) => {
                // Ask: "should the existing entry (self) be replaced by the
                // incoming one (other)?" If yes, overwrite. Else keep existing
                // (deterministic tie-break or older timestamp).
                if existing_v.should_be_replaced_by(&v) {
                    result.insert(k, v);
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::entry::{DefaultPropertyEntry, HasTimestamp, HasValue};
    use crate::value_objects::PropertyValue;
    use chrono::TimeZone;

    fn ts(secs: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.timestamp_opt(secs, 0).single().unwrap()
    }

    fn entry_str(s: &str) -> DefaultPropertyEntry<PropertyValue> {
        DefaultPropertyEntry::new(PropertyValue::string(s))
    }

    fn entry_str_ts(
        s: &str,
        t: chrono::DateTime<chrono::Utc>,
    ) -> DefaultPropertyEntry<PropertyValue> {
        DefaultPropertyEntry::with_timestamp(PropertyValue::string(s), t)
    }

    // ── F8 spec scenarios ──

    #[test]
    fn distinct_keys_both_survive() {
        let mut existing = HashMap::new();
        existing.insert("a".to_string(), entry_str("A0"));
        existing.insert("b".to_string(), entry_str("B0"));
        let mut incoming = HashMap::new();
        incoming.insert("c".to_string(), entry_str("C0"));

        let merged = merge_properties(&existing, incoming);

        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged["a"].value(),
            &PropertyValue::String("A0".to_string())
        );
        assert_eq!(
            merged["b"].value(),
            &PropertyValue::String("B0".to_string())
        );
        assert_eq!(
            merged["c"].value(),
            &PropertyValue::String("C0".to_string())
        );
    }

    #[test]
    fn same_key_distinct_timestamps_latest_wins() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str_ts("Doing", ts(100)));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str_ts("Done", ts(200)));

        let merged = merge_properties(&existing, incoming);

        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("Done".to_string())
        );
        assert_eq!(merged["status"].updated_at(), Some(ts(200)));
    }

    #[test]
    fn same_key_existing_older_incoming_wins() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str_ts("Done", ts(200)));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str_ts("Doing", ts(100)));

        let merged = merge_properties(&existing, incoming);

        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("Done".to_string())
        );
        assert_eq!(merged["status"].updated_at(), Some(ts(200)));
    }

    #[test]
    fn same_key_equal_timestamps_existing_wins() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str_ts("Doing", ts(100)));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str_ts("Done", ts(100)));

        let merged = merge_properties(&existing, incoming);

        // Same ts → existing wins (deterministic tie-break).
        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("Doing".to_string())
        );
    }

    #[test]
    fn same_key_only_one_timestamped_timestamped_wins() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str("old-bare"));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str_ts("new-ts", ts(50)));

        let merged = merge_properties(&existing, incoming);

        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("new-ts".to_string())
        );
        assert_eq!(merged["status"].updated_at(), Some(ts(50)));
    }

    #[test]
    fn same_key_existing_timestamped_incoming_bare_keeps_existing() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str_ts("old-ts", ts(50)));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str("new-bare"));

        let merged = merge_properties(&existing, incoming);

        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("old-ts".to_string())
        );
        assert_eq!(merged["status"].updated_at(), Some(ts(50)));
    }

    #[test]
    fn same_key_both_bare_existing_wins() {
        let mut existing = HashMap::new();
        existing.insert("status".to_string(), entry_str("existing-bare"));
        let mut incoming = HashMap::new();
        incoming.insert("status".to_string(), entry_str("incoming-bare"));

        let merged = merge_properties(&existing, incoming);

        // Both None → "keep self" tie-break → existing.
        assert_eq!(
            merged["status"].value(),
            &PropertyValue::String("existing-bare".to_string())
        );
    }

    // ── Pure function contract ──

    #[test]
    fn does_not_mutate_inputs() {
        let mut existing = HashMap::new();
        existing.insert("a".to_string(), entry_str("A0"));
        let mut incoming = HashMap::new();
        incoming.insert("a".to_string(), entry_str_ts("A1", ts(100)));

        let _ = merge_properties(&existing, incoming.clone());

        // Inputs are untouched after the merge.
        assert_eq!(existing.len(), 1);
        assert_eq!(
            existing["a"].value(),
            &PropertyValue::String("A0".to_string())
        );
        assert_eq!(incoming.len(), 1);
        assert_eq!(
            incoming["a"].value(),
            &PropertyValue::String("A1".to_string())
        );
    }

    #[test]
    fn empty_incoming_returns_existing_clone() {
        let mut existing = HashMap::new();
        existing.insert("a".to_string(), entry_str("A0"));
        existing.insert("b".to_string(), entry_str("B0"));

        let merged = merge_properties(&existing, HashMap::new());

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged["a"].value(),
            &PropertyValue::String("A0".to_string())
        );
        assert_eq!(
            merged["b"].value(),
            &PropertyValue::String("B0".to_string())
        );
    }

    #[test]
    fn empty_existing_returns_incoming_clone() {
        let mut incoming = HashMap::new();
        incoming.insert("a".to_string(), entry_str("A0"));
        incoming.insert("b".to_string(), entry_str("B0"));

        let merged = merge_properties(&HashMap::new(), incoming);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn deterministic_same_inputs_same_output() {
        let mut existing = HashMap::new();
        existing.insert("a".to_string(), entry_str_ts("A0", ts(100)));
        existing.insert("b".to_string(), entry_str_ts("B0", ts(200)));
        let mut incoming = HashMap::new();
        incoming.insert("b".to_string(), entry_str_ts("B1", ts(150)));
        incoming.insert("c".to_string(), entry_str("C0"));

        let merged1 = merge_properties(&existing, incoming.clone());
        let merged2 = merge_properties(&existing, incoming.clone());

        // HashMap ordering may vary, so compare via set semantics.
        assert_eq!(merged1.len(), merged2.len());
        for (k, v) in &merged1 {
            assert_eq!(merged2.get(k).unwrap().value(), v.value());
        }
    }

    // ── Property test: determinism (T-B.5 spec requirement) ──

    use proptest::prelude::*;

    /// Strategy: generate a HashMap<String, DefaultPropertyEntry<PropertyValue>>
    /// with 0-5 entries, each key a unique "kN" string, and each value a string
    /// with a random timestamp 0..1000.
    fn arb_entry_map() -> impl Strategy<Value = HashMap<String, DefaultPropertyEntry<PropertyValue>>>
    {
        proptest::collection::hash_map(
            proptest::string::string_regex("[a-z][a-z0-9]{0,4}")
                .expect("valid regex")
                .prop_filter("non-empty", |s| !s.is_empty()),
            (0..1000i64, "[A-Za-z0-9 ]{0,5}").prop_map(|(secs, s)| {
                let t = ts(secs);
                DefaultPropertyEntry::with_timestamp(
                    PropertyValue::string(format!("existing-{}-{}", s, secs)),
                    t,
                )
            }),
            0..5,
        )
    }

    proptest! {
        #[test]
        fn merge_is_deterministic(
            (existing, incoming) in (arb_entry_map(), arb_entry_map())
        ) {
            let m1 = merge_properties(&existing, incoming.clone());
            let m2 = merge_properties(&existing, incoming);
            prop_assert_eq!(m1.len(), m2.len());
            for (k, v) in &m1 {
                let other = m2.get(k).expect("same keys present in both runs");
                prop_assert_eq!(v.value(), other.value());
                prop_assert_eq!(v.updated_at(), other.updated_at());
            }
        }

        #[test]
        fn merge_does_not_mutate_existing(
            (existing, incoming) in (arb_entry_map(), arb_entry_map())
        ) {
            let snapshot: HashMap<String, PropertyValue> = existing
                .iter()
                .map(|(k, v)| (k.clone(), v.value().clone()))
                .collect();
            let _ = merge_properties(&existing, incoming);
            let after: HashMap<String, PropertyValue> = existing
                .iter()
                .map(|(k, v)| (k.clone(), v.value().clone()))
                .collect();
            prop_assert_eq!(snapshot, after);
        }
    }
}
