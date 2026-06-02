//! Connection scoring algorithms — pure functions.
//!
//! These functions operate on primitive types only (no `Block`, no repositories).
//! They are extracted from the duplicated implementations in:
//! - `quilt-analysis/src/connection_engine/engine.rs`
//! - `quilt-cognitive/src/serendipity/engine.rs`
//!
//! # Usage
//!
//! ```
//! use quilt_core::scoring::connection::*;
//!
//! let a = vec!["tag:rust".into(), "tag:async".into()];
//! let b = vec!["tag:rust".into(), "tag:testing".into()];
//!
//! let j = jaccard_similarity(&a, &b);
//! let t = temporal_decay(1000, 2000, 1.0);
//! let c = composite_score(j, t, 0.6, 0.4);
//! ```

use std::collections::HashSet;

/// Jaccard similarity between two sets of strings.
///
/// Returns `intersection_size / union_size`. Returns `0.0` when both sets are
/// empty (no evidence of similarity, rather than a degenerate 0/0).
///
/// # Examples
///
/// ```
/// use quilt_core::scoring::connection::jaccard_similarity;
///
/// // 2 of 4 elements overlap → 0.5
/// let a = vec!["x".into(), "y".into(), "z".into()];
/// let b = vec!["x".into(), "y".into(), "w".into()];
/// assert!((jaccard_similarity(&a, &b) - 0.5).abs() < 1e-12);
///
/// // Both empty → 0.0 (not NaN)
/// let empty: Vec<String> = vec![];
/// assert_eq!(jaccard_similarity(&empty, &empty), 0.0);
/// ```
pub fn jaccard_similarity(set_a: &[String], set_b: &[String]) -> f64 {
    let set_a: HashSet<&str> = set_a.iter().map(|s| s.as_str()).collect();
    let set_b: HashSet<&str> = set_b.iter().map(|s| s.as_str()).collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 0.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    intersection as f64 / union as f64
}

/// Temporal proximity score using halflife decay.
///
/// Given two Unix timestamps (in seconds) and a halflife in hours, returns a
/// score in `[0.0, 1.0]` where `1.0` means identical timestamps and values
/// decay toward `0.0` as the time difference increases.
///
/// Formula: `0.5 ^ (|ts_a - ts_b| / (halflife_hours * 3600))`
///
/// # Examples
///
/// ```
/// use quilt_core::scoring::connection::temporal_decay;
///
/// // Same timestamp → 1.0
/// assert!((temporal_decay(1000, 1000, 1.0) - 1.0).abs() < 1e-12);
///
/// // One halflife apart (1 hour diff, 1 hour halflife) → ≈0.5
/// let score = temporal_decay(0, 3600, 1.0);
/// assert!((score - 0.5).abs() < 0.01);
/// ```
pub fn temporal_decay(timestamp_a: i64, timestamp_b: i64, halflife_hours: f64) -> f64 {
    let diff_secs = (timestamp_a - timestamp_b).unsigned_abs() as f64;
    let halflife_secs = halflife_hours * 3600.0;
    let ratio = diff_secs / halflife_secs;
    let proximity = 0.5_f64.powf(ratio);
    proximity.clamp(0.0, 1.0)
}

/// Compute a weighted composite score from structural and temporal components.
///
/// `w_struct + w_temporal` need not sum to 1.0, but in practice they should
/// (the original implementations use 0.6 and 0.4).
///
/// # Examples
///
/// ```
/// use quilt_core::scoring::connection::composite_score;
///
/// // Equal components with (0.6, 0.4) → midpoint 0.5
/// assert!((composite_score(0.5, 0.5, 0.6, 0.4) - 0.5).abs() < 1e-12);
///
/// // Structural-only with (1.0, 0.0, 0.6, 0.4) → 0.6
/// assert!((composite_score(1.0, 0.0, 0.6, 0.4) - 0.6).abs() < 1e-12);
/// ```
pub fn composite_score(structural: f64, temporal: f64, w_struct: f64, w_temporal: f64) -> f64 {
    w_struct * structural + w_temporal * temporal
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── jaccard_similarity ──────────────────────────────────────────────

    #[test]
    fn test_jaccard_partial_overlap() {
        let a = vec!["a".into(), "b".into(), "c".into()];
        let b = vec!["a".into(), "b".into(), "d".into()];
        let sim = jaccard_similarity(&a, &b);
        assert!((sim - 0.5).abs() < 1e-12, "expected 0.5, got {sim}");
    }

    #[test]
    fn test_jaccard_no_overlap() {
        let a = vec!["a".into()];
        let b = vec!["b".into()];
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_both_empty() {
        let a: Vec<String> = vec![];
        let b: Vec<String> = vec![];
        // Both empty → 0.0 (not NaN, not 1.0)
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let a = vec!["a".into(), "b".into()];
        let b = vec!["a".into(), "b".into()];
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_jaccard_one_empty() {
        let a = vec!["a".into(), "b".into()];
        let b: Vec<String> = vec![];
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
        assert_eq!(jaccard_similarity(&b, &a), 0.0);
    }

    // ── temporal_decay ──────────────────────────────────────────────────

    #[test]
    fn test_temporal_same_timestamp() {
        let score = temporal_decay(1000, 1000, 1.0);
        assert!((score - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_temporal_halflife_apart() {
        // 1 hour apart, 1 hour halflife → 0.5
        let score = temporal_decay(0, 3600, 1.0);
        assert!((score - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_temporal_double_halflife() {
        // 2 hours apart, 1 hour halflife → 0.25
        let score = temporal_decay(0, 7200, 1.0);
        assert!((score - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_temporal_very_far() {
        // 100 years apart, 1 hour halflife → ≈ 0
        let score = temporal_decay(0, 3600 * 24 * 365 * 100, 1.0);
        assert!(score < 0.01);
    }

    #[test]
    fn test_temporal_clamped_range() {
        // Negative diff (order doesn't matter)
        let score = temporal_decay(5000, 0, 1.0);
        assert!((score - temporal_decay(0, 5000, 1.0)).abs() < 1e-12);
        // Within [0, 1]
        assert!((0.0..=1.0).contains(&score));
    }

    // ── composite_score ──────────────────────────────────────────────────

    #[test]
    fn test_composite_default_weights() {
        // Using weights 0.6, 0.4 — matching the original implementations
        let score = composite_score(0.5, 0.5, 0.6, 0.4);
        assert!((score - 0.5).abs() < 1e-12);
    }

    #[test]
    fn test_composite_structural_heavy() {
        let score = composite_score(1.0, 0.0, 0.6, 0.4);
        assert!((score - 0.6).abs() < 1e-12);
    }

    #[test]
    fn test_composite_temporal_heavy() {
        let score = composite_score(0.0, 1.0, 0.3, 0.7);
        assert!((score - 0.7).abs() < 1e-12);
    }

    #[test]
    fn test_composite_equal_weights() {
        let score = composite_score(1.0, 0.5, 0.5, 0.5);
        assert!((score - 0.75).abs() < 1e-12);
    }

    #[test]
    fn test_composite_zero_weights() {
        let score = composite_score(1.0, 1.0, 0.0, 0.0);
        assert_eq!(score, 0.0);
    }
}
