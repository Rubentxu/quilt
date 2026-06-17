//! Global app state HTTP handlers (ADR-0030, Slice C).
//!
//! Exposes:
//! - `GET /api/v1/global-state` — read the cross-graph app state
//! - `PUT /api/v1/global-state/last-opened` — update last_opened_graph
//! - `PUT /api/v1/global-state/right-sidebar` — update sidebar visibility
//!
//! Auth: required (Bearer token, enforced by the global middleware).

use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, put},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::instrument;

use quilt_domain::entities::GlobalAppState;

use crate::error::AppError;
use crate::state::AppState;

/// Response shape for `GET /api/v1/global-state`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalStateResponse {
    pub last_opened_graph: Option<String>,
    pub recent_graphs: Vec<String>,
    pub right_sidebar_visible: Option<bool>,
}

impl From<&GlobalAppState> for GlobalStateResponse {
    fn from(state: &GlobalAppState) -> Self {
        Self {
            last_opened_graph: state
                .last_opened_graph
                .as_ref()
                .map(|p| p.display().to_string()),
            recent_graphs: state
                .recent_graphs
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            right_sidebar_visible: state.right_sidebar_visible,
        }
    }
}

/// Body for `PUT /api/v1/global-state/last-opened`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetLastOpenedRequest {
    pub graph_path: Option<String>,
}

/// Body for `PUT /api/v1/global-state/right-sidebar`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetRightSidebarRequest {
    pub visible: Option<bool>,
}

/// Create the router for `/api/v1/global-state`.
pub fn routes() -> Router {
    Router::new()
        .route("/", get(get_global_state))
        .route("/last-opened", put(set_last_opened))
        .route("/right-sidebar", put(set_right_sidebar))
}

/// `GET /api/v1/global-state`
///
/// Returns the cross-graph app state: `lastOpenedGraph`,
/// `recentGraphs`, and `rightSidebarVisible`.
#[instrument(skip(state))]
pub async fn get_global_state(
    Extension(state): Extension<AppState>,
) -> Result<Json<GlobalStateResponse>, AppError> {
    let state_data = state
        .global_state_repo
        .load()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(GlobalStateResponse::from(&state_data)))
}

/// `PUT /api/v1/global-state/last-opened`
///
/// Update the `last_opened_graph` pointer. The path is stored as-is;
/// validation (whether the graph is openable) is a separate concern
/// handled by the navigate or graphs/create handlers.
#[instrument(skip(state))]
pub async fn set_last_opened(
    Extension(state): Extension<AppState>,
    Json(payload): Json<SetLastOpenedRequest>,
) -> Result<Json<GlobalStateResponse>, AppError> {
    let path = payload
        .graph_path
        .as_ref()
        .map(|s| PathBuf::from(s.as_str()));

    // Write-through to global state repo (best-effort)
    if let Err(e) = state
        .global_state_repo
        .set_last_opened_graph(path.as_deref())
        .await
    {
        tracing::warn!("failed to persist last_opened_graph: {}", e);
    }

    // Also update in-memory cache
    {
        let mut last = state.last_opened_graph.write().await;
        *last = path;
    }

    let state_data = state
        .global_state_repo
        .load()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(GlobalStateResponse::from(&state_data)))
}

/// `PUT /api/v1/global-state/right-sidebar`
///
/// Update the persisted right-sidebar visibility preference.
#[instrument(skip(state))]
pub async fn set_right_sidebar(
    Extension(state): Extension<AppState>,
    Json(payload): Json<SetRightSidebarRequest>,
) -> Result<Json<GlobalStateResponse>, AppError> {
    // Write-through to global state repo (best-effort)
    if let Err(e) = state
        .global_state_repo
        .set_right_sidebar_visible(payload.visible)
        .await
    {
        tracing::warn!("failed to persist right_sidebar_visible: {}", e);
    }

    // Also update in-memory cache
    {
        let mut visible = state.right_sidebar_visible.write().await;
        *visible = payload.visible;
    }

    let state_data = state
        .global_state_repo
        .load()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(GlobalStateResponse::from(&state_data)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serde_snake_to_camel() {
        let state = GlobalAppState::new(
            Some(PathBuf::from("/var/data/g1")),
            vec![PathBuf::from("/var/data/g1"), PathBuf::from("/var/data/g2")],
            Some(true),
        );
        let resp = GlobalStateResponse::from(&state);
        assert_eq!(resp.last_opened_graph, Some("/var/data/g1".to_string()));
        assert_eq!(resp.recent_graphs.len(), 2);
        assert_eq!(resp.right_sidebar_visible, Some(true));
    }

    #[test]
    fn set_last_opened_request_parses() {
        let req: SetLastOpenedRequest =
            serde_json::from_str(r#"{"graphPath":"/var/data/g1"}"#).unwrap();
        assert_eq!(req.graph_path, Some("/var/data/g1".to_string()));

        let req_null: SetLastOpenedRequest = serde_json::from_str(r#"{"graphPath":null}"#).unwrap();
        assert_eq!(req_null.graph_path, None);
    }

    #[test]
    fn set_right_sidebar_request_parses() {
        let req: SetRightSidebarRequest = serde_json::from_str(r#"{"visible":false}"#).unwrap();
        assert_eq!(req.visible, Some(false));
    }
}
