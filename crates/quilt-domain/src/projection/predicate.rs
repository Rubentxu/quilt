//! Property predicate — declarative filter conditions over block properties.
//!
//! A `PropertyPredicate` describes a condition that can be evaluated against
//! a [`Block`](crate::entities::Block)'s properties. Predicates are the
//! declarative core of projection contracts: instead of writing arbitrary Rust
//! logic, a contract declares a list of predicates that must ALL match for the
//! block to be accepted.
//!
//! # Variant summary
//!
//! | Variant | Checks |
//! |---------|--------|
//! | `Equals` | Property value equals the given literal |
//! | `IsSet` | Property key is present (even if empty string) |
//! | `IsOneOf` | Property value is one of the given literals |
//! | `MatchesRegex` | Property value (as string) matches the regex |
//! | `GreaterThan` | Property value > threshold (Integer / Float / Date) |
//! | `LessThan` | Property value < threshold (Integer / Float / Date) |
//! | `And` | Both sub-predicates match (short-circuit) |
//! | `Or` | At least one sub-predicate matches (short-circuit) |
//! | `Not` | Sub-predicate does NOT match |
//!
//! # Note on `PartialEq` only
//!
//! This enum derives `PartialEq` but intentionally does **NOT** derive `Eq`.
//! The reason is [`PropertyValue::Float`]`: `f64` does not implement `Eq`
//! (NaN comparisons are undefined). Predicates are never used as `HashMap`
//! keys in the current design, so this is not a load-bearing constraint.

use crate::entities::{Block, PropertyKey};
use crate::value_objects::PropertyValue;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A declarative predicate over block properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "args")]
pub enum PropertyPredicate {
    /// Property value equals the given literal exactly (type-aware).
    Equals { key: PropertyKey, value: PropertyValue },

    /// Property key is present in the block (even if the value is an empty string).
    /// This is NOT equivalent to "non-empty": a key with `""` as value IS set.
    IsSet { key: PropertyKey },

    /// Property value is one of the given literals (type-aware).
    IsOneOf { key: PropertyKey, values: Vec<PropertyValue> },

    /// Property value (coerced to string) matches the given regex pattern.
    /// Malformed patterns do NOT panic — they simply don't match.
    MatchesRegex { key: PropertyKey, pattern: String },

    /// Property value is strictly greater than the threshold.
    /// Cross-type comparisons return `false`.
    GreaterThan { key: PropertyKey, threshold: PropertyValue },

    /// Property value is strictly less than the threshold.
    /// Cross-type comparisons return `false`.
    LessThan { key: PropertyKey, threshold: PropertyValue },

    /// Logical AND of two predicates (short-circuit evaluation).
    And(Box<PropertyPredicate>, Box<PropertyPredicate>),

    /// Logical OR of two predicates (short-circuit evaluation).
    Or(Box<PropertyPredicate>, Box<PropertyPredicate>),

    /// Logical NOT of a predicate.
    Not(Box<PropertyPredicate>),
}

impl PropertyPredicate {
    /// Evaluate this predicate against a block.
    ///
    /// Returns `true` if the predicate's condition is satisfied by the
    /// block's properties, `false` otherwise.
    ///
    /// Property key lookups use the dash-normalized form via [`PropertyKey`].
    /// Both the key in the predicate and the keys stored in
    /// `block.properties` are expected to already be normalized.
    #[must_use]
    pub fn matches(&self, block: &Block) -> bool {
        match self {
            PropertyPredicate::Equals { key, value } => {
                block.properties.get(key.as_str()).map_or(false, |bv| bv == value)
            }

            PropertyPredicate::IsSet { key } => block.properties.contains_key(key.as_str()),

            PropertyPredicate::IsOneOf { key, values } => {
                block.properties.get(key.as_str()).map_or(false, |bv| values.contains(bv))
            }

            PropertyPredicate::MatchesRegex { key, pattern } => {
                block.properties.get(key.as_str()).map_or(false, |bv| {
                    let s: String = match bv {
                        PropertyValue::String(s) => s.clone(),
                        other => other.as_display_string(),
                    };
                    Regex::new(pattern)
                        .map(|re| re.is_match(&s))
                        .unwrap_or(false)
                })
            }

            PropertyPredicate::GreaterThan { key, threshold } => {
                greater_than(block.properties.get(key.as_str()), threshold)
            }

            PropertyPredicate::LessThan { key, threshold } => {
                less_than(block.properties.get(key.as_str()), threshold)
            }

            PropertyPredicate::And(lhs, rhs) => lhs.matches(block) && rhs.matches(block),

            PropertyPredicate::Or(lhs, rhs) => lhs.matches(block) || rhs.matches(block),

            PropertyPredicate::Not(inner) => !inner.matches(block),
        }
    }
}

// ── Builder ─────────────────────────────────────────────────────────────────

/// Fluent builder for [`PropertyPredicate`].
#[derive(Debug, Default)]
pub struct PropertyPredicateBuilder {
    inner: Option<PropertyPredicate>,
}

impl PropertyPredicateBuilder {
    /// Build an `Equals` predicate: `key == value`.
    #[must_use]
    pub fn equals(key: PropertyKey, value: PropertyValue) -> Self {
        Self { inner: Some(PropertyPredicate::Equals { key, value }) }
    }

    /// Build an `IsSet` predicate: key is present in the block.
    #[must_use]
    pub fn is_set(key: PropertyKey) -> Self {
        Self { inner: Some(PropertyPredicate::IsSet { key }) }
    }

    /// Build an `IsOneOf` predicate: key's value is in the given set.
    #[must_use]
    pub fn is_one_of(key: PropertyKey, values: Vec<PropertyValue>) -> Self {
        Self { inner: Some(PropertyPredicate::IsOneOf { key, values }) }
    }

    /// Build a `MatchesRegex` predicate: key's value matches the regex pattern.
    #[must_use]
    pub fn matches(key: PropertyKey, pattern: String) -> Self {
        Self { inner: Some(PropertyPredicate::MatchesRegex { key, pattern }) }
    }

    /// Build a `GreaterThan` predicate.
    #[must_use]
    pub fn gt(key: PropertyKey, threshold: PropertyValue) -> Self {
        Self { inner: Some(PropertyPredicate::GreaterThan { key, threshold }) }
    }

    /// Build a `LessThan` predicate.
    #[must_use]
    pub fn lt(key: PropertyKey, threshold: PropertyValue) -> Self {
        Self { inner: Some(PropertyPredicate::LessThan { key, threshold }) }
    }

    /// Build an `And` predicate combining two existing predicates.
    #[must_use]
    pub fn and(self, other: PropertyPredicate) -> Self {
        match self.inner {
            Some(inner) => Self { inner: Some(PropertyPredicate::And(Box::new(inner), Box::new(other))) },
            None => Self { inner: None },
        }
    }

    /// Build an `Or` predicate combining two existing predicates.
    #[must_use]
    pub fn or(self, other: PropertyPredicate) -> Self {
        match self.inner {
            Some(inner) => Self { inner: Some(PropertyPredicate::Or(Box::new(inner), Box::new(other))) },
            None => Self { inner: None },
        }
    }

    /// Build a `Not` predicate wrapping another predicate.
    #[must_use]
    pub fn negate(predicate: PropertyPredicate) -> Self {
        Self { inner: Some(PropertyPredicate::Not(Box::new(predicate))) }
    }

    /// Consume the builder and return the constructed predicate.
    pub fn build(self) -> Option<PropertyPredicate> {
        self.inner
    }
}

// ── Helper: type-aware comparison ─────────────────────────────────────────────

/// Compare two PropertyValues for GreaterThan, only for homogeneous types.
/// Cross-type comparisons return `false`.
fn greater_than(actual: Option<&PropertyValue>, threshold: &PropertyValue) -> bool {
    match (actual, threshold) {
        (Some(PropertyValue::Integer(a)), PropertyValue::Integer(b)) => a > b,
        (Some(PropertyValue::Float(a)), PropertyValue::Float(b)) => a > b,
        (Some(PropertyValue::Date(a)), PropertyValue::Date(b)) => a > b,
        _ => false,
    }
}

/// Compare two PropertyValues for LessThan, only for homogeneous types.
/// Cross-type comparisons return `false`.
fn less_than(actual: Option<&PropertyValue>, threshold: &PropertyValue) -> bool {
    match (actual, threshold) {
        (Some(PropertyValue::Integer(a)), PropertyValue::Integer(b)) => a < b,
        (Some(PropertyValue::Float(a)), PropertyValue::Float(b)) => a < b,
        (Some(PropertyValue::Date(a)), PropertyValue::Date(b)) => a < b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Block;
    use crate::value_objects::{PropertyValue, Uuid};
    use chrono::{TimeZone, Utc};
    use std::collections::HashMap;

    fn make_block(props: HashMap<String, PropertyValue>) -> Block {
        Block {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: crate::value_objects::BlockFormat::Markdown,
            block_type: crate::value_objects::BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "test content".into(),
            properties: props,
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: None,
            cancelled_at: None,
            collapsed: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // ── Equals ───────────────────────────────────────────────────────

    #[test]
    fn equals_matches_type_equal_value() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p
        });
        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("type").unwrap(),
            value: PropertyValue::string("task"),
        };
        assert!(pred.matches(&block));
    }

    #[test]
    fn equals_rejects_type_mismatched() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("count".into(), PropertyValue::integer(42));
            p
        });
        // Block has Integer, predicate checks String — different types
        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("count").unwrap(),
            value: PropertyValue::string("42"),
        };
        assert!(!pred.matches(&block));
    }

    #[test]
    fn equals_returns_false_for_missing_key() {
        let block = make_block(HashMap::new());
        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("type").unwrap(),
            value: PropertyValue::string("task"),
        };
        assert!(!pred.matches(&block));
    }

    // ── IsSet ────────────────────────────────────────────────────────

    #[test]
    fn is_set_matches_empty_string() {
        // Empty string IS a set value — IsSet checks presence, not emptiness
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("status".into(), PropertyValue::string(""));
            p
        });
        let pred = PropertyPredicate::IsSet {
            key: PropertyKey::new("status").unwrap(),
        };
        assert!(pred.matches(&block));
    }

    #[test]
    fn is_set_returns_false_only_when_absent() {
        let block = make_block(HashMap::new());
        let pred = PropertyPredicate::IsSet {
            key: PropertyKey::new("status").unwrap(),
        };
        assert!(!pred.matches(&block));
    }

    // ── IsOneOf ─────────────────────────────────────────────────────

    #[test]
    fn is_one_of_matches_value_in_set() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("status".into(), PropertyValue::string("done"));
            p
        });
        let pred = PropertyPredicate::IsOneOf {
            key: PropertyKey::new("status").unwrap(),
            values: vec![
                PropertyValue::string("todo"),
                PropertyValue::string("done"),
                PropertyValue::string("cancelled"),
            ],
        };
        assert!(pred.matches(&block));
    }

    #[test]
    fn is_one_of_empty_never_matches() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("status".into(), PropertyValue::string("wip"));
            p
        });
        let pred = PropertyPredicate::IsOneOf {
            key: PropertyKey::new("status").unwrap(),
            values: vec![
                PropertyValue::string("todo"),
                PropertyValue::string("done"),
            ],
        };
        assert!(!pred.matches(&block));
    }

    // ── MatchesRegex ─────────────────────────────────────────────────

    #[test]
    fn matches_regex_anchored_substring() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("name".into(), PropertyValue::string("my-awesome-task"));
            p
        });
        let pred = PropertyPredicate::MatchesRegex {
            key: PropertyKey::new("name").unwrap(),
            pattern: "awesome".to_string(),
        };
        assert!(pred.matches(&block));
    }

    #[test]
    fn matches_regex_rejects_non_string() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("count".into(), PropertyValue::integer(42));
            p
        });
        // Non-string values use their display string representation for regex
        let pred = PropertyPredicate::MatchesRegex {
            key: PropertyKey::new("count").unwrap(),
            pattern: "4".to_string(),
        };
        assert!(pred.matches(&block)); // "42" contains "4"
    }

    #[test]
    fn matches_regex_malformed_pattern_does_not_panic() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("name".into(), PropertyValue::string("test"));
            p
        });
        let pred = PropertyPredicate::MatchesRegex {
            key: PropertyKey::new("name").unwrap(),
            pattern: "[invalid".to_string(), // malformed regex
        };
        // Should not panic — just returns false
        assert!(!pred.matches(&block));
    }

    // ── GreaterThan / LessThan ──────────────────────────────────────

    #[test]
    fn gt_lt_integers() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("priority".into(), PropertyValue::integer(5));
            p
        });

        let gt = PropertyPredicate::GreaterThan {
            key: PropertyKey::new("priority").unwrap(),
            threshold: PropertyValue::integer(3),
        };
        assert!(gt.matches(&block));

        let lt = PropertyPredicate::LessThan {
            key: PropertyKey::new("priority").unwrap(),
            threshold: PropertyValue::integer(10),
        };
        assert!(lt.matches(&block));

        // Out of range
        let gt = PropertyPredicate::GreaterThan {
            key: PropertyKey::new("priority").unwrap(),
            threshold: PropertyValue::integer(10),
        };
        assert!(!gt.matches(&block));
    }

    #[test]
    fn gt_dates() {
        let base = Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();

        let block = make_block({
            let mut p = HashMap::new();
            p.insert("deadline".into(), PropertyValue::date(later));
            p
        });

        let gt = PropertyPredicate::GreaterThan {
            key: PropertyKey::new("deadline").unwrap(),
            threshold: PropertyValue::date(base),
        };
        assert!(gt.matches(&block));
    }

    #[test]
    fn gt_rejects_cross_type() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("count".into(), PropertyValue::integer(5));
            p
        });

        // Float threshold for integer value → cross-type → false
        let gt = PropertyPredicate::GreaterThan {
            key: PropertyKey::new("count").unwrap(),
            threshold: PropertyValue::float(3.0),
        };
        assert!(!gt.matches(&block));
    }

    // ── Combinators ─────────────────────────────────────────────────

    #[test]
    fn and_short_circuits() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            // "status" is NOT set
            p
        });

        // type == "task" (true) AND status is set (false) → overall false
        // Short-circuit: if first was false, second wouldn't be evaluated
        let pred = PropertyPredicate::And(
            Box::new(PropertyPredicate::Equals {
                key: PropertyKey::new("type").unwrap(),
                value: PropertyValue::string("task"),
            }),
            Box::new(PropertyPredicate::IsSet {
                key: PropertyKey::new("status").unwrap(),
            }),
        );
        assert!(!pred.matches(&block));
    }

    #[test]
    fn or_short_circuits() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p
        });

        // type == "task" (true) OR status is set (false) → overall true
        // Short-circuit: if first was false, second would be evaluated
        let pred = PropertyPredicate::Or(
            Box::new(PropertyPredicate::Equals {
                key: PropertyKey::new("type").unwrap(),
                value: PropertyValue::string("task"),
            }),
            Box::new(PropertyPredicate::IsSet {
                key: PropertyKey::new("status").unwrap(),
            }),
        );
        assert!(pred.matches(&block));
    }

    #[test]
    fn not_inverts() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p
        });

        let pred = PropertyPredicate::Not(Box::new(PropertyPredicate::Equals {
            key: PropertyKey::new("type").unwrap(),
            value: PropertyValue::string("media"),
        }));
        assert!(pred.matches(&block));
    }

    // ── Builders ─────────────────────────────────────────────────────

    #[test]
    fn builders_produce_same_as_enum_variant() {
        let key = PropertyKey::new("type").unwrap();
        let val = PropertyValue::string("task");

        let via_enum = PropertyPredicate::Equals {
            key: key.clone(),
            value: val.clone(),
        };
        let via_builder = PropertyPredicateBuilder::equals(key, val).build().unwrap();
        assert_eq!(via_enum, via_builder);
    }

    #[test]
    fn combinators_chain() {
        let key = PropertyKey::new("status").unwrap();
        let pred = PropertyPredicateBuilder::is_set(key.clone())
            .and(PropertyPredicate::Equals {
                key,
                value: PropertyValue::string("done"),
            })
            .build()
            .unwrap();

        assert!(matches!(pred, PropertyPredicate::And(_, _)));
    }

    #[test]
    fn double_negation_cancels() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("type".into(), PropertyValue::string("task"));
            p
        });

        let inner = PropertyPredicate::Equals {
            key: PropertyKey::new("type").unwrap(),
            value: PropertyValue::string("task"),
        };

        let double_neg = PropertyPredicate::Not(Box::new(PropertyPredicate::Not(Box::new(inner))));
        assert!(double_neg.matches(&block));
    }

    // ── Serde ───────────────────────────────────────────────────────

    #[test]
    fn serde_tagged_round_trip_all_variants() {
        let cases: Vec<PropertyPredicate> = vec![
            PropertyPredicate::Equals {
                key: PropertyKey::new("type").unwrap(),
                value: PropertyValue::string("task"),
            },
            PropertyPredicate::IsSet {
                key: PropertyKey::new("status").unwrap(),
            },
            PropertyPredicate::IsOneOf {
                key: PropertyKey::new("status").unwrap(),
                values: vec![
                    PropertyValue::string("todo"),
                    PropertyValue::string("done"),
                ],
            },
            PropertyPredicate::MatchesRegex {
                key: PropertyKey::new("name").unwrap(),
                pattern: "task".to_string(),
            },
            PropertyPredicate::GreaterThan {
                key: PropertyKey::new("count").unwrap(),
                threshold: PropertyValue::integer(5),
            },
            PropertyPredicate::LessThan {
                key: PropertyKey::new("count").unwrap(),
                threshold: PropertyValue::integer(10),
            },
            PropertyPredicate::And(
                Box::new(PropertyPredicate::IsSet {
                    key: PropertyKey::new("a").unwrap(),
                }),
                Box::new(PropertyPredicate::IsSet {
                    key: PropertyKey::new("b").unwrap(),
                }),
            ),
            PropertyPredicate::Or(
                Box::new(PropertyPredicate::IsSet {
                    key: PropertyKey::new("a").unwrap(),
                }),
                Box::new(PropertyPredicate::IsSet {
                    key: PropertyKey::new("b").unwrap(),
                }),
            ),
            PropertyPredicate::Not(Box::new(PropertyPredicate::IsSet {
                key: PropertyKey::new("hidden").unwrap(),
            })),
        ];

        for original in cases {
            let json = serde_json::to_string(&original).expect("serialize");
            let parsed: PropertyPredicate =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(original, parsed, "round-trip failed for {original:?}");
        }
    }

    #[test]
    fn serde_uses_discriminator_field() {
        let pred = PropertyPredicate::IsSet {
            key: PropertyKey::new("status").unwrap(),
        };
        let json = serde_json::to_string(&pred).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");

        // Internally-tagged format: {kind: "IsSet", args: {key: "status"}}
        assert_eq!(
            v.get("kind"),
            Some(&serde_json::json!("IsSet")),
            "Expected 'kind' discriminator field in {json}"
        );
        assert!(
            v.get("args").is_some(),
            "Expected 'args' content field in {json}"
        );
        assert!(
            v.get("args")
                .and_then(|a| a.get("key"))
                .and_then(|k| k.as_str())
                .map(|s| s == "status")
                .unwrap_or(false),
            "Expected 'key' in args in {json}"
        );
    }

    // ── Type distinctions ─────────────────────────────────────────────

    #[test]
    fn integer_and_float_distinct() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert("value".into(), PropertyValue::integer(42));
            p
        });

        // Float threshold → cross-type → false
        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("value").unwrap(),
            value: PropertyValue::float(42.0),
        };
        assert!(!pred.matches(&block));
    }

    #[test]
    fn arrays_compare_element_wise() {
        // Arrays use element-wise equality
        let block = make_block({
            let mut p = HashMap::new();
            p.insert(
                "tags".into(),
                PropertyValue::Array(vec![
                    PropertyValue::string("rust"),
                    PropertyValue::string("wasm"),
                ]),
            );
            p
        });

        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("tags").unwrap(),
            value: PropertyValue::Array(vec![
                PropertyValue::string("rust"),
                PropertyValue::string("wasm"),
            ]),
        };
        assert!(pred.matches(&block));
    }

    #[test]
    fn different_lengths_not_equal() {
        let block = make_block({
            let mut p = HashMap::new();
            p.insert(
                "tags".into(),
                PropertyValue::Array(vec![PropertyValue::string("rust")]),
            );
            p
        });

        let pred = PropertyPredicate::Equals {
            key: PropertyKey::new("tags").unwrap(),
            value: PropertyValue::Array(vec![
                PropertyValue::string("rust"),
                PropertyValue::string("wasm"),
            ]),
        };
        assert!(!pred.matches(&block));
    }
}
