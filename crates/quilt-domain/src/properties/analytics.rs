//! Property analytics value objects — co-occurrence, trends, PMI scoring.
//!
//! These are pure domain types with no persistence knowledge.
//! Computed by the application layer from raw repository data.

use serde::{Deserialize, Serialize};

/// A co-occurrence pair: two properties that appear together on blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyCoOccurrence {
    /// First property key (alphabetically first for stable ordering).
    pub key_a: String,
    /// Second property key.
    pub key_b: String,
    /// Number of blocks where both properties appear.
    pub co_occurrence_count: u64,
    /// Number of blocks that have key_a.
    pub count_a: u64,
    /// Number of blocks that have key_b.
    pub count_b: u64,
    /// Pointwise Mutual Information: log2(P(a,b) / (P(a) * P(b))).
    /// Positive = co-occur more than expected by chance.
    /// Negative = co-occur less than expected.
    pub pmi: f64,
}

/// A property's usage trend over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyTrend {
    /// Property key.
    pub key: String,
    /// Usage count in the current period.
    pub current_count: u64,
    /// Usage count in the previous period.
    pub previous_count: u64,
    /// Percentage change: ((current - previous) / max(previous, 1)) * 100.
    pub change_percent: f64,
    /// Trend direction.
    pub direction: TrendDirection,
}

/// Direction of a trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrendDirection {
    /// Usage is increasing (> 10% growth).
    Rising,
    /// Usage is stable (within ±10%).
    Stable,
    /// Usage is declining (> 10% drop).
    Declining,
    /// New property with no previous data.
    New,
}

/// Analytics request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsParams {
    /// Maximum co-occurrence pairs to return.
    pub co_occurrence_limit: usize,
    /// Maximum trending properties to return.
    pub trend_limit: usize,
    /// Period in days for trend comparison (default: 30).
    pub trend_period_days: u32,
}

impl Default for AnalyticsParams {
    fn default() -> Self {
        Self {
            co_occurrence_limit: 20,
            trend_limit: 20,
            trend_period_days: 30,
        }
    }
}

/// Complete analytics result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyAnalytics {
    /// Top co-occurring property pairs, sorted by PMI descending.
    pub co_occurrences: Vec<PropertyCoOccurrence>,
    /// Properties sorted by trend direction and magnitude.
    pub trends: Vec<PropertyTrend>,
    /// Total number of distinct properties in use.
    pub total_properties: u64,
    /// Total number of blocks with at least one property.
    pub total_blocks_with_properties: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_direction_new() {
        let t = PropertyTrend {
            key: "status".to_string(),
            current_count: 5,
            previous_count: 0,
            change_percent: f64::INFINITY,
            direction: TrendDirection::New,
        };
        assert_eq!(t.direction, TrendDirection::New);
    }

    #[test]
    fn test_analytics_params_default() {
        let p = AnalyticsParams::default();
        assert_eq!(p.co_occurrence_limit, 20);
        assert_eq!(p.trend_limit, 20);
        assert_eq!(p.trend_period_days, 30);
    }

    #[test]
    fn test_co_occurrence_serialization() {
        let co = PropertyCoOccurrence {
            key_a: "status".to_string(),
            key_b: "priority".to_string(),
            co_occurrence_count: 42,
            count_a: 100,
            count_b: 80,
            pmi: 1.23,
        };
        let json = serde_json::to_string(&co).unwrap();
        assert!(json.contains("status"));
        assert!(json.contains("pmi"));
    }
}
