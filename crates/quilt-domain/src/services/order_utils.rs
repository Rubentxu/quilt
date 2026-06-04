//! Fractional ordering utilities for block siblings.
//!
//! Uses fractional indexing (midpoint-based) to assign `order` values
//! to blocks so that insertions between siblings don't require re-indexing
//! existing blocks.
//!
//! Inspired by Quilt's approach: each block gets an `f64` order value,
//! and inserting between two siblings calculates the midpoint. When the
//! gap between two adjacent blocks is exhausted (midpoint rounds to one
//! of the endpoints), a full re-index of all siblings is triggered.

use crate::entities::Block;
use std::cmp::Ordering;

/// Order calculator using fractional indexing.
///
/// # Fractional Indexing Algorithm
///
/// Each block has an `order: f64`. Siblings are sorted by `order`.
/// To insert after a given preceding block, we calculate the midpoint
/// between its order and the next sibling's order.
///
/// ```text
/// order: 1.0      2.0      3.0
///        |─── A ──|─── B ──|─── C
/// insert after B → midpoint(2.0, 3.0) = 2.5
/// ```
///
/// When the gap is exhausted (midpoint == preceding_order), we trigger
/// a full re-index with evenly spaced values.
pub struct OrderCalculator;

impl OrderCalculator {
    /// Calculate the order for a new block inserted after `preceding_block_id`.
    ///
    /// # Arguments
    ///
    /// * `preceding_order` - The `order` value of the block being inserted after.
    /// * `siblings` - All sibling blocks (same parent) sorted by `order`.
    ///
    /// # Returns
    ///
    /// * `Ok(f64)` - The order value for the new block.
    /// * `Err(String)` - If re-indexing is needed (gap exhausted) or no siblings.
    pub fn insert_after(preceding_order: f64, siblings: &[Block]) -> Result<f64, String> {
        Self::insert_after_raw(preceding_order, siblings.iter().map(|b| b.order))
    }

    /// Raw variant that operates on a sorted sequence of f64 values.
    /// Used internally and for testing.
    fn insert_after_raw(
        preceding_order: f64,
        sibling_orders: impl Iterator<Item = f64>,
    ) -> Result<f64, String> {
        // Collect sorted orders
        let mut orders: Vec<f64> = sibling_orders.collect();
        orders.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

        // Find the next sibling after preceding_order
        let next_order = orders.into_iter().find(|o| *o > preceding_order);

        match next_order {
            Some(next) => {
                // Calculate midpoint
                let mid = (preceding_order + next) / 2.0;

                // Check if gap is exhausted (midpoint rounded to one of the endpoints)
                if mid <= preceding_order || mid >= next {
                    return Err("Gap exhausted — re-indexing required".to_string());
                }

                // Check for float precision issues
                if mid == preceding_order || mid == next {
                    return Err("Gap exhausted — re-indexing required".to_string());
                }

                Ok(mid)
            }
            None => {
                // No next sibling — we're inserting at the end
                // Use preceding_order + 1.0
                Ok(preceding_order + 1.0)
            }
        }
    }

    /// Insert as the first child (before any existing siblings).
    ///
    /// # Arguments
    ///
    /// * `siblings` - All sibling blocks sorted by `order`.
    ///
    /// # Returns
    ///
    /// * The order value for the new first block.
    pub fn insert_first(siblings: &[Block]) -> f64 {
        if siblings.is_empty() {
            1.0
        } else {
            // Find the minimum order
            let min_order = siblings
                .iter()
                .map(|b| b.order)
                .min_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap_or(1.0);

            // Place before the first sibling
            min_order / 2.0
        }
    }

    /// Re-index blocks with evenly spaced order values.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of blocks to re-index.
    ///
    /// # Returns
    ///
    /// * A `Vec<f64>` of evenly spaced order values: 1.0, 2.0, 3.0, ...
    pub fn reindex(count: usize) -> Vec<f64> {
        (1..=count).map(|i| i as f64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::Uuid;

    fn make_block(order: f64) -> Block {
        Block {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order,
            level: 1,
            format: crate::value_objects::BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: format!("Block at order {}", order),
            properties: std::collections::HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_insert_after_normal_midpoint() {
        // Given siblings with orders 1.0, 2.0, 3.0
        let siblings = vec![make_block(1.0), make_block(2.0), make_block(3.0)];

        // When inserting after order 2.0
        let result = OrderCalculator::insert_after(2.0, &siblings).unwrap();

        // Then the new order should be the midpoint between 2.0 and 3.0
        assert!((result - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_last_block() {
        // Given siblings with orders 1.0, 2.0, 3.0
        let siblings = vec![make_block(1.0), make_block(2.0), make_block(3.0)];

        // When inserting after the last block (order 3.0)
        let result = OrderCalculator::insert_after(3.0, &siblings).unwrap();

        // Then the new order should be preceding + 1.0
        assert!((result - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_only_block() {
        // Given a single sibling at order 1.0
        let siblings = vec![make_block(1.0)];

        // When inserting after it
        let result = OrderCalculator::insert_after(1.0, &siblings).unwrap();

        // Then the new order should be preceding + 1.0
        assert!((result - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_middle_of_two() {
        // Given siblings with orders 1.0, 10.0
        let siblings = vec![make_block(1.0), make_block(10.0)];

        // When inserting after 1.0
        let result = OrderCalculator::insert_after(1.0, &siblings).unwrap();

        // Then the new order should be midpoint
        assert!((result - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_gap_exhausted_returns_error() {
        // When the gap between preceding_order and next is too small
        // (midpoint rounds to one of the endpoints), we should get an error.
        //
        // Note: f64 precision means this is hard to trigger with normal values.
        // Using very close values to simulate exhaustion.
        let siblings = vec![make_block(1.0), make_block(1.0000000000000002)];

        let result = OrderCalculator::insert_after(1.0, &siblings);

        // This might succeed or fail depending on f64 precision
        // The important thing is don't panic
        match result {
            Ok(order) => {
                // Verify it's between 1.0 and 1.0000000000000002
                assert!(order > 1.0);
                assert!(order < 1.0000000000000002);
            }
            Err(msg) => {
                assert!(msg.contains("re-indexing"));
            }
        }
    }

    #[test]
    fn test_insert_first_empty_parent() {
        let siblings = vec![];
        let result = OrderCalculator::insert_first(&siblings);
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_first_with_existing() {
        let siblings = vec![make_block(1.0), make_block(2.0)];
        let result = OrderCalculator::insert_first(&siblings);
        assert!((result - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_reindex_basic() {
        let result = OrderCalculator::reindex(5);
        assert_eq!(result, vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_reindex_empty() {
        let result = OrderCalculator::reindex(0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_reindex_single() {
        let result = OrderCalculator::reindex(1);
        assert_eq!(result, vec![1.0]);
    }

    #[test]
    fn test_insert_after_unsorted_siblings() {
        // Should handle unsorted input by sorting internally
        let siblings = vec![make_block(3.0), make_block(1.0), make_block(2.0)];

        let result = OrderCalculator::insert_after(1.0, &siblings).unwrap();

        // Midpoint between 1.0 and 2.0
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_preceding_not_in_siblings() {
        // preceding_order may not be in siblings (e.g., block hasn't been saved yet)
        let siblings = vec![make_block(2.0), make_block(3.0)];

        // Insert after order 1.5 (which was the previous block's order)
        let result = OrderCalculator::insert_after(1.5, &siblings).unwrap();

        // Midpoint between 1.5 and 2.0
        assert!((result - 1.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_raw_midpoint() {
        let result = OrderCalculator::insert_after_raw(1.0, [2.0, 3.0].into_iter()).unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_raw_end() {
        let result = OrderCalculator::insert_after_raw(3.0, [1.0, 2.0].into_iter()).unwrap();
        assert!((result - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_insert_after_raw_empty_siblings() {
        let result = OrderCalculator::insert_after_raw(1.0, std::iter::empty()).unwrap();
        assert!((result - 2.0).abs() < f64::EPSILON);
    }
}
