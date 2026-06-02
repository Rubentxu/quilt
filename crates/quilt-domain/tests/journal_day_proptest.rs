//! Property-based tests for the `JournalDay` value object.
//!
//! `JournalDay` represents a calendar day in `YYYY-MM-DD` form, stored
//! internally as `i32` for efficient indexing. These properties verify
//! that the parsing, formatting, ordering, and arithmetic invariants
//! hold for any valid calendar date.

use proptest::prelude::*;
use std::str::FromStr;
use quilt_domain::JournalDay;

/// Build a JournalDay from safe (year, month, day) where every
/// triple is a valid calendar date. We restrict the day to <= 28 to
/// avoid month-end edge cases (Feb 30 etc. would never construct).
fn make_day(year: u16, month: u8, day: u8) -> JournalDay {
    JournalDay::from_ymd(year, month, day).expect("from_ymd should accept month 1-12, day 1-28")
}

proptest! {
    /// Property: `to_string` -> `from_str` is the identity for any valid
    /// date. The ISO format `YYYY-MM-DD` is the canonical string form.
    #[test]
    fn string_roundtrip(
        year in 1970u16..2100u16,
        month in 1u8..=12u8,
        day in 1u8..=28u8
    ) {
        let original = make_day(year, month, day);
        let s = original.to_string();
        let parsed = JournalDay::from_str(&s).expect("ISO string must roundtrip");
        prop_assert_eq!(original, parsed);
    }

    /// Property: parsing the displayed string is case-insensitive and
    /// whitespace-tolerant (the parser trims).
    #[test]
    fn from_str_trims_whitespace(
        year in 1970u16..2100u16,
        month in 1u8..=12u8,
        day in 1u8..=28u8,
        leading in "[ \\t]{0,5}",
        trailing in "[ \\t]{0,5}",
    ) {
        let original = make_day(year, month, day);
        let s = format!("{}{}{}", leading, original, trailing);
        let parsed = JournalDay::from_str(&s)
            .expect("parser must accept leading/trailing whitespace");
        prop_assert_eq!(original, parsed);
    }

    /// Property: month values outside 1..=12 are rejected.
    #[test]
    fn invalid_month_rejected(
        year in 1970u16..2100u16,
        day in 1u8..=28u8
    ) {
        prop_assert!(JournalDay::from_ymd(year, 0, day).is_none());
        prop_assert!(JournalDay::from_ymd(year, 13, day).is_none());
        prop_assert!(JournalDay::from_ymd(year, 255, day).is_none());
    }

    /// Property: day values outside 1..=31 are rejected.
    /// We use 32 and 0 to exercise the bounds check.
    #[test]
    fn invalid_day_rejected(
        year in 1970u16..2100u16,
        month in 1u8..=12u8
    ) {
        prop_assert!(JournalDay::from_ymd(year, month, 0).is_none());
        prop_assert!(JournalDay::from_ymd(year, month, 32).is_none());
    }

    /// Property: ordering is lexicographically consistent with the
    /// year/month/day components. If year A < year B, then A < B.
    /// We compare via `as_i32()` because `JournalDay`'s internal
    /// `YYYYMMDD` representation is lexicographically ordered.
    #[test]
    fn ordering_consistent_by_year(
        y1 in 2000u16..2100u16,
        y2 in 2000u16..2100u16,
        m1 in 1u8..=12u8,
        m2 in 1u8..=12u8,
        d1 in 1u8..=28u8,
        d2 in 1u8..=28u8
    ) {
        let a = make_day(y1, m1, d1);
        let b = make_day(y2, m2, d2);
        let a_int = a.as_i32();
        let b_int = b.as_i32();
        if y1 < y2 {
            prop_assert!(a_int < b_int, "y1={} < y2={} but a >= b", y1, y2);
        }
        if y1 > y2 {
            prop_assert!(a_int > b_int, "y1={} > y2={} but a <= b", y1, y2);
        }
    }

    /// Property: equal dates compare as equal.
    #[test]
    fn equality_reflexive(
        year in 1970u16..2100u16,
        month in 1u8..=12u8,
        day in 1u8..=28u8
    ) {
        let a = make_day(year, month, day);
        let b = make_day(year, month, day);
        prop_assert_eq!(a, b);
    }

    /// Property: `add_days(0)` returns the same date.
    #[test]
    fn add_zero_is_identity(
        year in 1970u16..2100u16,
        month in 1u8..=12u8,
        day in 1u8..=28u8
    ) {
        let a = make_day(year, month, day);
        let b = a.add_days(0).expect("adding 0 days must succeed");
        prop_assert_eq!(a, b);
    }

    /// Property: `add_days(1)` followed by `add_days(-1)` returns the
    /// same date (roundtrip for adjacent days).
    #[test]
    fn add_one_then_sub_one_is_identity(
        year in 1970u16..2100u16,
        month in 1u8..=12u8,
        day in 1u8..=27u8
    ) {
        let a = make_day(year, month, day);
        let b = a.add_days(1).unwrap().add_days(-1).unwrap();
        prop_assert_eq!(a, b);
    }

    /// Property: `a - b == -(b - a)`. The Sub implementation is
    /// anti-symmetric.
    #[test]
    fn sub_is_antisymmetric(
        y1 in 2000u16..2100u16,
        y2 in 2000u16..2100u16,
        m1 in 1u8..=12u8,
        m2 in 1u8..=12u8,
        d1 in 1u8..=28u8,
        d2 in 1u8..=28u8
    ) {
        let a = make_day(y1, m1, d1);
        let b = make_day(y2, m2, d2);
        let ab = a - b;
        let ba = b - a;
        prop_assert_eq!(ab, -ba, "a-b={} but b-a={}", ab, ba);
    }

    /// Property: `days_between(a, b) == (a - b)`. The Sub impl and
    /// the days_between method must agree.
    #[test]
    fn sub_matches_days_between(
        y1 in 2000u16..2100u16,
        y2 in 2000u16..2100u16,
        m1 in 1u8..=12u8,
        m2 in 1u8..=12u8,
        d1 in 1u8..=28u8,
        d2 in 1u8..=28u8
    ) {
        let a = make_day(y1, m1, d1);
        let b = make_day(y2, m2, d2);
        prop_assert_eq!(a - b, a.days_between(&b));
    }
}
