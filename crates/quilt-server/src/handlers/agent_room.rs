//! HTTP handlers for the Agent Room surface.
//!
//! Mounted under `/api/v1/agents`. The four endpoints are
//! `GET /agents` (list), `POST /agents` (spawn), `GET
//! /agents/:id` (status), and `POST /agents/:id/cancel`.
//!
//! Auth: every endpoint is behind the `auth` middleware
//! already applied at the server root. 401 vs 404 is
//! documented in the spec; the middleware is what actually
//! returns 401, the handler returns 404 for missing ids.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::Deserialize;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_analysis::agent_room::{
    AgentDto, AgentLifecycle, AgentListFilter, AgentListResponse, AgentRegistry, AgentStatus,
    SpawnAgentRequest,
};
use quilt_domain::value_objects::Uuid;

/// Query params for `GET /agents`. All optional; absent =
/// "no filter on this dimension".
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListAgentsQuery {
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub limit: Option<usize>,
}

impl ListAgentsQuery {
    pub fn into_filter(self) -> AgentListFilter {
        AgentListFilter {
            status: self.status.as_deref().and_then(AgentStatus::parse),
            agent_type: self.r#type,
            limit: self.limit,
        }
    }
}

/// `GET /api/v1/agents` — list runs.
#[instrument(skip(state))]
pub async fn list_agents(
    Extension(state): Extension<AppState>,
    Query(q): Query<ListAgentsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let lifecycle = state
        .agent_lifecycle
        .as_ref()
        .ok_or_else(|| AppError::Internal("agent_lifecycle not wired".to_string()))?;
    let resp: AgentListResponse = lifecycle.list(q.into_filter());
    Ok(Json(resp))
}

/// `POST /api/v1/agents` — spawn a new run. Returns 201.
#[instrument(skip(state, req))]
pub async fn spawn_agent(
    Extension(state): Extension<AppState>,
    Json(req): Json<SpawnAgentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let lifecycle = state
        .agent_lifecycle
        .as_ref()
        .ok_or_else(|| AppError::Internal("agent_lifecycle not wired".to_string()))?;
    let registry = state
        .agent_registry
        .as_ref()
        .ok_or_else(|| AppError::Internal("agent_registry not wired".to_string()))?;
    let known = registry.list_types();
    match lifecycle.spawn(req, &known).await {
        Ok(dto) => Ok((StatusCode::CREATED, Json(dto))),
        Err(quilt_analysis::agent_room::AgentError::UnknownType(t)) => {
            Err(AppError::BadRequest(format!(
                "Unknown agent type '{t}'. Supported types: {}",
                known.join(", ")
            )))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

/// `GET /api/v1/agents/:id` — single-run status.
#[instrument(skip(state))]
pub async fn get_agent(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let lifecycle = state
        .agent_lifecycle
        .as_ref()
        .ok_or_else(|| AppError::Internal("agent_lifecycle not wired".to_string()))?;
    let uuid =
        Uuid::parse_str(&id).map_err(|_| AppError::NotFound(format!("Agent {id} not found")))?;
    match lifecycle.get(uuid) {
        Some(dto) => Ok(Json(dto)),
        None => Err(AppError::NotFound(format!("Agent {id} not found"))),
    }
}

/// `POST /api/v1/agents/:id/cancel` — cancel a run. Idempotent.
#[instrument(skip(state))]
pub async fn cancel_agent(
    Extension(state): Extension<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let lifecycle = state
        .agent_lifecycle
        .as_ref()
        .ok_or_else(|| AppError::Internal("agent_lifecycle not wired".to_string()))?;
    let uuid =
        Uuid::parse_str(&id).map_err(|_| AppError::NotFound(format!("Agent {id} not found")))?;
    match lifecycle.cancel(uuid).await {
        Ok(dto) => Ok(Json(dto)),
        Err(quilt_analysis::agent_room::AgentError::NotFound(_)) => {
            Err(AppError::NotFound(format!("Agent {id} not found")))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}

/// Build the Axum router for `/api/v1/agents`.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_agents).post(spawn_agent))
        .route("/:id", get(get_agent))
        .route("/:id/cancel", post(cancel_agent))
}

// ── test wiring helpers ──────────────────────────────────────────

/// Convenience for tests: build an `AgentLifecycle` and an
/// `AgentRegistry` wired to the supplied repositories.
pub fn build_lifecycle_and_registry(
    block_repo: Arc<dyn quilt_domain::repositories::BlockRepository>,
    page_repo: Arc<dyn quilt_domain::repositories::PageRepository>,
) -> (AgentLifecycle, Arc<AgentRegistry>) {
    let lifecycle = AgentLifecycle::new(block_repo, page_repo);
    let registry = Arc::new(AgentRegistry::with_defaults());
    (lifecycle, registry)
}

// Convenience: silences unused warnings for the DTO type
// at this level.
#[allow(dead_code)]
fn _dto_link(_: AgentDto) {}
