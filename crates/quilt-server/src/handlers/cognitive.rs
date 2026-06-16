//! Cognitive analysis HTTP handlers
//!
//! HTTP endpoints for cognitive/analysis features.
//! Currently includes only the Morning Briefing endpoint.

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
    AgendaItem, DecayAlert, MorningBriefing, MorningBriefingDto,
    SerendipityHighlight,
};

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

/// Create the cognitive routes router.
pub fn routes() -> Router {
    Router::new().route("/morning-briefing", get(get_morning_briefing))
}


