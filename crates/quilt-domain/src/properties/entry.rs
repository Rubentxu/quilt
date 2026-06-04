//! Property entry traits — OCP/ISP-compliant extension point for property values.
//!
//! ## Design rationale (see `docs/adr/drafts/DRAFT-solid-trait-extension-pattern.md`)
//!
//! The original `PropertyValue` enum (7 variants, 284 usage sites) is intentionally
//! NOT extended with an `updated_at` field. Instead, this module defines a small
//! hierarchy of segregated traits so new metadata dimensions (timestamp, version,
//! source authority, …) can be added by implementing a trait, not by modifying
//! `PropertyValue`.
//!
//! - [`HasValue`]: minimal — just the value. Clients that only read use this.
//! - [`HasTimestamp`]: just the timestamp. Clients that only track time use this.
//! - [`Mergeable`]: the LWW (last-write-wins) decision. Depends on `HasTimestamp`.
//! - [`PropertyEntry`]: composite trait — value + timestamp + mergeable.
//!
//! The blanket impl `impl<T: HasValue + HasTimestamp + Mergeable> PropertyEntry for T {}`
//! means any type that implements the three minimal traits automatically implements
//! `PropertyEntry`. This is the OCP extension point: V2 can add `VersionedPropertyEntry`
//! without touching `merge_properties` or `PropertyValue`.
//!
//! [`PropertyValue`] is provided with a blanket impl that returns `None` for the
//! timestamp — Block uses bare `HashMap<String, PropertyValue>` and that's a valid
//! `PropertyEntry` (its updates are not subject to LWW).

use crate::value_objects::PropertyValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimal abstraction for an entry that exposes a value.
///
/// Use this trait when the caller only needs to read the value (e.g. UI rendering).
pub trait HasValue {
    /// The value type this entry carries.
    type Value;
    /// Borrow the value.
    fn value(&self) -> &Self::Value;
}

/// Minimal abstraction for an entry that carries an optional timestamp.
///
/// Use this trait when the caller only needs to track when the entry was last updated
/// (e.g. CRDT merge, audit trail).
pub trait HasTimestamp {
    /// Optional last-update timestamp. `None` means the entry has no timestamp — it
    /// loses to any timestamped entry in an LWW merge.
    fn updated_at(&self) -> Option<DateTime<Utc>>;
    /// Set the last-update timestamp.
    fn set_updated_at(&mut self, ts: DateTime<Utc>);
}

/// Mergeable entries know how to decide whether a newer entry should replace them.
///
/// The default implementation is **last-write-wins (LWW) by timestamp**:
///
/// | self ts | other ts | decision         |
/// |---------|----------|------------------|
/// | Some(a) | Some(b)  | `b > a`          |
/// | None    | Some(_)  | replace          |
/// | Some(_) | None     | keep self        |
/// | None    | None     | keep self (tie)  |
///
/// Implementors can override `should_be_replaced_by` to provide custom merge logic
/// (e.g. version-based, actor-based). The default is the OCP-compliant baseline.
pub trait Mergeable: HasTimestamp {
    /// Returns `true` if `other` should replace `self` in a merge.
    fn should_be_replaced_by(&self, other: &Self) -> bool {
        match (self.updated_at(), other.updated_at()) {
            (Some(a), Some(b)) => b > a,
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (None, None) => false,
        }
    }
}

/// Composite entry: a value with a timestamp, subject to LWW merge.
///
/// Any type that implements the three minimal traits automatically implements
/// `PropertyEntry` via the blanket impl below. This is the OCP extension point.
pub trait PropertyEntry: HasValue + HasTimestamp + Mergeable {}

impl<T: HasValue + HasTimestamp + Mergeable> PropertyEntry for T {}

/// Default concrete implementation: a value with an optional timestamp.
///
/// This is what `Page.properties` uses. V2 can introduce `VersionedPropertyEntry<V>`
/// implementing the same [`PropertyEntry`] trait without touching [`merge_properties`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultPropertyEntry<V> {
    /// The value carried by this entry.
    pub value: V,
    /// Optional last-update timestamp. `#[serde(default)]` ensures backward
    /// compatibility with old JSON that has no `updated_at` field.
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
}

impl<V> DefaultPropertyEntry<V> {
    /// Create a new entry with no timestamp.
    pub fn new(value: V) -> Self {
        Self {
            value,
            updated_at: None,
        }
    }

    /// Create a new entry with a timestamp.
    pub fn with_timestamp(value: V, ts: DateTime<Utc>) -> Self {
        Self {
            value,
            updated_at: Some(ts),
        }
    }
}

impl<V> HasValue for DefaultPropertyEntry<V> {
    type Value = V;
    fn value(&self) -> &V {
        &self.value
    }
}

impl<V> HasTimestamp for DefaultPropertyEntry<V> {
    fn updated_at(&self) -> Option<DateTime<Utc>> {
        self.updated_at
    }
    fn set_updated_at(&mut self, ts: DateTime<Utc>) {
        self.updated_at = Some(ts);
    }
}

impl<V: Clone> Mergeable for DefaultPropertyEntry<V> {}

/// Blanket impl: `PropertyValue` is a valid bare entry with no timestamp.
///
/// Block uses `HashMap<String, PropertyValue>` and that map is itself a valid
/// `PropertyEntry` collection — Block's properties are not subject to LWW because
/// Block entities are not expected to merge across writers. The `set_updated_at`
/// method is a no-op because `PropertyValue` has no storage for a timestamp.
impl HasValue for PropertyValue {
    type Value = PropertyValue;
    fn value(&self) -> &PropertyValue {
        self
    }
}

impl HasTimestamp for PropertyValue {
    fn updated_at(&self) -> Option<DateTime<Utc>> {
        None
    }
    fn set_updated_at(&mut self, _ts: DateTime<Utc>) {
        // no-op: PropertyValue is intentionally not extended with a timestamp
        // field. Bare entries are not subject to LWW.
    }
}

impl Mergeable for PropertyValue {}

/// Helper to extract the values of a property map as a flat `HashMap<String, &V>`.
///
/// This is the bridge from `HashMap<String, DefaultPropertyEntry<PropertyValue>>`
/// (what `Page.properties` carries) to `HashMap<String, PropertyValue>` (what
/// Block uses and what serializes cleanly to JSON). It's a 3-line method, not
/// a structural problem.
pub fn flatten_values<E: PropertyEntry>(
    entries: &HashMap<String, E>,
) -> HashMap<String, &E::Value> {
    entries
        .iter()
        .map(|(k, v)| (k.clone(), v.value()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::PropertyValue;
    use chrono::TimeZone;

    fn ts(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(secs, 0).single().unwrap()
    }

    // ── HasValue / HasTimestamp / Mergeable for DefaultPropertyEntry<V> ──

    #[test]
    fn default_entry_new_has_no_timestamp() {
        let e = DefaultPropertyEntry::new(PropertyValue::string("v"));
        assert_eq!(e.value(), &PropertyValue::String("v".to_string()));
        assert!(e.updated_at().is_none());
    }

    #[test]
    fn default_entry_with_timestamp_carries_it() {
        let t = ts(1000);
        let e = DefaultPropertyEntry::with_timestamp(PropertyValue::string("v"), t);
        assert_eq!(e.updated_at(), Some(t));
    }

    #[test]
    fn default_entry_set_updated_at_mutates() {
        let mut e = DefaultPropertyEntry::new(PropertyValue::string("v"));
        let t = ts(2000);
        e.set_updated_at(t);
        assert_eq!(e.updated_at(), Some(t));
    }

    #[test]
    fn default_entry_lww_later_wins() {
        let older = DefaultPropertyEntry::with_timestamp(PropertyValue::string("old"), ts(100));
        let newer = DefaultPropertyEntry::with_timestamp(PropertyValue::string("new"), ts(200));
        assert!(older.should_be_replaced_by(&newer));
        assert!(!newer.should_be_replaced_by(&older));
    }

    #[test]
    fn default_entry_lww_tied_timestamps_keep_self() {
        let a = DefaultPropertyEntry::with_timestamp(PropertyValue::string("a"), ts(500));
        let b = DefaultPropertyEntry::with_timestamp(PropertyValue::string("b"), ts(500));
        // Equal timestamps → keep self (deterministic tie-break).
        assert!(!a.should_be_replaced_by(&b));
        assert!(!b.should_be_replaced_by(&a));
    }

    #[test]
    fn default_entry_lww_any_timestamp_beats_none() {
        let bare = DefaultPropertyEntry::new(PropertyValue::string("bare"));
        let ts_entry = DefaultPropertyEntry::with_timestamp(PropertyValue::string("ts"), ts(1));
        assert!(bare.should_be_replaced_by(&ts_entry));
        assert!(!ts_entry.should_be_replaced_by(&bare));
    }

    #[test]
    fn default_entry_lww_two_bares_keep_self() {
        let a = DefaultPropertyEntry::new(PropertyValue::string("a"));
        let b = DefaultPropertyEntry::new(PropertyValue::string("b"));
        assert!(!a.should_be_replaced_by(&b));
        assert!(!b.should_be_replaced_by(&a));
    }

    // ── Serde backward compat (F8 spec discovery #1) ──

    #[test]
    fn default_entry_deserializes_legacy_json_without_timestamp() {
        // The `#[serde(default)]` on `updated_at` means JSON without the
        // field deserializes to `None` (forward-compat with possible future
        // omission). The `value` field is required and uses PropertyValue's
        // externally-tagged representation: `{"String": "v"}`.
        let json = r#"{"value":{"String":"v"}}"#;
        let e: DefaultPropertyEntry<PropertyValue> = serde_json::from_str(json).unwrap();
        assert_eq!(e.value(), &PropertyValue::String("v".to_string()));
        assert!(e.updated_at().is_none());
    }

    #[test]
    fn default_entry_round_trips_with_timestamp() {
        let t = ts(1234);
        let original = DefaultPropertyEntry::with_timestamp(PropertyValue::string("v"), t);
        let json = serde_json::to_string(&original).unwrap();
        let restored: DefaultPropertyEntry<PropertyValue> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn default_entry_round_trips_without_timestamp() {
        let original = DefaultPropertyEntry::new(PropertyValue::integer(42));
        let json = serde_json::to_string(&original).unwrap();
        let restored: DefaultPropertyEntry<PropertyValue> = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
        assert!(restored.updated_at().is_none());
    }

    // ── Blanket impls: PropertyValue as bare entry ──

    #[test]
    fn property_value_is_bare_entry_with_no_timestamp() {
        let v = PropertyValue::string("bare");
        assert!(HasTimestamp::updated_at(&v).is_none());
    }

    #[test]
    fn property_value_set_timestamp_is_noop() {
        let mut v = PropertyValue::string("bare");
        v.set_updated_at(ts(999));
        // Still None — bare entries are immutable w.r.t. timestamps.
        assert!(HasTimestamp::updated_at(&v).is_none());
        // And the value is unchanged.
        assert_eq!(v, PropertyValue::String("bare".to_string()));
    }

    #[test]
    fn property_value_bare_pair_with_none_vs_some_replaces() {
        // For bare PropertyValue, should_be_replaced_by always returns false
        // (both have timestamp None → "keep self"). This documents the
        // intentional asymmetry: bare entries are immutable w.r.t. merge.
        let bare_a = PropertyValue::string("a");
        let bare_b = PropertyValue::string("b");
        assert!(!bare_a.should_be_replaced_by(&bare_b));
        assert!(!bare_b.should_be_replaced_by(&bare_a));
    }

    #[test]
    fn property_value_bare_pair_keeps_self() {
        let a = PropertyValue::string("a");
        let b = PropertyValue::string("b");
        assert!(!a.should_be_replaced_by(&b));
    }

    // ── flatten_values bridge helper ──

    #[test]
    fn flatten_values_strips_entry_wrapper() {
        let mut m: HashMap<String, DefaultPropertyEntry<PropertyValue>> = HashMap::new();
        m.insert(
            "k1".to_string(),
            DefaultPropertyEntry::new(PropertyValue::string("v1")),
        );
        m.insert(
            "k2".to_string(),
            DefaultPropertyEntry::new(PropertyValue::integer(42)),
        );
        let flat = flatten_values(&m);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat["k1"], &PropertyValue::String("v1".to_string()));
        assert_eq!(flat["k2"], &PropertyValue::Integer(42));
    }
}
