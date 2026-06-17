//! Navigation HTTP handlers
//!
//! These handlers emit WebSocket events for real-time navigation updates.

use axum::{Json, extract::Extension};
use axum::{Router, routing::post};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::{AppState, NavigationEvent};
use quilt_domain::entities::Page;
use quilt_domain::repositories::PageRepository;

/// A page returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
}

impl From<Page> for PageDto {
    fn from(page: Page) -> Self {
        Self {
            id: page.id.to_string(),
            name: page.name,
            title: page.title,
            journal: page.journal,
            journal_day: page.journal_day.map(|d| d.as_i32() as i64),
            created_at: page.created_at.to_rfc3339(),
        }
    }
}

/// Navigate to page request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateToPageRequest {
    pub graph_id: Option<String>,
    pub page_name: String,
}

/// Navigate to block request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateToBlockRequest {
    pub graph_id: Option<String>,
    pub page_name: String,
    pub block_uuid: String,
}

/// Create router for /api/v1/navigate
pub fn routes() -> Router {
    Router::new()
        .route("/page", post(navigate_to_page))
        .route("/block", post(navigate_to_block))
}

/// POST /api/v1/navigate/page
///
/// Navigate to a specific page, updating last_opened_graph and broadcasting
/// a WebSocket event to all connected clients.
#[instrument(skip(state, page_repo))]
pub async fn navigate_to_page(
    Extension(state): Extension<AppState>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Json(payload): Json<NavigateToPageRequest>,
) -> Result<Json<PageDto>, AppError> {
    // Update last_opened_graph if graph_id is provided. The
    // `graph_id` field is treated as an absolute or relative path
    // string; the typed slot is `PathBuf` (ADR-0030 §5).
    let graph_path = payload.graph_id.as_ref().map(|gid| PathBuf::from(gid.clone()));

    if let Some(ref path) = graph_path {
        // Write-through to global state repo (best-effort)
        if let Err(e) = state.global_state_repo.set_last_opened_graph(Some(path)).await {
            tracing::warn!("failed to persist last_opened_graph: {}", e);
        }
        if let Err(e) = state.global_state_repo.push_recent(path).await {
            tracing::warn!("failed to push recent graph: {}", e);
        }
        // Also update in-memory cache
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(path.clone());
        let mut recents = state.recent_graphs.write().await;
        recents.retain(|p| p != path);
        recents.insert(0, path.clone());
        recents.truncate(10);
    }

    // Fetch the page
    let page = page_repo
        .get_by_name(&payload.page_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let page_dto = match page {
        Some(p) => PageDto::from(p),
        None => {
            // Return a placeholder page if not found
            PageDto {
                id: "".to_string(),
                name: payload.page_name.clone(),
                title: None,
                journal: false,
                journal_day: None,
                created_at: chrono::Utc::now().to_rfc3339(),
            }
        }
    };

    // Broadcast navigation event to WebSocket subscribers
    let event = NavigationEvent::page(payload.graph_id.clone(), payload.page_name.clone());
    if let Err(e) = state.navigation_tx.send(event) {
        tracing::warn!("No WebSocket subscribers for navigation event: {}", e);
    }

    Ok(Json(page_dto))
}

/// POST /api/v1/navigate/block
///
/// Navigate to a specific block, updating last_opened_graph and broadcasting
/// a WebSocket event to all connected clients.
#[instrument(skip(state))]
pub async fn navigate_to_block(
    Extension(state): Extension<AppState>,
    Json(payload): Json<NavigateToBlockRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Update last_opened_graph if graph_id is provided
    let graph_path = payload.graph_id.as_ref().map(|gid| PathBuf::from(gid.clone()));

    if let Some(ref path) = graph_path {
        // Write-through to global state repo (best-effort)
        if let Err(e) = state.global_state_repo.set_last_opened_graph(Some(path)).await {
            tracing::warn!("failed to persist last_opened_graph: {}", e);
        }
        if let Err(e) = state.global_state_repo.push_recent(path).await {
            tracing::warn!("failed to push recent graph: {}", e);
        }
        // Also update in-memory cache
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(path.clone());
        let mut recents = state.recent_graphs.write().await;
        recents.retain(|p| p != path);
        recents.insert(0, path.clone());
        recents.truncate(10);
    }

    // Broadcast navigation event to WebSocket subscribers
    let event = NavigationEvent::block(payload.graph_id, payload.page_name, payload.block_uuid);
    if let Err(e) = state.navigation_tx.send(event) {
        tracing::warn!("No WebSocket subscribers for navigation event: {}", e);
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}
