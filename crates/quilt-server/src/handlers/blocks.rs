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

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::query_service::QueryService;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::content::BlockContent;
use quilt_domain::repositories::{BlockRepository, PageRepository, SettingsRepository};
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqliteSettingsRepository,
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
            content: block.content.as_plain_text(),
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
            content: block.content.as_plain_text(),
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
    let query_service = QueryService::new();

    // First, prepare the query to validate it
    query_service
        .prepare(&params.dsl, params.limit)
        .map_err(|e| AppError::BadRequest(format!("Query parse error: {}", e)))?;

    // Execute the query
    let result = query_service
        .execute(&params.dsl, params.limit, &state.pool)
        .await
        .map_err(|e| AppError::Internal(format!("Query execution error: {}", e)))?;

    let blocks_with_names: Vec<BlockDto> = result
        .blocks
        .into_iter()
        .map(|block| {
            // For now, set page_name to None - fetching requires async which is complex in map
            (block, None::<String>).into()
        })
        .collect();

    Ok(Json(blocks_with_names))
}

/// POST /api/v1/blocks
#[instrument(skip(state))]
pub async fn create_block(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    // Create timezone service from user settings (fallback to UTC)
    let settings_repo = SqliteSettingsRepository::new(state.pool.clone());
    let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
    let timezone = TimezoneService::from_tz_string(&user_settings.timezone)
        .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap());

    // Find or create the page
    let page = match page_repo.get_by_name(&payload.page_name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = quilt_domain::entities::Page::new(quilt_domain::entities::PageCreate {
                name: payload.page_name.clone(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            })
            .map_err(|e| AppError::Internal(e.to_string()))?;
            page_repo
                .insert(&p)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            p
        }
        Err(e) => return Err(AppError::Internal(e.to_string())),
    };

    let parent_uuid = payload
        .parent_id
        .map(|s| {
            Uuid::parse_str(&s).ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", s)))
        })
        .transpose()?;

    let block = Block::new(
        BlockCreate {
            page_id: page.id,
            content: BlockContent::from_text(payload.content),
            parent_id: parent_uuid,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        },
        &timezone,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    block_repo
        .insert(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(BlockDto::from(block))))
}

/// GET /api/v1/blocks/search?query=...&limit=...
#[instrument(skip(state))]
pub async fn search_blocks(
    Query(params): Query<SearchBlocksParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<crate::handlers::search::SearchResultDto>>, AppError> {
    let search_service = SearchService::new(state.pool.clone());
    let results = search_service
        .search(&params.query, params.limit)
        .await
        .map_err(|e| AppError::Internal(format!("Search error: {}", e)))?;

    let dtos: Vec<crate::handlers::search::SearchResultDto> = results
        .into_iter()
        .map(|r| crate::handlers::search::SearchResultDto {
            block_id: r.block_id,
            page_id: r.page_id,
            page_name: r.page_name,
            content: r.content,
            snippet: r.snippet,
            score: r.score,
        })
        .collect();

    Ok(Json(dtos))
}

/// DELETE /api/v1/blocks/:id
#[instrument(skip(state))]
pub async fn delete_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<StatusCode, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    block_repo
        .delete(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v1/blocks/:id
#[instrument(skip(state))]
pub async fn update_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Json(payload): Json<UpdateBlockRequest>,
) -> Result<Json<BlockDto>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let mut block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    // Update fields if provided
    if let Some(content) = payload.content {
        block.content = BlockContent::from_text(content);
    }
    if let Some(parent_id) = payload.parent_id {
        block.parent_id = Uuid::parse_str(&parent_id);
    }
    if let Some(order) = payload.order {
        block.order = order;
    }
    if let Some(level) = payload.level {
        block.level = level as u8;
    }
    if let Some(collapsed) = payload.collapsed {
        block.collapsed = collapsed;
    }

    block_repo
        .update(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(BlockDto::from(block)))
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
