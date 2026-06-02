//! Integration tests for Priority value object.
//!
//! Covers: ordering, Default, from_char, parse_str, as_char,
//! as_property_value, Display, all(), default_priority(), serde
//! roundtrip, and property-based from_char roundtrip.

use quilt_domain::value_objects::Priority;

// ── Ordering ──────────────────────────────────────────────────

#[test]
fn test_priority_ordering() {
    assert!(Priority::A < Priority::B);
    assert!(Priority::B < Priority::C);
    assert!(Priority::A < Priority::C);
}

#[test]
fn test_default_is_b() {
    assert_eq!(Priority::default(), Priority::B);
}

// ── from_char ─────────────────────────────────────────────────

#[test]
fn test_from_char_valid() {
    assert_eq!(Priority::from_char('A'), Some(Priority::A));
    assert_eq!(Priority::from_char('a'), Some(Priority::A));
    assert_eq!(Priority::from_char('B'), Some(Priority::B));
    assert_eq!(Priority::from_char('b'), Some(Priority::B));
    assert_eq!(Priority::from_char('C'), Some(Priority::C));
    assert_eq!(Priority::from_char('c'), Some(Priority::C));
}

#[test]
fn test_from_char_invalid() {
    assert_eq!(Priority::from_char('X'), None);
    assert_eq!(Priority::from_char('1'), None);
    assert_eq!(Priority::from_char(' '), None);
    assert_eq!(Priority::from_char('\0'), None);
}

// ── parse_str ─────────────────────────────────────────────────

#[test]
fn test_parse_str_valid() {
    assert_eq!(Priority::parse_str("A"), Some(Priority::A));
    assert_eq!(Priority::parse_str("B"), Some(Priority::B));
    assert_eq!(Priority::parse_str("C priority"), Some(Priority::C));
}

#[test]
fn test_parse_str_empty() {
    assert_eq!(Priority::parse_str(""), None);
}

#[test]
fn test_parse_str_invalid() {
    assert_eq!(Priority::parse_str("X"), None);
    assert_eq!(Priority::parse_str("1"), None);
}

// ── as_char / as_property_value ───────────────────────────────

#[test]
fn test_as_char() {
    assert_eq!(Priority::A.as_char(), 'A');
    assert_eq!(Priority::B.as_char(), 'B');
    assert_eq!(Priority::C.as_char(), 'C');
}

#[test]
fn test_as_property_value() {
    assert_eq!(Priority::A.as_property_value(), "A");
    assert_eq!(Priority::B.as_property_value(), "B");
    assert_eq!(Priority::C.as_property_value(), "C");
}

// ── Display ───────────────────────────────────────────────────

#[test]
fn test_display() {
    assert_eq!(format!("{}", Priority::A), "A");
    assert_eq!(format!("{}", Priority::B), "B");
    assert_eq!(format!("{}", Priority::C), "C");
}

// ── all() / default_priority() ────────────────────────────────

#[test]
fn test_all_returns_three_variants() {
    let all = Priority::all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&Priority::A));
    assert!(all.contains(&Priority::B));
    assert!(all.contains(&Priority::C));
}

#[test]
fn test_default_priority_is_b() {
    assert_eq!(Priority::default_priority(), Priority::B);
}

// ── Serde roundtrip ───────────────────────────────────────────

#[test]
fn test_serde_roundtrip_all_variants() {
    for p in Priority::all() {
        let json = serde_json::to_string(p).unwrap();
        let restored: Priority = serde_json::from_str(&json).unwrap();
        assert_eq!(*p, restored, "roundtrip failed for {:?}", p);
    }
}

#[test]
fn test_serde_rejects_unknown_variant() {
    assert!(serde_json::from_str::<Priority>("\"D\"").is_err());
    assert!(serde_json::from_str::<Priority>("\"Z\"").is_err());
}

#[test]
fn test_serde_rejects_invalid_json() {
    assert!(serde_json::from_str::<Priority>("1").is_err());
    assert!(serde_json::from_str::<Priority>("null").is_err());
    assert!(serde_json::from_str::<Priority>("[]").is_err());
}

// ── Property-based: from_char roundtrip ───────────────────────

#[test]
fn proptest_from_char_roundtrip() {
    use proptest::prelude::*;
    proptest!(|(c in "[ABCabc]")| {
        // proptest generates String from regex patterns
        let ch = c.chars().next().unwrap();
        let parsed = Priority::from_char(ch);
        assert!(parsed.is_some(), "failed to parse '{}'", ch);
        let roundtripped = parsed.unwrap().as_char().to_ascii_uppercase();
        assert_eq!(ch.to_ascii_uppercase(), roundtripped);
    });
}
