//! T-B.17: OCP-compliance proptest.
//!
//! This is the V2 forward-compatibility proof. The [`PropertyEntry`] trait
//! hierarchy (see `properties/entry.rs`) is the OCP extension point: V2 can
//! add new entry types (e.g. `VersionedPropertyEntry`, `SourcedPropertyEntry`)
//! by implementing the trait, without touching `merge_properties`.
//!
//! This test defines a `VersionedPropertyEntry<V>` that adds a `version` field
//! and a custom merge policy (higher version wins, tie-break by timestamp).
//! It then runs `merge_properties<VersionedPropertyEntry<PropertyValue>>` through
//! a proptest and asserts:
//!
//! 1. The function accepts the new entry type without modification.
//! 2. The custom merge policy is honored (higher version wins, not timestamp).
//! 3. The merge is still deterministic (same inputs → same outputs).
//!
//! If `merge_properties` is OCP-compliant, this test passes. If it isn't (e.g.
//! if `merge_properties` were specialized to `DefaultPropertyEntry`), the
//! compile would fail at the type signature — which is exactly the proof.
//!
//! See `docs/adr/drafts/DRAFT-solid-trait-extension-pattern.md` for the
//! canonical pattern.

use chrono::{DateTime, TimeZone, Utc};
use proptest::prelude::*;
use quilt_domain::properties::entry::{DefaultPropertyEntry, HasTimestamp, HasValue, Mergeable};
use quilt_domain::properties::merge::merge_properties;
use quilt_domain::value_objects::PropertyValue;

/// V2-style entry: adds a `version: u64` dimension alongside the timestamp.
/// Demonstrates that the trait hierarchy accepts new metadata dimensions
/// without modification to `merge_properties` or `PropertyValue`.
#[derive(Debug, Clone, PartialEq)]
pub struct VersionedPropertyEntry<V> {
    pub value: V,
    pub version: u64,
    pub updated_at: DateTime<Utc>,
}

impl<V> VersionedPropertyEntry<V> {
    pub fn new(value: V, version: u64, updated_at: DateTime<Utc>) -> Self {
        Self {
            value,
            version,
            updated_at,
        }
    }
}

impl<V> HasValue for VersionedPropertyEntry<V> {
    type Value = V;
    fn value(&self) -> &V {
        &self.value
    }
}

impl<V> HasTimestamp for VersionedPropertyEntry<V> {
    fn updated_at(&self) -> Option<DateTime<Utc>> {
        Some(self.updated_at)
    }
    fn set_updated_at(&mut self, ts: DateTime<Utc>) {
        self.updated_at = ts;
    }
}

/// Custom merge policy: higher `version` wins; tie-break by timestamp.
/// This OVERRIDES the default LWW-by-timestamp policy, proving that
/// implementors control their own merge semantics.
impl<V> Mergeable for VersionedPropertyEntry<V> {
    fn should_be_replaced_by(&self, other: &Self) -> bool {
        if other.version > self.version {
            return true; // newer version always wins
        }
        if other.version < self.version {
            return false; // older version always loses
        }
        // Equal versions → tie-break by timestamp.
        other.updated_at > self.updated_at
    }
}

// Note: no explicit `impl PropertyEntry for VersionedPropertyEntry` here —
// the blanket impl in `properties/entry.rs`
//   `impl<T: HasValue + HasTimestamp + Mergeable> PropertyEntry for T`
// covers it automatically. This is part of the OCP proof: V2 implementors
// only need to write the three minimal trait impls, and the composite is
// derived.

proptest! {
    /// OCP-compliance test: merge_properties works on VersionedPropertyEntry
    /// without any modification to the function itself. The compile error
    /// would catch a violation (e.g. if merge_properties were specialized to
    /// `DefaultPropertyEntry`).
    #[test]
    fn merge_properties_accepts_versioned_entries(
        existing in proptest::collection::hash_map(
            proptest::string::string_regex("[a-z]{1,3}").expect("valid regex"),
            (0u64..1000, 0i64..10000).prop_map(|(v, ts)| {
                let t = Utc.timestamp_opt(ts, 0).single().unwrap();
                VersionedPropertyEntry::new(PropertyValue::integer(v as i64), v, t)
            }),
            0..5,
        ),
        incoming in proptest::collection::hash_map(
            proptest::string::string_regex("[a-z]{1,3}").expect("valid regex"),
            (0u64..1000, 0i64..10000).prop_map(|(v, ts)| {
                let t = Utc.timestamp_opt(ts, 0).single().unwrap();
                VersionedPropertyEntry::new(PropertyValue::integer(v as i64), v, t)
            }),
            0..5,
        ),
    ) {
        // Compile-time proof: merge_properties<E: PropertyEntry + Clone>
        // accepts VersionedPropertyEntry<PropertyValue> (any PropertyEntry).
        let merged: std::collections::HashMap<String, VersionedPropertyEntry<PropertyValue>> =
            merge_properties(&existing, incoming.clone());

        // Distinct keys: all preserved.
        for (k, v) in &existing {
            if !incoming.contains_key(k) {
                prop_assert_eq!(merged.get(k).unwrap(), v);
            }
        }
        for (k, v) in &incoming {
            if !existing.contains_key(k) {
                prop_assert_eq!(merged.get(k).unwrap(), v);
            }
        }

        // Same key: incoming wins if its version > existing's version
        // (this is the custom policy, not the default LWW).
        for (k, vi) in &incoming {
            if let Some(ve) = existing.get(k) {
                let chosen = merged.get(k).unwrap();
                if vi.version > ve.version {
                    prop_assert_eq!(chosen, vi);
                } else if vi.version < ve.version {
                    prop_assert_eq!(chosen, ve);
                } else {
                    // Equal versions → tie-break by timestamp.
                    if vi.updated_at > ve.updated_at {
                        prop_assert_eq!(chosen, vi);
                    } else {
                        prop_assert_eq!(chosen, ve);
                    }
                }
            }
        }
    }

    /// Determinism: same inputs → same output.
    #[test]
    fn merge_versioned_is_deterministic(
        existing in proptest::collection::hash_map(
            proptest::string::string_regex("[a-z]{1,3}").expect("valid regex"),
            (0u64..100, 0i64..1000).prop_map(|(v, ts)| {
                let t = Utc.timestamp_opt(ts, 0).single().unwrap();
                VersionedPropertyEntry::new(PropertyValue::integer(v as i64), v, t)
            }),
            0..3,
        ),
        incoming in proptest::collection::hash_map(
            proptest::string::string_regex("[a-z]{1,3}").expect("valid regex"),
            (0u64..100, 0i64..1000).prop_map(|(v, ts)| {
                let t = Utc.timestamp_opt(ts, 0).single().unwrap();
                VersionedPropertyEntry::new(PropertyValue::integer(v as i64), v, t)
            }),
            0..3,
        ),
    ) {
        let m1: std::collections::HashMap<_, _> = merge_properties(&existing, incoming.clone());
        let m2 = merge_properties(&existing, incoming);
        prop_assert_eq!(m1.len(), m2.len());
        for (k, v) in &m1 {
            prop_assert_eq!(m2.get(k).unwrap(), v);
        }
    }

    /// Cross-type sanity: DefaultPropertyEntry and VersionedPropertyEntry
    /// are NOT substitutable for each other (they have different Value types
    /// and merge policies). This documents the boundary: same trait, different
    /// implementations → not interchangeable.
    #[test]
    fn default_and_versioned_use_distinct_policies(
        ts in 0i64..1000,
    ) {
        let t = Utc.timestamp_opt(ts, 0).single().unwrap();
        // DefaultPropertyEntry: tie-break on equal timestamps → keep self.
        let d_old = DefaultPropertyEntry::with_timestamp(PropertyValue::string("old"), t);
        let d_new = DefaultPropertyEntry::with_timestamp(PropertyValue::string("new"), t);
        prop_assert!(!d_old.should_be_replaced_by(&d_new));

        // VersionedPropertyEntry: tie-break on equal versions, but here
        // v_old has version 0 and v_new has version 1 → new wins.
        let v_old = VersionedPropertyEntry::new(PropertyValue::string("old"), 0, t);
        let v_new = VersionedPropertyEntry::new(PropertyValue::string("new"), 1, t);
        prop_assert!(v_old.should_be_replaced_by(&v_new));
    }
}
