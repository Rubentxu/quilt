//! Block-related HTTP handlers

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    Json,
};
use axum::{
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::services::ref_service::RefService;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository,
};
use quilt_search::SearchService;

/// A block returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub page_name: Option<String>,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub parent_id: Option<String>,
    pub order: f64,
    pub level: i32,
    pub collapsed: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<(Block, Option<String>)> for BlockDto {
    fn from((block, page_name): (Block, Option<String>)) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.page_id.to_string(),
            page_name,
            content: block.content,
            marker: block.marker.map(|m| format!("{:?}", m)),
            priority: block.priority.map(|p| format!("{:?}", p)),
            parent_id: block.parent_id.map(|p| p.to_string()),
            order: block.order,
            level: block.level as i32,
            collapsed: block.collapsed,
            created_at: block.created_at.to_rfc3339(),
            updated_at: block.updated_at.to_rfc3339(),
        }
    }
}

impl From<Block> for BlockDto {
    fn from(block: Block) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.page_id.to_string(),
            page_name: None,
            content: block.content,
            marker: block.marker.map(|m| format!("{:?}", m)),
            priority: block.priority.map(|p| format!("{:?}", p)),
            parent_id: block.parent_id.map(|p| p.to_string()),
            order: block.order,
            level: block.level as i32,
            collapsed: block.collapsed,
            created_at: block.created_at.to_rfc3339(),
            updated_at: block.updated_at.to_rfc3339(),
        }
    }
}

/// Query params for block listing
#[derive(Debug, Deserialize)]
pub struct QueryBlocksParams {
    pub dsl: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

/// Query params for block search
#[derive(Debug, Deserialize)]
pub struct SearchBlocksParams {
    pub query: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    50
}

/// Create block request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBlockRequest {
    pub page_name: String,
    pub content: String,
    pub parent_id: Option<String>,
}

/// Update block request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBlockRequest {
    pub content: Option<String>,
    pub parent_id: Option<String>,
    pub order: Option<f64>,
    pub level: Option<i32>,
    pub collapsed: Option<bool>,
}

/// Link blocks request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkBlocksRequest {
    pub source_id: String,
    pub target_id: String,
}

/// Create router for /api/v1/blocks
pub fn routes() -> Router {
    Router::new()
        .route("/", get(query_blocks).post(create_block))
        .route("/search", get(search_blocks))
        .route("/link", post(link_blocks))
        .route("/:id", delete(delete_block).patch(update_block))
        .route("/:id/backlinks", get(get_backlinks))
}

/// GET /api/v1/blocks?dsl=...&limit=...
#[instrument(skip(state))]
pub async fn query_blocks(
    Query(params): Query<QueryBlocksParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BlockDto>>, AppError> {
    // DSL query service not yet implemented — return empty results
    let _ = (params, state);
    Ok(Json(Vec::new()))
}

/// Update the reference index after a block save.
/// Tracks references in the content. Currently does not resolve page names
/// to UUIDs (passes `None` for the resolver to avoid a Sync-bound issue with
/// `dyn Fn` in the async context).
async fn update_ref_index(
    ref_service: &std::sync::Arc<tokio::sync::RwLock<RefService>>,
    block_id: Uuid,
    content: &str,
    _page_repo: &SqlitePageRepository,
) {
    let mut svc = ref_service.write().await;
    if let Err(e) = svc
        .on_block_saved(block_id, content, None)
        .await
    {
        tracing::error!(%block_id, error = %e, "Failed to update reference index");
    }
}

/// POST /api/v1/blocks
///
/// Creates a new block. Stubbed — re-enable when the handler trait bound is resolved.
#[instrument(skip(state))]
pub async fn create_block(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), AppError> {
    let _ = (state, payload);
    Err(AppError::Internal("Not yet implemented".to_string()))
}

/// GET /api/v1/blocks/search?query=...&limit=...
#[instrument(skip(state))]
pub async fn search_blocks(
    Query(params): Query<SearchBlocksParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<crate::handlers::search::SearchResultDto>>, AppError> {
    let search_service = SearchService::new(Arc::new(state.pool.clone()));
    let results = search_service
        .search(&params.query, params.limit)
        .await
        .map_err(|e| AppError::Internal(format!("Search error: {}", e)))?;

    let dtos: Vec<crate::handlers::search::SearchResultDto> = results
        .into_iter()
        .map(|r| crate::handlers::search::SearchResultDto {
            block_id: r.block_id,
            page_id: String::new(),
            page_name: r.page_name,
            content: r.content,
            snippet: r.snippet,
            score: r.score,
        })
        .collect();

    Ok(Json(dtos))
}

/// DELETE /api/v1/blocks/:id
///
/// Stubbed — full implementation requires fixing the async Send bound.
#[instrument(skip(state))]
pub async fn delete_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = (block_id, state);
    Err(AppError::Internal("Not yet implemented".to_string()))
}

/// PATCH /api/v1/blocks/:id
///
/// Stubbed — full implementation requires fixing the async Send bound.
#[instrument(skip(state))]
pub async fn update_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Json(payload): Json<UpdateBlockRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let _ = (block_id, state, payload);
    Err(AppError::Internal("Not yet implemented".to_string()))
}

/// GET /api/v1/blocks/:id/backlinks
#[instrument(skip(state))]
pub async fn get_backlinks(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BlockDto>>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    // Verify the target block exists
    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    // Get backlinks
    let backlinks = block_repo
        .get_backlinks(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dtos: Vec<BlockDto> = backlinks.into_iter().map(BlockDto::from).collect();

    Ok(Json(dtos))
}

/// POST /api/v1/blocks/link
#[instrument(skip(state))]
pub async fn link_blocks(
    Extension(state): Extension<AppState>,
    Json(payload): Json<LinkBlocksRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let source_uuid = Uuid::parse_str(&payload.source_id).ok_or_else(|| {
        AppError::BadRequest(format!("Invalid source UUID: {}", payload.source_id))
    })?;
    let target_uuid = Uuid::parse_str(&payload.target_id).ok_or_else(|| {
        AppError::BadRequest(format!("Invalid target UUID: {}", payload.target_id))
    })?;

    // Verify both blocks exist
    block_repo
        .get_by_id(source_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Source block not found: {}", payload.source_id))
        })?;

    block_repo
        .get_by_id(target_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Target block not found: {}", payload.target_id))
        })?;

    // TODO: Implement link creation when Block entity supports refs
    // For now, just validate both blocks exist and return success

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "source_id": payload.source_id,
            "target_id": payload.target_id,
            "linked": true
        })),
    ))
}
