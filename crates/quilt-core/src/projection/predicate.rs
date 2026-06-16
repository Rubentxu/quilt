//! WASM property predicate — declarative filter conditions over block properties.
//!
//! Mirrors the server's `quilt_domain::projection::predicate::PropertyPredicate`
//! (slice #4) but operates on `serde_json::Value` instead of `PropertyValue`.
//! Used by the V1 projection contracts in [`crate::projection::contracts`].
//!
//! # Variant summary
//!
//! | Variant        | Checks |
//! |----------------|--------|
//! | `Equals`       | Property value equals the given JSON literal |
//! | `IsSet`        | Property key is present (even if null) |
//! | `IsOneOf`      | Property value is one of the given JSON literals |
//! | `MatchesRegex` | Property value (as string) matches the regex |
//! | `GreaterThan`  | Property value > threshold (number, string-lex, or string-date-iso) |
//! | `LessThan`     | Property value < threshold (same types) |
//! | `And`          | Both sub-predicates match (short-circuit) |
//! | `Or`           | At least one sub-predicate matches (short-circuit) |
//! | `Not`          | Sub-predicate does NOT match |
//!
//! All nine variants are supported in WASM. The V1 contracts only use
//! `Equals`, `IsSet`, `IsOneOf`, and `Or`; the others are provided for
//! future V2 contract expansion.

use serde::{Deserialize, Serialize};

/// A declarative predicate over block properties (operates on JSON values).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "args")]
pub enum WasmPropertyPredicate {
    /// Property value equals the given JSON literal exactly.
    Equals {
        key: String,
        value: serde_json::Value,
    },

    /// Property key is present in the block (even if the value is null).
    /// This is NOT equivalent to "non-empty": a key with `null` or `""` as
    /// value IS set.
    IsSet { key: String },

    /// Property value is one of the given JSON literals (type-aware).
    IsOneOf {
        key: String,
        values: Vec<serde_json::Value>,
    },

    /// Property value (coerced to string) matches the given regex pattern.
    /// Malformed patterns do NOT panic — they simply don't match.
    MatchesRegex { key: String, pattern: String },

    /// Property value is strictly greater than the threshold.
    /// Cross-type comparisons return `false`.
    GreaterThan {
        key: String,
        threshold: serde_json::Value,
    },

    /// Property value is strictly less than the threshold.
    /// Cross-type comparisons return `false`.
    LessThan {
        key: String,
        threshold: serde_json::Value,
    },

    /// Logical AND of two predicates (short-circuit evaluation).
    And(Box<WasmPropertyPredicate>, Box<WasmPropertyPredicate>),

    /// Logical OR of two predicates (short-circuit evaluation).
    Or(Box<WasmPropertyPredicate>, Box<WasmPropertyPredicate>),

    /// Logical NOT of a predicate.
    Not(Box<WasmPropertyPredicate>),
}

impl WasmPropertyPredicate {
    /// Evaluate this predicate against a block's properties map.
    ///
    /// Returns `true` if the predicate's condition is satisfied,
    /// `false` otherwise.
    ///
    /// `properties` is the block's `serde_json::Map<String, serde_json::Value>`
    /// (the value of `BlockDto.properties` when it's an object — which is
    /// always the case for V1 blocks).
    #[must_use]
    pub fn matches(&self, properties: &serde_json::Map<String, serde_json::Value>) -> bool {
        match self {
            WasmPropertyPredicate::Equals { key, value } => properties
                .get(key.as_str())
                .map_or(false, |bv| bv == value),

            WasmPropertyPredicate::IsSet { key } => properties.contains_key(key.as_str()),

            WasmPropertyPredicate::IsOneOf { key, values } => properties
                .get(key.as_str())
                .map_or(false, |bv| values.contains(bv)),

            WasmPropertyPredicate::MatchesRegex { key, pattern } => {
                properties.get(key.as_str()).map_or(false, |bv| {
                    let s: String = match bv {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    regex::Regex::new(pattern)
                        .map(|re| re.is_match(&s))
                        .unwrap_or(false)
                })
            }

            WasmPropertyPredicate::GreaterThan { key, threshold } => {
                greater_than(properties.get(key.as_str()), threshold)
            }

            WasmPropertyPredicate::LessThan { key, threshold } => {
                less_than(properties.get(key.as_str()), threshold)
            }

            WasmPropertyPredicate::And(lhs, rhs) => lhs.matches(properties) && rhs.matches(properties),

            WasmPropertyPredicate::Or(lhs, rhs) => lhs.matches(properties) || rhs.matches(properties),

            WasmPropertyPredicate::Not(inner) => !inner.matches(properties),
        }
    }
}

// ── Helpers: type-aware comparison ─────────────────────────────────────────

/// Compare two JSON values for GreaterThan. Cross-type comparisons return
/// `false`. Supported types: number (number), string (lexicographic or
/// ISO 8601 date — auto-detected).
fn greater_than(actual: Option<&serde_json::Value>, threshold: &serde_json::Value) -> bool {
    match (actual, threshold) {
        (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
            match (a.as_f64(), b.as_f64()) {
                (Some(av), Some(bv)) => av > bv,
                _ => false,
            }
        }
        (Some(serde_json::Value::String(a)), serde_json::Value::String(b)) => {
            // Try ISO 8601 date comparison first (alphabetical works for ISO)
            if is_iso8601(a) && is_iso8601(b) {
                a > b
            } else {
                // Fallback: lexicographic
                a > b
            }
        }
        _ => false,
    }
}

fn less_than(actual: Option<&serde_json::Value>, threshold: &serde_json::Value) -> bool {
    match (actual, threshold) {
        (Some(serde_json::Value::Number(a)), serde_json::Value::Number(b)) => {
            match (a.as_f64(), b.as_f64()) {
                (Some(av), Some(bv)) => av < bv,
                _ => false,
            }
        }
        (Some(serde_json::Value::String(a)), serde_json::Value::String(b)) => {
            if is_iso8601(a) && is_iso8601(b) {
                a < b
            } else {
                a < b
            }
        }
        _ => false,
    }
}

/// Best-effort ISO 8601 date detection (date or datetime).
/// We accept anything starting with YYYY-MM-DD.
fn is_iso8601(s: &str) -> bool {
    s.len() >= 10 && s.is_ascii() && {
        let bytes = s.as_bytes();
        bytes[4] == b'-' && bytes[7] == b'-'
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn props(pairs: &[(&str, Value)]) -> serde_json::Map<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    // ── Equals ────────────────────────────────────────────────────

    #[test]
    fn equals_matches_string() {
        let p = props(&[("type", json!("task"))]);
        let pred = WasmPropertyPredicate::Equals {
            key: "type".to_string(),
            value: json!("task"),
        };
        assert!(pred.matches(&p));
    }

    #[test]
    fn equals_rejects_type_mismatch() {
        let p = props(&[("count", json!(42))]);
        let pred = WasmPropertyPredicate::Equals {
            key: "count".to_string(),
            value: json!("42"),
        };
        assert!(!pred.matches(&p));
    }

    #[test]
    fn equals_rejects_missing_key() {
        let p = props(&[]);
        let pred = WasmPropertyPredicate::Equals {
            key: "type".to_string(),
            value: json!("task"),
        };
        assert!(!pred.matches(&p));
    }

    // ── IsSet ─────────────────────────────────────────────────────

    #[test]
    fn is_set_matches_empty_string() {
        let p = props(&[("status", json!(""))]);
        let pred = WasmPropertyPredicate::IsSet {
            key: "status".to_string(),
        };
        assert!(pred.matches(&p));
    }

    #[test]
    fn is_set_matches_null_value() {
        let p = props(&[("status", Value::Null)]);
        let pred = WasmPropertyPredicate::IsSet {
            key: "status".to_string(),
        };
        assert!(pred.matches(&p));
    }

    #[test]
    fn is_set_returns_false_only_when_absent() {
        let p = props(&[]);
        let pred = WasmPropertyPredicate::IsSet {
            key: "status".to_string(),
        };
        assert!(!pred.matches(&p));
    }

    // ── IsOneOf ───────────────────────────────────────────────────

    #[test]
    fn is_one_of_matches_value_in_set() {
        let p = props(&[("status", json!("done"))]);
        let pred = WasmPropertyPredicate::IsOneOf {
            key: "status".to_string(),
            values: vec![json!("todo"), json!("done"), json!("cancelled")],
        };
        assert!(pred.matches(&p));
    }

    #[test]
    fn is_one_of_empty_never_matches() {
        let p = props(&[("status", json!("wip"))]);
        let pred = WasmPropertyPredicate::IsOneOf {
            key: "status".to_string(),
            values: vec![],
        };
        assert!(!pred.matches(&p));
    }

    // ── MatchesRegex ──────────────────────────────────────────────

    #[test]
    fn matches_regex_anchored_substring() {
        let p = props(&[("name", json!("my-awesome-task"))]);
        let pred = WasmPropertyPredicate::MatchesRegex {
            key: "name".to_string(),
            pattern: "awesome".to_string(),
        };
        assert!(pred.matches(&p));
    }

    #[test]
    fn matches_regex_rejects_non_string_via_to_string() {
        let p = props(&[("count", json!(42))]);
        let pred = WasmPropertyPredicate::MatchesRegex {
            key: "count".to_string(),
            pattern: "4".to_string(),
        };
        assert!(pred.matches(&p)); // "42" contains "4"
    }

    #[test]
    fn matches_regex_malformed_pattern_does_not_panic() {
        let p = props(&[("name", json!("test"))]);
        let pred = WasmPropertyPredicate::MatchesRegex {
            key: "name".to_string(),
            pattern: "[invalid".to_string(),
        };
        assert!(!pred.matches(&p));
    }

    // ── GreaterThan / LessThan ────────────────────────────────────

    #[test]
    fn gt_lt_numbers() {
        let p = props(&[("priority", json!(5))]);

        let gt = WasmPropertyPredicate::GreaterThan {
            key: "priority".to_string(),
            threshold: json!(3),
        };
        assert!(gt.matches(&p));

        let lt = WasmPropertyPredicate::LessThan {
            key: "priority".to_string(),
            threshold: json!(10),
        };
        assert!(lt.matches(&p));

        // Out of range
        let gt = WasmPropertyPredicate::GreaterThan {
            key: "priority".to_string(),
            threshold: json!(10),
        };
        assert!(!gt.matches(&p));
    }

    #[test]
    fn gt_dates_strings() {
        let p = props(&[("deadline", json!("2026-02-01T00:00:00Z"))]);
        let gt = WasmPropertyPredicate::GreaterThan {
            key: "deadline".to_string(),
            threshold: json!("2026-01-15T00:00:00Z"),
        };
        assert!(gt.matches(&p));
    }

    #[test]
    fn gt_rejects_cross_type() {
        let p = props(&[("count", json!(5))]);
        let gt = WasmPropertyPredicate::GreaterThan {
            key: "count".to_string(),
            threshold: json!("five"),
        };
        assert!(!gt.matches(&p));
    }

    // ── Combinators ──────────────────────────────────────────────

    #[test]
    fn and_short_circuits() {
        let p = props(&[("type", json!("task"))]);
        let pred = WasmPropertyPredicate::And(
            Box::new(WasmPropertyPredicate::Equals {
                key: "type".to_string(),
                value: json!("task"),
            }),
            Box::new(WasmPropertyPredicate::IsSet {
                key: "status".to_string(),
            }),
        );
        assert!(!pred.matches(&p)); // status is NOT set
    }

    #[test]
    fn or_short_circuits() {
        let p = props(&[("type", json!("task"))]);
        let pred = WasmPropertyPredicate::Or(
            Box::new(WasmPropertyPredicate::Equals {
                key: "type".to_string(),
                value: json!("task"),
            }),
            Box::new(WasmPropertyPredicate::IsSet {
                key: "status".to_string(),
            }),
        );
        assert!(pred.matches(&p)); // first branch matches
    }

    #[test]
    fn not_inverts() {
        let p = props(&[("type", json!("task"))]);
        let pred = WasmPropertyPredicate::Not(Box::new(WasmPropertyPredicate::Equals {
            key: "type".to_string(),
            value: json!("media"),
        }));
        assert!(pred.matches(&p));
    }

    // ── Serde ─────────────────────────────────────────────────────

    #[test]
    fn serde_tagged_round_trip_all_variants() {
        let cases = vec![
            WasmPropertyPredicate::Equals {
                key: "type".to_string(),
                value: json!("task"),
            },
            WasmPropertyPredicate::IsSet {
                key: "status".to_string(),
            },
            WasmPropertyPredicate::IsOneOf {
                key: "status".to_string(),
                values: vec![json!("todo"), json!("done")],
            },
            WasmPropertyPredicate::MatchesRegex {
                key: "name".to_string(),
                pattern: "task".to_string(),
            },
            WasmPropertyPredicate::GreaterThan {
                key: "count".to_string(),
                threshold: json!(5),
            },
            WasmPropertyPredicate::LessThan {
                key: "count".to_string(),
                threshold: json!(10),
            },
            WasmPropertyPredicate::And(
                Box::new(WasmPropertyPredicate::IsSet {
                    key: "a".to_string(),
                }),
                Box::new(WasmPropertyPredicate::IsSet {
                    key: "b".to_string(),
                }),
            ),
            WasmPropertyPredicate::Or(
                Box::new(WasmPropertyPredicate::IsSet {
                    key: "a".to_string(),
                }),
                Box::new(WasmPropertyPredicate::IsSet {
                    key: "b".to_string(),
                }),
            ),
            WasmPropertyPredicate::Not(Box::new(WasmPropertyPredicate::IsSet {
                key: "hidden".to_string(),
            })),
        ];

        for original in cases {
            let json = serde_json::to_string(&original).expect("serialize");
            let parsed: WasmPropertyPredicate = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(original, parsed, "round-trip failed for {original:?}");
        }
    }
}
