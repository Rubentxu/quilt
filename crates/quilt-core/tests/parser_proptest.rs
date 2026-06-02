//! Property-based tests for the inline parser.
//!
//! These tests verify invariants that should hold for ANY input string.
//! Proptest generates hundreds of random inputs and tries to find
//! counter-examples. If a test fails, proptest shrinks the input to
//! the minimal reproduction.

use proptest::prelude::*;
use quilt_core::parser::inline::{InlineParser, Range, Segment};

/// Helper: extract the byte range from any segment variant.
fn seg_range(seg: &Segment) -> &Range {
    match seg {
        Segment::Text { range, .. }
        | Segment::PageRef { range, .. }
        | Segment::BlockRef { range, .. }
        | Segment::Tag { range, .. }
        | Segment::Property { range, .. }
        | Segment::Bold { range, .. }
        | Segment::Italic { range, .. }
        | Segment::Code { range, .. }
        | Segment::Link { range, .. }
        | Segment::BoldItalic { range, .. }
        | Segment::Strikethrough { range, .. }
        | Segment::Highlight { range, .. }
        | Segment::Header { range, .. } => range,
    }
}

proptest! {
    /// Property: parsing any string never panics.
    /// The parser should be total — it must accept any `&str` and return
    /// a value, never unwind.
    #[test]
    fn parser_never_panics(s in ".*") {
        let _ = InlineParser::new().parse(&s);
    }

    /// Property: every segment's range is within the input bounds.
    /// A segment with `range.end > s.len()` would indicate an out-of-bounds
    /// bug in the parser.
    #[test]
    fn segments_within_input_bounds(s in ".*") {
        let parsed = InlineParser::new().parse(&s);
        for seg in &parsed.segments {
            let r = seg_range(seg);
            prop_assert!(
                r.start <= r.end,
                "reversed range in segment {:?}",
                seg
            );
            prop_assert!(
                r.end <= s.len(),
                "segment {:?} has end={} > input len={}",
                seg,
                r.end,
                s.len()
            );
        }
    }

    /// Property: total bytes consumed by all segments <= input length.
    /// This is implied by the bounds check above, but the aggregate form
    /// is a useful regression check.
    #[test]
    fn total_consumed_at_most_input(s in ".*") {
        let parsed = InlineParser::new().parse(&s);
        let total: usize = parsed
            .segments
            .iter()
            .map(|seg| {
                let r = seg_range(seg);
                r.end - r.start
            })
            .sum();
        prop_assert!(
            total <= s.len(),
            "segments consumed {} bytes but input is {} bytes",
            total,
            s.len()
        );
    }

    /// Property: segments don't overlap.
    /// For any two adjacent segments, the first's `end` must be <= the
    /// second's `start`. The parser advances position monotonically, so
    /// this should always hold.
    #[test]
    fn segments_dont_overlap(s in ".*") {
        let parsed = InlineParser::new().parse(&s);
        for i in 0..parsed.segments.len().saturating_sub(1) {
            let a = seg_range(&parsed.segments[i]);
            let b = seg_range(&parsed.segments[i + 1]);
            prop_assert!(
                a.end <= b.start,
                "segments overlap: a={}..{} and b={}..{} in input {:?}",
                a.start, a.end, b.start, b.end, s
            );
        }
    }

    /// Property: `**inner**` is recognized as Bold when `inner` has no `**`.
    /// Verifies the bold parser actually fires for valid input.
    #[test]
    fn bold_recognized(inner in "[a-zA-Z0-9 _-]{1,50}") {
        let input = format!("**{}**", inner);
        let parsed = InlineParser::new().parse(&input);
        let has_bold = parsed
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Bold { .. }));
        prop_assert!(has_bold, "input {:?} should contain a Bold segment", input);
    }

    /// Property: `[[name]]` is recognized as PageRef when name has no `]]`.
    /// Verifies the page ref parser fires for valid input.
    #[test]
    fn page_ref_recognized(name in "[a-zA-Z][a-zA-Z0-9 _-]{0,30}") {
        let input = format!("[[{}]]", name);
        let parsed = InlineParser::new().parse(&input);
        let has_page_ref = parsed
            .segments
            .iter()
            .any(|s| matches!(s, Segment::PageRef { .. }));
        prop_assert!(has_page_ref, "input {:?} should contain a PageRef", input);
    }

    /// Property: empty input produces an empty segment list.
    #[test]
    fn empty_input_empty_output(_dummy: ()) {
        let parsed = InlineParser::new().parse("");
        prop_assert!(parsed.segments.is_empty());
    }

    /// Property: parsing a string and parsing it again yields the same
    /// segments. The parser is pure.
    #[test]
    fn parser_is_pure(s in ".*") {
        let parser = InlineParser::new();
        let a = parser.parse(&s);
        let b = parser.parse(&s);
        prop_assert_eq!(a.segments.len(), b.segments.len());
        for (sa, sb) in a.segments.iter().zip(b.segments.iter()) {
            let ra = seg_range(sa);
            let rb = seg_range(sb);
            prop_assert_eq!(ra, rb);
        }
    }

    /// Property: parser survives any unicode input.
    /// Ranges use byte offsets, so the invariant is that all byte ranges
    /// stay within `s.as_bytes().len()` (== s.len()).
    #[test]
    fn unicode_input_within_bounds(s in "\\PC{0,100}") {
        let parsed = InlineParser::new().parse(&s);
        for seg in &parsed.segments {
            let r = seg_range(seg);
            prop_assert!(
                r.end <= s.len(),
                "unicode segment out of bounds: {:?} (end={}, len={})",
                seg,
                r.end,
                s.len()
            );
        }
    }
}
