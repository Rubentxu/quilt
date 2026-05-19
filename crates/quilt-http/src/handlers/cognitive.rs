//! Cognitive HTTP handlers
//!
//! REST endpoints for cognitive operations:
//! - GET /api/cognitive/map        - Get cognitive map for a page
//! - GET /api/cognitive/briefing   - Get morning briefing
//! - GET /api/cognitive/serendipity - Get serendipity connections
//! - GET /api/cognitive/availability - Check cognitive engine availability

use std::sync::Arc;

use axum::{extract::{Path, Query, State}, Json};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::HttpError;
use crate::state::HttpState;

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

/// Availability response for cognitive services
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

/// Serendipity query parameters
#[derive(Debug, Deserialize)]
pub struct SerendipityQuery {
    pub since: Option<String>,
    pub limit: Option<usize>,
    pub min_confidence: Option<f32>,
}

/// Argument map query parameters
#[derive(Debug, Deserialize)]
pub struct ArgumentMapQuery {
    pub page_name: String,
}

/// Mental model query parameters
#[derive(Debug, Deserialize)]
pub struct MentalModelQuery {
    pub agent_id: String,
}

/// Check if cognitive mirror is available and get analysis for a page
#[instrument(skip(state))]
pub async fn cognitive_mirror(
    State(state): State<Arc<HttpState>>,
    Path(page_name): Path<String>,
) -> Result<Json<CognitiveMapDto>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let result = mcp
        .cognitive_mirror_analysis(&page_name)
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    let dto: CognitiveMapDto = serde_json::from_value(result)
        .map_err(|e| HttpError::InternalError(format!("Failed to parse response: {}", e)))?;
    Ok(Json(dto))
}

/// Check cognitive engine availability
#[instrument(skip(state))]
pub async fn cognitive_available(
    State(state): State<Arc<HttpState>>,
) -> Result<Json<AvailabilityDto>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let status = mcp.cognitive_engine_status();
    Ok(Json(AvailabilityDto {
        available: status.cognitive_mirror,
        message: if status.cognitive_mirror {
            None
        } else {
            Some("Cognitive mirror engine not configured".into())
        },
    }))
}

/// Get serendipity connections
#[instrument(skip(state))]
pub async fn serendipity(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<SerendipityQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    // Parse the since string to DateTime<Utc> if provided
    let since_dt = params
        .since
        .as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let result = mcp
        .serendipity_query(
            since_dt,
            params.limit.unwrap_or(20),
            params.min_confidence.unwrap_or(0.3),
        )
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

/// Get argument map for a page
#[instrument(skip(state))]
pub async fn argument_map(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<ArgumentMapQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let result = mcp
        .argument_map_page(&params.page_name)
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

/// Get mental model for an agent
#[instrument(skip(state))]
pub async fn mental_model(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<MentalModelQuery>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let result = mcp
        .mental_model_for_agent(&params.agent_id)
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

/// Get availability status for all cognitive services
#[instrument(skip(state))]
pub async fn get_availability(
    State(state): State<Arc<HttpState>>,
) -> Result<Json<CognitiveAvailabilityDto>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let status = mcp.cognitive_engine_status();
    Ok(Json(CognitiveAvailabilityDto {
        cognitive_mirror: status.cognitive_mirror,
        serendipity_engine: status.serendipity_engine,
        argument_cartographer: status.argument_cartographer,
        mental_model_gardener: status.mental_model_gardener,
        counterfactual_explorer: status.counterfactual_explorer,
        knowledge_evolution_tracker: status.knowledge_evolution_tracker,
    }))
}

/// Get morning briefing with cognitive pulse, serendipity highlights, and decay alerts
#[instrument(skip(state))]
pub async fn morning_briefing(
    State(state): State<Arc<HttpState>>,
) -> Result<Json<serde_json::Value>, HttpError> {
    let mcp = state
        .mcp_server
        .as_ref()
        .ok_or_else(|| HttpError::InternalError("MCP server not initialized".to_string()))?;

    let result = mcp
        .morning_briefing()
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

/// Mount cognitive routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/api/cognitive/map/{pageName}", axum::routing::get(cognitive_mirror))
        .route("/api/cognitive/briefing", axum::routing::get(morning_briefing))
        .route("/api/cognitive/serendipity", axum::routing::get(serendipity))
        .route("/api/cognitive/availability", axum::routing::get(cognitive_available))
        .route("/api/cognitive/argument-map", axum::routing::get(argument_map))
        .route("/api/cognitive/mental-model", axum::routing::get(mental_model))
        .route("/api/cognitive/status", axum::routing::get(get_availability))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::cognitive::{
        ArgumentMapQuery, AvailabilityDto, CognitiveAvailabilityDto, CognitiveMapDto, MentalModelQuery,
        SerendipityQuery,
    };

    #[test]
    fn test_availability_dto_not_available() {
        let dto = AvailabilityDto {
            available: false,
            message: Some("MCP not initialized".to_string()),
        };

        assert!(!dto.available);
        assert!(dto.message.is_some());
    }

    #[test]
    fn test_availability_dto_available() {
        let dto = AvailabilityDto {
            available: true,
            message: None,
        };

        assert!(dto.available);
        assert!(dto.message.is_none());
    }

    #[test]
    fn test_cognitive_map_dto_serialization() {
        let dto = CognitiveMapDto {
            total_clusters: 5,
            total_frontiers: 3,
            total_gaps: 2,
            pages_analyzed: 10,
            available: true,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"totalClusters\":5"));
        assert!(json.contains("\"totalFrontiers\":3"));
        assert!(json.contains("\"totalGaps\":2"));
        assert!(json.contains("\"pagesAnalyzed\":10"));
        assert!(json.contains("\"available\":true"));
    }

    #[test]
    fn test_cognitive_availability_dto() {
        let dto = CognitiveAvailabilityDto {
            cognitive_mirror: true,
            serendipity_engine: false,
            argument_cartographer: true,
            mental_model_gardener: false,
            counterfactual_explorer: false,
            knowledge_evolution_tracker: false,
        };

        assert!(dto.cognitive_mirror);
        assert!(!dto.serendipity_engine);
        assert!(dto.argument_cartographer);
    }

    #[test]
    fn test_serendipity_query_defaults() {
        let query = SerendipityQuery {
            since: None,
            limit: None,
            min_confidence: None,
        };

        assert!(query.since.is_none());
        assert!(query.limit.is_none());
        assert!(query.min_confidence.is_none());
    }

    #[test]
    fn test_serendipity_query_with_values() {
        let query = SerendipityQuery {
            since: Some("2024-01-01T00:00:00Z".to_string()),
            limit: Some(25),
            min_confidence: Some(0.5),
        };

        assert_eq!(query.since, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(query.limit, Some(25));
        assert_eq!(query.min_confidence, Some(0.5));
    }

    #[test]
    fn test_argument_map_query_deserialization() {
        // Note: ArgumentMapQuery uses snake_case field names (page_name)
        // not camelCase, so the JSON must use snake_case
        let json = r#"{"page_name":"Test Page"}"#;
        let query: ArgumentMapQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.page_name, "Test Page");
    }

    #[test]
    fn test_mental_model_query_deserialization() {
        // Note: MentalModelQuery uses snake_case field names (agent_id)
        // not camelCase, so the JSON must use snake_case
        let json = r#"{"agent_id":"agent-123"}"#;
        let query: MentalModelQuery = serde_json::from_str(json).unwrap();

        assert_eq!(query.agent_id, "agent-123");
    }
}
