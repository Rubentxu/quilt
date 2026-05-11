//! Cognitive-related Tauri commands

use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Cognitive map response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveMapDto {
    pub total_clusters: usize,
    pub total_frontiers: usize,
    pub total_gaps: usize,
    pub pages_analyzed: usize,
    pub available: bool,
}

/// Availability response for a single service
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityDto {
    pub available: bool,
    pub message: Option<String>,
}

/// Full availability status for all cognitive services
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

/// Check if cognitive mirror is available and get analysis for a page
#[tauri::command]
pub async fn cognitive_mirror(
    page_name: String,
    state: State<'_, AppState>,
) -> Result<CognitiveMapDto, String> {
    let mcp = &*state.mcp_server;

    let result = mcp
        .cognitive_mirror_analysis(&page_name)
        .await
        .map_err(|e| e.to_string())?;

    let dto: CognitiveMapDto =
        serde_json::from_value(result).map_err(|e| format!("Failed to parse response: {}", e))?;
    Ok(dto)
}

/// Check cognitive engine availability
#[tauri::command]
pub async fn cognitive_available(state: State<'_, AppState>) -> Result<AvailabilityDto, String> {
    let mcp = state.mcp_server.as_ref();

    let status = mcp.cognitive_engine_status();
    Ok(AvailabilityDto {
        available: status.cognitive_mirror,
        message: if status.cognitive_mirror {
            None
        } else {
            Some("Cognitive mirror engine not configured".into())
        },
    })
}

/// Get serendipity connections
#[tauri::command]
pub async fn serendipity(
    since: Option<String>,
    limit: Option<usize>,
    min_confidence: Option<f32>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mcp = state.mcp_server.as_ref();

    // Parse the since string to DateTime<Utc> if provided
    let since_dt = since
        .as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let result = mcp
        .serendipity_query(since_dt, limit.unwrap_or(20), min_confidence.unwrap_or(0.3))
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get argument map for a page
#[tauri::command]
pub async fn argument_map(
    page_name: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mcp = state.mcp_server.as_ref();

    let result = mcp
        .argument_map_page(&page_name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get mental model for an agent
#[tauri::command]
pub async fn mental_model(
    agent_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let mcp = state.mcp_server.as_ref();

    let result = mcp
        .mental_model_for_agent(&agent_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(result)
}

/// Get availability status for all cognitive services
#[tauri::command]
pub async fn get_availability(
    state: State<'_, AppState>,
) -> Result<CognitiveAvailabilityDto, String> {
    let mcp = state.mcp_server.as_ref();

    let status = mcp.cognitive_engine_status();
    Ok(CognitiveAvailabilityDto {
        cognitive_mirror: status.cognitive_mirror,
        serendipity_engine: status.serendipity_engine,
        argument_cartographer: status.argument_cartographer,
        mental_model_gardener: status.mental_model_gardener,
        counterfactual_explorer: status.counterfactual_explorer,
        knowledge_evolution_tracker: status.knowledge_evolution_tracker,
    })
}

/// Get morning briefing with cognitive pulse, serendipity highlights, and decay alerts
#[tauri::command]
pub async fn morning_briefing(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mcp = &*state.mcp_server;

    let result = mcp.morning_briefing().await.map_err(|e| e.to_string())?;

    Ok(result)
}
