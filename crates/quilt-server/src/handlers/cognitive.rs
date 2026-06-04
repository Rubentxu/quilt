//! Cognitive engine HTTP handlers
//!
//! These handlers provide access to cognitive engines for AI-augmented knowledge work.
//! Full functionality requires the MCP server to be integrated into AppState.

use axum::{
    Json,
    extract::{Extension, Query, State},
    http::StatusCode,
};
use axum::{Router, routing::get};
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

// ─── Analysis endpoints for Dream Cycle Display (G7) ────────────────────────────────────────
// These are display-only REST endpoints returning cognitive analysis results.

// Query params for analysis connections
#[derive(Debug, Deserialize)]
pub struct AnalysisConnectionsParams {
    #[serde(default = "default_connection_limit")]
    pub limit: usize,
}

fn default_connection_limit() -> usize {
    10
}

// Mirror analysis DTO — structural map of clusters, gaps, frontiers, density, top influencers
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorAnalysisDto {
    pub clusters: Vec<ClusterDto>,
    pub gaps: Vec<GapDto>,
    pub frontiers: Vec<String>,
    pub density: f64,
    pub top_influencers: Vec<InfluencerDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterDto {
    pub block_ids: Vec<String>,
    pub theme: Option<String>,
    pub coherence_score: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GapDto {
    pub from_block: String,
    pub to_block: String,
    pub shared_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluencerDto {
    pub block_id: String,
    pub influence_score: f32,
}

// Connections analysis DTO
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDto {
    pub pairs: Vec<ConnectionPairDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionPairDto {
    pub block_a: String,
    pub block_b: String,
    pub score: f32,
    pub reason: String,
}

// Gardener analysis DTO — beliefs and suggestions
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GardenerDto {
    pub beliefs: Vec<BeliefDto>,
    pub suggestions: Vec<DeepeningSuggestionDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BeliefDto {
    pub id: String,
    pub statement: String,
    pub confidence: f64,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepeningSuggestionDto {
    pub concept: String,
    pub current_depth: usize,
    pub suggested_questions: Vec<String>,
}

// ─── Analysis routes ─────────────────────────────────────────────────────────────────────

/// Create router for /api/v1/analysis
pub fn analysis_routes() -> Router {
    Router::new()
        .route("/mirror", get(analysis_mirror))
        .route("/connections", get(analysis_connections))
        .route("/gardener", get(analysis_gardener))
}

/// GET /api/v1/analysis/mirror
///
/// Returns structural mirror analysis: clusters, gaps, frontiers, density.
/// Display-only endpoint for the Dream Cycle cognitive panel.
#[instrument(skip(state))]
pub async fn analysis_mirror(
    Extension(state): Extension<AppState>,
) -> Result<Json<MirrorAnalysisDto>, AppError> {
    // Get all blocks from the database for analysis
    let blocks = state
        .pool
        .get_all_blocks()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if blocks.is_empty() {
        return Ok(Json(MirrorAnalysisDto {
            clusters: Vec::new(),
            gaps: Vec::new(),
            frontiers: Vec::new(),
            density: 0.0,
            top_influencers: Vec::new(),
        }));
    }

    // Use the structural mirror if available
    let Some(mirror) = &state.cognitive_mirror else {
        return Ok(MirrorAnalysisDto {
            clusters: Vec::new(),
            gaps: Vec::new(),
            frontiers: Vec::new(),
            density: 0.0,
            top_influencers: Vec::new(),
        });
    };

    // Analyze the blocks - convert to domain Block format
    use quilt_domain::entities::Block;
    use quilt_domain::value_objects::{BlockFormat, Uuid};

    let domain_blocks: Vec<Block> = blocks
        .into_iter()
        .map(|b| {
            let refs: Vec<Uuid> = b
                .refs
                .unwrap_or_default()
                .iter()
                .filter_map(|r| Uuid::parse_str(r).ok())
                .collect();
            Block {
                id: Uuid::parse_str(&b.id).unwrap_or_else(|_| Uuid::new_v4()),
                page_id: Uuid::parse_str(&b.page_id).unwrap_or_else(|_| Uuid::new_v4()),
                parent_id: b.parent_id.and_then(|p| Uuid::parse_str(&p).ok()),
                order: b.order.unwrap_or(1.0),
                level: b.level.unwrap_or(0) as i32,
                format: BlockFormat::Markdown,
                marker: b.marker.as_ref().and_then(|m| m.parse().ok()),
                priority: b.priority.as_ref().and_then(|p| p.parse().ok()),
                content: b.content.unwrap_or_default(),
                properties: std::collections::HashMap::new(),
                refs,
                tags: Vec::new(),
                scheduled: None,
                deadline: None,
                start_time: None,
                repeated: None,
                collapsed: false,
                created_at: b
                    .created_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
                updated_at: b
                    .updated_at
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
            }
        })
        .collect();

    let structure_map = mirror.analyze_blocks(&domain_blocks).await;

    // Convert to DTOs
    let clusters = structure_map
        .clusters
        .into_iter()
        .map(|c| ClusterDto {
            block_ids: c.block_ids.iter().map(|u| u.to_string()).collect(),
            theme: c.theme,
            coherence_score: c.coherence_score,
        })
        .collect();

    let gaps = structure_map
        .gaps
        .into_iter()
        .map(|g| GapDto {
            from_block: g.from.to_string(),
            to_block: g.to.to_string(),
            shared_refs: g.shared_refs.iter().map(|u| u.to_string()).collect(),
        })
        .collect();

    let frontiers = structure_map
        .frontiers
        .iter()
        .map(|u| u.to_string())
        .collect();

    // Calculate average density
    let density = if structure_map.density.is_empty() {
        0.0
    } else {
        structure_map.density.values().copied().sum::<f32>() as f64
            / structure_map.density.len() as f64
    };

    // Convert top influencers - take top 10 by influence score
    let top_influencers = structure_map
        .influences
        .into_iter()
        .take(10)
        .map(|inf| InfluencerDto {
            block_id: inf.block_id.to_string(),
            influence_score: inf.influence_score,
        })
        .collect();

    Ok(Json(MirrorAnalysisDto {
        clusters,
        gaps,
        frontiers,
        density,
        top_influencers,
    }))
}

/// GET /api/v1/analysis/connections
///
/// Returns serendipitous connections between blocks.
/// Limits results to 50 maximum regardless of requested limit.
#[instrument(skip(state))]
pub async fn analysis_connections(
    Extension(state): Extension<AppState>,
    Query(params): Query<AnalysisConnectionsParams>,
) -> Result<Json<ConnectionDto>, AppError> {
    // Clamp limit to 50
    let limit = params.limit.min(50);

    // Get all blocks
    let blocks = state
        .pool
        .get_all_blocks()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if blocks.is_empty() {
        return Ok(Json(ConnectionDto { pairs: Vec::new() }));
    }

    // Use the serendipity engine if available
    let Some(engine) = &state.serendipity_engine else {
        return Ok(Json(ConnectionDto { pairs: Vec::new() }));
    };

    // Build query for the engine
    use crate::connection_engine::types::SerendipityQuery;
    let query = SerendipityQuery {
        topic: None,
        limit,
        offset: 0,
        min_confidence: 0.1,
        temporal_window_days: Some(30),
        page_id: None,
    };

    let connections = engine
        .find_connections(query)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Convert to DTOs
    let pairs = connections
        .into_iter()
        .map(|c| ConnectionPairDto {
            block_a: c.idea_a.to_string(),
            block_b: c.idea_b.to_string(),
            score: c.confidence,
            reason: c.explanation,
        })
        .collect();

    Ok(Json(ConnectionDto { pairs }))
}

/// GET /api/v1/analysis/gardener
///
/// Returns belief suggestions from the structure gardener.
/// Display-only endpoint for cognitive panel.
#[instrument(skip(state))]
pub async fn analysis_gardener(
    Extension(state): Extension<AppState>,
) -> Result<Json<GardenerDto>, AppError> {
    // Get all blocks
    let blocks = state
        .pool
        .get_all_blocks()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if blocks.is_empty() {
        return Ok(Json(GardenerDto {
            beliefs: Vec::new(),
            suggestions: Vec::new(),
        }));
    }

    // For now, return empty beliefs and suggestions
    // The StructureGardener would need to be integrated into state
    // to provide real suggestions
    Ok(Json(GardenerDto {
        beliefs: Vec::new(),
        suggestions: Vec::new(),
    }))
}
