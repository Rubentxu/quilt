//! Navigation HTTP handlers
//!
//! These handlers emit WebSocket events for real-time navigation updates.

use axum::{Json, extract::Extension};
use axum::{Router, routing::post};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::AppError;
use crate::state::{AppState, NavigationEvent};
use quilt_domain::entities::Page;
use quilt_domain::repositories::PageRepository;
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;

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
#[instrument(skip(state))]
pub async fn navigate_to_page(
    Extension(state): Extension<AppState>,
    Json(payload): Json<NavigateToPageRequest>,
) -> Result<Json<PageDto>, AppError> {
    // Update last_opened_graph if graph_id is provided
    if let Some(ref gid) = payload.graph_id {
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(gid.clone());
    }

    // Fetch the page
    let page_repo = SqlitePageRepository::new(state.pool.clone());
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
    if let Err(e) = state.broadcast_navigation(event) {
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
    if let Some(ref gid) = payload.graph_id {
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(gid.clone());
    }

    // Broadcast navigation event to WebSocket subscribers
    let event = NavigationEvent::block(payload.graph_id, payload.page_name, payload.block_uuid);
    if let Err(e) = state.broadcast_navigation(event) {
        tracing::warn!("No WebSocket subscribers for navigation event: {}", e);
    }

    Ok(Json(serde_json::json!({ "status": "ok" })))
}
