//! Weekly Review DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Direction of the decay trend over the last two weeks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecayTrend {
    /// More decay alerts this week than last week.
    Worsening,
    /// Fewer decay alerts this week than last week.
    Improving,
    /// Equal (or no comparison possible).
    Stable,
}

impl DecayTrend {
    /// Build a trend from `current` and `previous` counts.
    pub fn from_counts(current: u32, previous: u32) -> (Self, i32) {
        if current == previous {
            (Self::Stable, 0)
        } else if current > previous {
            (Self::Worsening, current as i32 - previous as i32)
        } else {
            (Self::Improving, previous as i32 - current as i32)
        }
    }
}

/// Response body for `GET /api/v1/cognitive/weekly-review`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReviewDto {
    /// Start of the rolling 7-day window (UTC).
    pub week_start: DateTime<Utc>,
    /// End of the rolling 7-day window (UTC). Equal to `weekStart + 7 days`.
    pub week_end: DateTime<Utc>,
    /// Number of blocks created in the window.
    pub blocks_created: u32,
    /// Number of blocks updated in the window (regardless of `created_at`).
    pub blocks_updated: u32,
    /// Number of task blocks that transitioned to `Done` in the window.
    pub tasks_completed: u32,
    /// Decay trend direction.
    pub decay_trend: DecayTrend,
    /// `previous - current` if improving, `current - previous` if worsening (negated), 0 if stable.
    /// Positive = fewer decay alerts; negative = more decay alerts.
    pub decay_delta: i32,
    /// Number of distinct journal pages updated in the window.
    pub journal_days: u32,
    /// Heuristic "what to focus on next week" list. Capped at 5.
    pub suggestions: Vec<String>,
    /// When this response was generated (RFC 3339).
    pub generated_at: DateTime<Utc>,
}
