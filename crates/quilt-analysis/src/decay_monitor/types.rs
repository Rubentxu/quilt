//! Decay Monitor DTOs

use crate::morning_briefing::types::DecayAlert;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Per-severity counts for the returned decay alerts.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeverityCounts {
    /// Number of `low` severity alerts (informational).
    pub low: u32,
    /// Number of `medium` severity alerts (consider reviewing).
    pub medium: u32,
    /// Number of `high` severity alerts (significantly stale).
    pub high: u32,
}

impl SeverityCounts {
    /// Build a `SeverityCounts` by walking a slice of `DecayAlert`s.
    pub fn from_alerts(alerts: &[DecayAlert]) -> Self {
        let mut counts = Self::default();
        for alert in alerts {
            match alert.severity.as_str() {
                "high" => counts.high += 1,
                "medium" => counts.medium += 1,
                _ => counts.low += 1,
            }
        }
        counts
    }

    /// Sum of all severity counts.
    pub fn total(&self) -> u32 {
        self.low + self.medium + self.high
    }
}

/// Response body for `GET /api/v1/cognitive/decay`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayMonitorDto {
    /// The list of decay alerts, sorted by `daysSinceUpdate` desc, capped at 10.
    pub alerts: Vec<DecayAlert>,
    /// Number of alerts returned. Equal to `alerts.len()` and to
    /// `countsBySeverity.total()` in V1 (the API caps at 10).
    pub total_alerts: u32,
    /// Precomputed per-severity counts so the UI does not have to
    /// walk the array on every render.
    pub counts_by_severity: SeverityCounts,
    /// When this response was generated (RFC 3339).
    pub generated_at: DateTime<Utc>,
}
