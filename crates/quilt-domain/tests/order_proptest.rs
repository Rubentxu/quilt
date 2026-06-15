//! Property-based tests for the fractional ordering algorithm.
//!
//! `OrderCalculator` uses fractional indexing (midpoint-based) to assign
//! `order` values to blocks. Insertions between siblings should not
//! require re-indexing existing blocks. These properties verify that
//! the invariants of the algorithm hold for arbitrary input.

use proptest::prelude::*;
use quilt_domain::OrderCalculator;
use quilt_domain::entities::Block;
use quilt_domain::value_objects::{BlockFormat, BlockType, Uuid};

/// Build a Block with a controllable `order` value. All other fields are
/// filled with sensible defaults — the order algorithm only reads
/// `block.order`, so the other fields are irrelevant to the properties.
fn make_block(order: f64) -> Block {
    Block {
        id: Uuid::new_v4(),
        page_id: Uuid::new_v4(),
        parent_id: None,
        order,
        level: 1,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        marker: None,
        priority: None,
        content: format!("block@{}", order),
        properties: std::collections::HashMap::new(),
        refs: Vec::new(),
        tags: Vec::new(),
        scheduled: None,
        deadline: None,
        start_time: None,
        repeated: None,
        logbook: None,
        completed_at: None,
        cancelled_at: None,
        collapsed: false,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

proptest! {
    /// Property: `reindex(n)` produces exactly n values.
    #[test]
    fn reindex_count(n in 0usize..100) {
        let result = OrderCalculator::reindex(n);
        prop_assert_eq!(result.len(), n);
    }

    /// Property: `reindex(n)` produces strictly increasing values
    /// (i.e. (i+1).0 - i.0 == 1.0 for all i in 0..n-1).
    #[test]
    fn reindex_strictly_increasing(n in 1usize..100) {
        let result = OrderCalculator::reindex(n);
        for i in 0..result.len().saturating_sub(1) {
            prop_assert!(
                result[i + 1] > result[i],
                "reindex not strictly increasing at index {}: {:?}", i, result
            );
        }
    }

    /// Property: `reindex(n)` starts at 1.0 and ends at n as f64.
    #[test]
    fn reindex_endpoints(n in 1usize..100) {
        let result = OrderCalculator::reindex(n);
        prop_assert!((result[0] - 1.0).abs() < f64::EPSILON);
        prop_assert!((result[n - 1] - n as f64).abs() < f64::EPSILON);
    }

    /// Property: `reindex` produces values spaced exactly 1.0 apart.
    #[test]
    fn reindex_unit_spacing(n in 2usize..100) {
        let result = OrderCalculator::reindex(n);
        for i in 0..result.len() - 1 {
            let gap = result[i + 1] - result[i];
            prop_assert!(
                (gap - 1.0).abs() < f64::EPSILON,
                "gap between indices {} and {} was {}", i, i + 1, gap
            );
        }
    }

    /// Property: `insert_after(p, [siblings])` where p >= max(siblings)
    /// returns p + 1.0 (i.e. we append to the end).
    #[test]
    fn insert_after_appends_at_end(
        max_sibling in 1.0f64..1000.0,
        preceding_delta in 0.0f64..10.0
    ) {
        let preceding = max_sibling + preceding_delta;
        let siblings: Vec<Block> = (1..=3).map(|i| make_block(i as f64)).collect();
        let result = OrderCalculator::insert_after(preceding, &siblings).unwrap();
        prop_assert!(
            (result - (preceding + 1.0)).abs() < f64::EPSILON,
            "expected {}, got {}", preceding + 1.0, result
        );
    }

    /// Property: `insert_after(p, [siblings])` where p < min(siblings)
    /// returns a value strictly between p and the first sibling.
    #[test]
    fn insert_after_between_min_siblings(
        preceding in 0.0f64..10.0,
        first in 11.0f64..100.0
    ) {
        let siblings: Vec<Block> = (1..=3).map(|i| make_block(first + i as f64)).collect();
        let result = OrderCalculator::insert_after(preceding, &siblings).unwrap();
        prop_assert!(
            result > preceding,
            "result {} should be > preceding {}", result, preceding
        );
        prop_assert!(
            result < siblings[0].order,
            "result {} should be < first sibling {}", result, siblings[0].order
        );
    }

    /// Property: `insert_after(p, [siblings])` where p matches the i-th
    /// sibling and i+1 exists, returns a value between siblings[i] and
    /// siblings[i+1].
    #[test]
    fn insert_after_midpoint_between_adjacent(
        n in 3usize..10,
        target_idx in 0usize..3,
    ) {
        // Generate n siblings with orders 1.0, 2.0, ..., n
        let siblings: Vec<Block> = (1..=n).map(|i| make_block(i as f64)).collect();
        let target = target_idx.min(n - 2); // ensure siblings[target+1] exists
        let preceding = siblings[target].order;
        let next = siblings[target + 1].order;
        let result = OrderCalculator::insert_after(preceding, &siblings).unwrap();
        prop_assert!(
            result > preceding,
            "result {} should be > preceding {}", result, preceding
        );
        prop_assert!(
            result < next,
            "result {} should be < next {}", result, next
        );
    }

    /// Property: `insert_first([])` returns 1.0.
    #[test]
    fn insert_first_empty_returns_one(_dummy: ()) {
        let result = OrderCalculator::insert_first(&[]);
        prop_assert!((result - 1.0).abs() < f64::EPSILON);
    }

    /// Property: `insert_first(siblings)` returns a value strictly less
    /// than the minimum order of any sibling (when siblings is non-empty).
    #[test]
    fn insert_first_below_min(
        n in 1usize..20,
        offset in 1.0f64..100.0
    ) {
        let siblings: Vec<Block> =
            (1..=n).map(|i| make_block(offset + i as f64)).collect();
        let min_order = siblings
            .iter()
            .map(|b| b.order)
            .fold(f64::INFINITY, f64::min);
        let result = OrderCalculator::insert_first(&siblings);
        prop_assert!(
            result < min_order,
            "insert_first returned {} which is >= min {}", result, min_order
        );
    }

    /// Property: ordering is independent of input order. Siblings passed
    /// in any permutation should yield the same result.
    #[test]
    fn insert_after_robust_to_permutation(
        n in 3usize..10
    ) {
        // Use a preceding value strictly less than the max sibling but
        // not equal to any sibling — guarantees the midpoint path is hit.
        let siblings_ordered: Vec<Block> = (1..=n).map(|i| make_block(i as f64)).collect();
        let mut siblings_reversed = siblings_ordered.clone();
        siblings_reversed.reverse();

        let preceding = 1.5;
        let r_ordered = OrderCalculator::insert_after(preceding, &siblings_ordered).unwrap();
        let r_reversed = OrderCalculator::insert_after(preceding, &siblings_reversed).unwrap();
        prop_assert!(
            (r_ordered - r_reversed).abs() < 1e-6,
            "permutation changed result: ordered={} reversed={}",
            r_ordered,
            r_reversed
        );
        // Both should be the midpoint between 1.5 and 2.0
        prop_assert!(
            (r_ordered - 1.75).abs() < 1e-6,
            "expected ~1.75, got {}", r_ordered
        );
    }
}
