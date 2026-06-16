//! Cognitive analysis HTTP handlers
//!
//! HTTP endpoints for cognitive/analysis features.
//! Currently includes:
//! - Morning Briefing (CG-1)
//! - Decay Monitor (CG-7)
//! - Weekly Review (CG-7)

use axum::{
    extract::Extension,
    response::IntoResponse,
    Json, Router,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_analysis::{
    AgendaItem, DecayAlert, DecayMonitorDto, DecayMonitorService, DecayTrend, MorningBriefing,
    MorningBriefingDto, SerendipityHighlight, WeeklyReviewDto, WeeklyReviewService,
};

// ─── Morning Briefing (CG-1) ─────────────────────────────────────────────────

/// Response envelope for morning briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MorningBriefingResponse {
    /// Today's agenda items.
    pub agenda_items: Vec<AgendaItem>,
    /// Blocks that have decayed and may need attention.
    pub decay_alerts: Vec<DecayAlert>,
    /// Unexpected connections discovered since last briefing.
    pub serendipity_highlights: Vec<SerendipityHighlight>,
    /// When this briefing was generated (RFC3339).
    pub generated_at: String,
    /// Number of days since last journal entry (0 = today).
    pub days_since_last_journal: i64,
}

impl From<MorningBriefingDto> for MorningBriefingResponse {
    fn from(dto: MorningBriefingDto) -> Self {
        Self {
            agenda_items: dto.agenda_items,
            decay_alerts: dto.decay_alerts,
            serendipity_highlights: dto.serendipity_highlights,
            generated_at: dto.generated_at.to_rfc3339(),
            days_since_last_journal: dto.days_since_last_journal,
        }
    }
}

/// GET /api/v1/cognitive/morning-briefing
///
/// Returns the morning briefing — a daily snapshot of the knowledge graph
/// including today's agenda, decay alerts, and serendipity highlights.
#[instrument(skip(state))]
pub async fn get_morning_briefing(
    Extension(state): Extension<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();

    // Create morning briefing (connection engine optional — serendipity highlights
    // will be empty if not provided)
    let briefing = MorningBriefing::new(block_repo, page_repo, None);
    let dto = briefing.generate().await;

    Ok(Json(MorningBriefingResponse::from(dto)))
}

// ─── Decay Monitor (CG-7) ────────────────────────────────────────────────────

/// Response envelope for the Decay Monitor endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayMonitorResponse {
    /// The list of decay alerts (capped at 10).
    pub alerts: Vec<DecayAlert>,
    /// Total alerts returned (equal to `alerts.len()` in V1).
    pub total_alerts: u32,
    /// Precomputed per-severity counts.
    pub counts_by_severity: DecayMonitorSeverityCounts,
    /// When this response was generated (RFC3339).
    pub generated_at: String,
}

/// Per-severity counts (mirrors `SeverityCounts` but as a flat DTO).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DecayMonitorSeverityCounts {
    pub low: u32,
    pub medium: u32,
    pub high: u32,
}

impl From<quilt_analysis::SeverityCounts> for DecayMonitorSeverityCounts {
    fn from(c: quilt_analysis::SeverityCounts) -> Self {
        Self {
            low: c.low,
            medium: c.medium,
            high: c.high,
        }
    }
}

impl From<DecayMonitorDto> for DecayMonitorResponse {
    fn from(dto: DecayMonitorDto) -> Self {
        Self {
            alerts: dto.alerts,
            total_alerts: dto.total_alerts,
            counts_by_severity: dto.counts_by_severity.into(),
            generated_at: dto.generated_at.to_rfc3339(),
        }
    }
}

/// GET /api/v1/cognitive/decay
///
/// Returns decay alerts grouped by severity (high, medium, low).
/// Reuses the same algorithm as the Morning Briefing, but exposes
/// only the decay section as a focused DTO with precomputed
/// per-severity counts.
#[instrument(skip(state))]
pub async fn get_decay(Extension(state): Extension<AppState>) -> Result<impl IntoResponse, AppError> {
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();

    let service = DecayMonitorService::new(block_repo, page_repo);
    let dto = service.detect_now().await;

    Ok(Json(DecayMonitorResponse::from(dto)))
}

// ─── Weekly Review (CG-7) ────────────────────────────────────────────────────

/// Response envelope for the Weekly Review endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyReviewResponse {
    /// Start of the rolling 7-day window (RFC3339).
    pub week_start: String,
    /// End of the rolling 7-day window (RFC3339). `weekStart + 7 days`.
    pub week_end: String,
    /// Number of blocks created in the window.
    pub blocks_created: u32,
    /// Number of blocks updated in the window.
    pub blocks_updated: u32,
    /// Number of task blocks that transitioned to Done in the window.
    pub tasks_completed: u32,
    /// Decay trend direction: "worsening", "improving", or "stable".
    pub decay_trend: String,
    /// Delta vs previous week. Positive = fewer decay alerts.
    pub decay_delta: i32,
    /// Number of distinct journal pages updated in the window.
    pub journal_days: u32,
    /// Heuristic "what to focus on next week" suggestions. Capped at 5.
    pub suggestions: Vec<String>,
    /// When this response was generated (RFC3339).
    pub generated_at: String,
}

impl From<WeeklyReviewDto> for WeeklyReviewResponse {
    fn from(dto: WeeklyReviewDto) -> Self {
        let trend_str = match dto.decay_trend {
            DecayTrend::Worsening => "worsening",
            DecayTrend::Improving => "improving",
            DecayTrend::Stable => "stable",
        };
        Self {
            week_start: dto.week_start.to_rfc3339(),
            week_end: dto.week_end.to_rfc3339(),
            blocks_created: dto.blocks_created,
            blocks_updated: dto.blocks_updated,
            tasks_completed: dto.tasks_completed,
            decay_trend: trend_str.to_string(),
            decay_delta: dto.decay_delta,
            journal_days: dto.journal_days,
            suggestions: dto.suggestions,
            generated_at: dto.generated_at.to_rfc3339(),
        }
    }
}

/// GET /api/v1/cognitive/weekly-review
///
/// Returns aggregate statistics for the last 7 days plus a
/// heuristic list of "suggestions for next week".
#[instrument(skip(state))]
pub async fn get_weekly_review(
    Extension(state): Extension<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();

    let service = WeeklyReviewService::new(block_repo, page_repo);
    let dto = service.generate().await;

    Ok(Json(WeeklyReviewResponse::from(dto)))
}

/// Create the cognitive routes router.
pub fn routes() -> Router {
    Router::new()
        .route("/morning-briefing", get(get_morning_briefing))
        .route("/decay", get(get_decay))
        .route("/weekly-review", get(get_weekly_review))
}
