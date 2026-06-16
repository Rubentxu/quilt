//! Morning Briefing DTOs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An item in today's agenda — a block from today's journal page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgendaItem {
    /// Block UUID.
    pub block_id: String,
    /// Truncated content preview (max 200 chars).
    pub content_preview: String,
    /// Page name this block belongs to.
    pub page_name: String,
    /// Whether this block has children.
    pub has_children: bool,
    /// When this block was last updated.
    pub updated_at: DateTime<Utc>,
}

/// A block that has decayed — not updated in a while and may need attention.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayAlert {
    /// Block UUID.
    pub block_id: String,
    /// Truncated content preview.
    pub content_preview: String,
    /// Page name this block belongs to.
    pub page_name: String,
    /// How many days since last update.
    pub days_since_update: i64,
    /// Severity: "low", "medium", "high".
    pub severity: String,
    /// Why this block is flagged.
    pub reason: String,
}

/// A serendipitous connection discovered between two blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerendipityHighlight {
    /// First block UUID.
    pub block_a_id: String,
    /// Second block UUID.
    pub block_b_id: String,
    /// Content preview of block A.
    pub block_a_preview: String,
    /// Content preview of block B.
    pub block_b_preview: String,
    /// Human-readable explanation of the connection.
    pub explanation: String,
    /// Confidence score 0.0–1.0.
    pub confidence: f32,
}

/// The complete morning briefing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MorningBriefingDto {
    /// Today's agenda items.
    pub agenda_items: Vec<AgendaItem>,
    /// Blocks that have decayed and may need attention.
    pub decay_alerts: Vec<DecayAlert>,
    /// Unexpected connections discovered since last briefing.
    pub serendipity_highlights: Vec<SerendipityHighlight>,
    /// When this briefing was generated.
    pub generated_at: DateTime<Utc>,
    /// Number of days since last journal entry (0 = today).
    pub days_since_last_journal: i64,
}

impl Default for MorningBriefingDto {
    fn default() -> Self {
        Self {
            agenda_items: Vec::new(),
            decay_alerts: Vec::new(),
            serendipity_highlights: Vec::new(),
            generated_at: Utc::now(),
            days_since_last_journal: 0,
        }
    }
}
