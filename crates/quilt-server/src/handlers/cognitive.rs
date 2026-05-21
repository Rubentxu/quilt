//! Cognitive engine HTTP handlers
//!
//! These handlers provide access to cognitive engines for AI-augmented knowledge work.
//! Full functionality requires the MCP server to be integrated into AppState.

use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use axum::{routing::get, Router};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;

/// Query params for morning briefing
#[derive(Debug, Deserialize)]
pub struct MorningBriefingParams {
    pub date: Option<NaiveDate>,
    pub include_stats: Option<bool>,
}

/// Morning briefing response
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MorningBriefingResponse {
    pub cognitive_pulse: CognitivePulse,
    pub serendipity_highlights: Vec<SerendipityHighlight>,
    pub decay_alerts: Vec<DecayAlert>,
    pub stats: BriefingStats,
    pub knowledge_evolution: Vec<KnowledgeEvolution>,
    pub generated_at: DateTime<Utc>,
    pub degraded: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitivePulse {
    pub total_pages: usize,
    pub total_blocks: usize,
    pub clusters: usize,
    pub frontiers: usize,
    pub gaps: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerendipityHighlight {
    pub from_page: String,
    pub to_page: String,
    pub connection_type: String,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecayAlert {
    pub page_name: String,
    pub last_modified: DateTime<Utc>,
    pub days_stale: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefingStats {
    pub pages_created_today: usize,
    pub blocks_created_today: usize,
    pub queries_run_today: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEvolution {
    pub topic: String,
    pub belief_changes: usize,
    pub reinforced_count: usize,
    pub abandoned_count: usize,
}

/// Availability response for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityDto {
    pub available: bool,
    pub message: Option<String>,
}

/// Full availability status for all cognitive services
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveAvailabilityDto {
    pub cognitive_mirror: bool,
    pub serendipity_engine: bool,
    pub argument_cartographer: bool,
    pub mental_model_gardener: bool,
    pub counterfactual_explorer: bool,
    pub knowledge_evolution_tracker: bool,
}

/// Query params for serendipity
#[derive(Debug, Deserialize)]
pub struct SerendipityParams {
    #[allow(dead_code)]
    pub since: Option<String>,
    #[allow(dead_code)]
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[allow(dead_code)]
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f32,
}

fn default_limit() -> usize {
    20
}

fn default_min_confidence() -> f32 {
    0.3
}

/// Query params for argument map
#[derive(Debug, Deserialize)]
pub struct ArgumentMapParams {
    pub page_name: String,
}

/// Query params for mental model
#[derive(Debug, Deserialize)]
pub struct MentalModelParams {
    pub agent_id: String,
}

/// Create router for /api/v1/cognitive
pub fn routes() -> Router {
    Router::new()
        .route("/availability", get(get_availability))
        .route("/morning-briefing", get(morning_briefing))
        .route("/serendipity", get(serendipity))
        .route("/argument-map", get(argument_map))
        .route("/mental-model", get(mental_model))
}

/// GET /api/v1/cognitive/availability
///
/// Returns overall availability of cognitive services.
#[instrument(skip(state))]
pub async fn get_availability(
    Extension(state): Extension<AppState>,
) -> Result<Json<AvailabilityDto>, AppError> {
    let available = state.cognitive_mirror.is_some()
        && state.serendipity_engine.is_some()
        && state.morning_briefing.is_some()
        && state.argument_cartographer.is_some();

    Ok(Json(AvailabilityDto {
        available,
        message: if available {
            None
        } else {
            Some("Cognitive services are available".to_string())
        },
    }))
}

/// GET /api/v1/cognitive/morning-briefing
///
/// Returns morning briefing with cognitive pulse, serendipity highlights, and decay alerts.
#[instrument(skip(state))]
pub async fn morning_briefing(
    Extension(state): Extension<AppState>,
    Query(params): Query<MorningBriefingParams>,
) -> Result<Json<MorningBriefingResponse>, StatusCode> {
    let Some(briefing) = &state.morning_briefing else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let briefing_dto = briefing.generate().await;

    let response = MorningBriefingResponse {
        cognitive_pulse: CognitivePulse {
            total_pages: briefing_dto.cognitive_pulse.total_pages,
            total_blocks: briefing_dto.cognitive_pulse.total_blocks,
            clusters: briefing_dto.cognitive_pulse.clusters,
            frontiers: briefing_dto.cognitive_pulse.frontiers,
            gaps: briefing_dto.cognitive_pulse.gaps,
        },
        serendipity_highlights: briefing_dto
            .serendipity_highlights
            .into_iter()
            .map(|h| SerendipityHighlight {
                from_page: h.from_page,
                to_page: h.to_page,
                connection_type: h.connection_type,
                confidence: h.confidence,
            })
            .collect(),
        decay_alerts: briefing_dto
            .decay_alerts
            .into_iter()
            .map(|a| DecayAlert {
                page_name: a.page_name,
                last_modified: a.last_modified,
                days_stale: a.days_stale,
            })
            .collect(),
        stats: BriefingStats {
            pages_created_today: briefing_dto.stats.pages_created_today,
            blocks_created_today: briefing_dto.stats.blocks_created_today,
            queries_run_today: briefing_dto.stats.queries_run_today,
        },
        knowledge_evolution: briefing_dto
            .knowledge_evolution
            .into_iter()
            .map(|k| KnowledgeEvolution {
                topic: k.topic,
                belief_changes: k.belief_changes,
                reinforced_count: k.reinforced_count,
                abandoned_count: k.abandoned_count,
            })
            .collect(),
        generated_at: briefing_dto.generated_at,
        degraded: briefing_dto.degraded,
    };

    Ok(Json(response))
}

/// GET /api/v1/cognitive/serendipity
///
/// Returns unexpected connections and knowledge intersections.
#[instrument(skip(state))]
pub async fn serendipity(
    Extension(state): Extension<AppState>,
    Query(params): Query<SerendipityParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let Some(engine) = &state.serendipity_engine else {
        return Ok(Json(serde_json::json!({
            "available": false,
            "message": "Serendipity engine not configured",
            "results": []
        })));
    };

    // TODO: Convert page_name to page_id and call engine.find_connections()
    let _ = (engine, params);

    Ok(Json(serde_json::json!({
        "available": true,
        "message": "Serendipity engine available",
        "results": []
    })))
}

/// GET /api/v1/cognitive/argument-map
///
/// Returns argument map for a page showing claims, evidence, and logical structure.
#[instrument(skip(state))]
pub async fn argument_map(
    Extension(state): Extension<AppState>,
    Query(params): Query<ArgumentMapParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let Some(_cartographer) = &state.argument_cartographer else {
        return Ok(Json(serde_json::json!({
            "available": false,
            "page_name": params.page_name,
            "message": "Argument cartographer not configured",
            "clusters": [],
            "frontiers": [],
            "gaps": []
        })));
    };

    Ok(Json(serde_json::json!({
        "available": true,
        "page_name": params.page_name,
        "message": "Argument cartographer available",
        "clusters": [],
        "frontiers": [],
        "gaps": []
    })))
}

/// GET /api/v1/cognitive/mental-model
///
/// Returns mental model for an agent showing beliefs, goals, and knowledge state.
#[instrument(skip(state))]
pub async fn mental_model(
    Extension(state): Extension<AppState>,
    Query(params): Query<MentalModelParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = state;

    Ok(Json(serde_json::json!({
        "available": false,
        "agent_id": params.agent_id,
        "message": "Mental model gardener not yet implemented",
        "beliefs": [],
        "goals": []
    })))
}
