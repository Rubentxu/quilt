//! Serendipity Monitor DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single serendipity highlight with block content previews and page names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerendipityHighlightDetail {
    /// First block UUID.
    pub block_a_id: String,
    /// Second block UUID.
    pub block_b_id: String,
    /// Content preview of block A (up to 200 chars).
    pub block_a_preview: String,
    /// Content preview of block B (up to 200 chars).
    pub block_b_preview: String,
    /// Human-readable explanation of the connection.
    pub explanation: String,
    /// Confidence score 0.0–1.0.
    pub confidence: f32,
    /// Page name of block A (if resolvable).
    pub block_a_page: Option<String>,
    /// Page name of block B (if resolvable).
    pub block_b_page: Option<String>,
}

/// Response body for `GET /api/v1/cognitive/serendipity`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerendipityMonitorDto {
    /// List of discovered connections, sorted by confidence desc.
    pub highlights: Vec<SerendipityHighlightDetail>,
    /// Total number of connections found (may exceed `highlights.len()` if paginated).
    pub total: usize,
    /// When this response was generated (RFC 3339).
    pub generated_at: DateTime<Utc>,
}
