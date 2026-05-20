//! Cognitive engine HTTP handlers
//!
//! These handlers provide access to cognitive engines for AI-augmented knowledge work.
//! Full functionality requires the MCP server to be integrated into AppState.

use axum::{
    extract::{Extension, Query},
    Json,
};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;

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
/// Note: Full status requires MCP server integration.
#[instrument(skip(_state))]
pub async fn get_availability(
    Extension(_state): Extension<AppState>,
) -> Result<Json<AvailabilityDto>, AppError> {
    // Cognitive services require MCP server integration
    // For now, indicate that cognitive features are not yet available
    Ok(Json(AvailabilityDto {
        available: false,
        message: Some("Cognitive services require MCP server integration".to_string()),
    }))
}

/// GET /api/v1/cognitive/morning-briefing
///
/// Returns morning briefing with cognitive pulse, serendipity highlights, and decay alerts.
/// Requires MCP server integration for full functionality.
#[instrument(skip(_state))]
pub async fn morning_briefing(
    Extension(_state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Integrate with MCP server for full morning briefing
    Ok(Json(serde_json::json!({
        "available": false,
        "message": "Morning briefing requires MCP server integration",
        "cognitive_engines": {
            "cognitive_mirror": false,
            "serendipity_engine": false,
            "morning_briefing": false
        }
    })))
}

/// GET /api/v1/cognitive/serendipity
///
/// Returns unexpected connections and knowledge intersections.
/// Requires MCP server integration for full functionality.
#[instrument(skip(state))]
pub async fn serendipity(
    Query(params): Query<SerendipityParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = (params, state);

    // TODO: Integrate with MCP server for serendipity engine
    Ok(Json(serde_json::json!({
        "available": false,
        "message": "Serendipity engine requires MCP server integration",
        "results": []
    })))
}

/// GET /api/v1/cognitive/argument-map
///
/// Returns argument map for a page showing claims, evidence, and logical structure.
/// Requires MCP server integration for full functionality.
#[instrument(skip(_state))]
pub async fn argument_map(
    Query(params): Query<ArgumentMapParams>,
    Extension(_state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Integrate with MCP server for argument cartographer
    Ok(Json(serde_json::json!({
        "available": false,
        "page_name": params.page_name,
        "message": "Argument cartographer requires MCP server integration",
        "clusters": [],
        "frontiers": [],
        "gaps": []
    })))
}

/// GET /api/v1/cognitive/mental-model
///
/// Returns mental model for an agent showing beliefs, goals, and knowledge state.
/// Requires MCP server integration for full functionality.
#[instrument(skip(_state))]
pub async fn mental_model(
    Query(params): Query<MentalModelParams>,
    Extension(_state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO: Integrate with MCP server for mental model gardener
    Ok(Json(serde_json::json!({
        "available": false,
        "agent_id": params.agent_id,
        "message": "Mental model gardener requires MCP server integration",
        "beliefs": [],
        "goals": []
    })))
}
