//! Block HTTP handlers
//!
//! REST endpoints for block operations:
//! - GET  /api/blocks      - Query blocks using DSL
//! - POST /api/blocks      - Create a new block
//! - GET  /api/blocks/:id  - Get a single block
//! - PUT  /api/blocks/:id  - Update a block
//! - DELETE /api/blocks/:id - Delete a block

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::HttpError;
use crate::state::HttpState;
use quilt_application::query_service::QueryService;
use quilt_domain::entities::{Block, BlockCreate, Page};
use quilt_domain::repositories::{BlockReader, BlockRepository, BlockWriter, PageReader, PageRepository, PageWriter, SettingsRepository};
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqliteSettingsRepository,
};

/// Block response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub page_name: Option<String>,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<(Block, Option<String>)> for BlockDto {
    fn from((block, page_name): (Block, Option<String>)) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.hierarchy.page_id.to_string(),
            page_name,
            content: block.content.content.clone(),
            marker: block.task_state.marker.map(|m| format!("{:?}", m)),
            priority: block.task_state.priority.map(|p| format!("{:?}", p)),
            created_at: block.timestamps.created_at.to_rfc3339(),
            updated_at: block.timestamps.updated_at.to_rfc3339(),
        }
    }
}

impl From<Block> for BlockDto {
    fn from(block: Block) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.hierarchy.page_id.to_string(),
            page_name: None,
            content: block.content.content.clone(),
            marker: block.task_state.marker.map(|m| format!("{:?}", m)),
            priority: block.task_state.priority.map(|p| format!("{:?}", p)),
            created_at: block.timestamps.created_at.to_rfc3339(),
            updated_at: block.timestamps.updated_at.to_rfc3339(),
        }
    }
}

/// Block tree response DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTreeDto {
    pub block: BlockDto,
    pub children: Vec<BlockDto>,
    pub children_count: usize,
}

/// Query parameters for block queries
#[derive(Debug, Deserialize)]
pub struct BlocksQuery {
    pub dsl: Option<String>,
    pub limit: Option<usize>,
}

impl BlocksQuery {
    fn dsl(&self) -> &str {
        self.dsl.as_deref().unwrap_or("(all)")
    }

    fn limit(&self) -> usize {
        self.limit.unwrap_or(100)
    }
}

/// Request to create a new block
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBlockRequest {
    pub page_name: String,
    pub content: String,
    pub parent_id: Option<String>,
}

/// Request to update a block
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBlockRequest {
    pub content: Option<String>,
    pub parent_id: Option<String>,
    pub order: Option<f64>,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: Option<bool>,
}

/// Query blocks using DSL
#[instrument(skip(state))]
pub async fn query_blocks(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<BlocksQuery>,
) -> Result<Json<Vec<BlockDto>>, HttpError> {
    let query_service = QueryService::new();
    let pool = state.pool.clone();

    let results = query_service
        .execute(params.dsl(), params.limit(), &pool)
        .await
        .map_err(|e| HttpError::ValidationError(format!("Query error: {}", e)))?;

    let blocks: Vec<BlockDto> = results
        .blocks
        .into_iter()
        .map(BlockDto::from)
        .collect();

    Ok(Json(blocks))
}

/// Create a new block
#[instrument(skip(state))]
pub async fn create_block(
    State(state): State<Arc<HttpState>>,
    Json(req): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let settings_repo = SqliteSettingsRepository::new(state.pool.clone());

    // Get user timezone
    let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
    let timezone = TimezoneService::from_tz_string(&user_settings.timezone)
        .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap());

    // Find or create the page
    let page: Page = match page_repo.get_by_name(&req.page_name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = Page::new(quilt_domain::entities::PageCreate {
                name: req.page_name.clone(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            })
            .map_err(|e| HttpError::ValidationError(e.to_string()))?;
            page_repo.insert(&p).await?;
            p
        }
        Err(e) => return Err(HttpError::DatabaseError(e.to_string())),
    };

    // Parse parent ID if provided
    let parent_uuid = match req.parent_id {
        Some(ref s) => {
            if s.is_empty() {
                None
            } else {
                Some(Uuid::parse_str(s).ok_or_else(|| {
                    HttpError::ValidationError("Invalid parent UUID".to_string())
                })?)
            }
        }
        None => None,
    };

    let block = Block::new(
        BlockCreate {
            page_id: page.id,
            content: req.content,
            parent_id: parent_uuid,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        },
        &timezone,
    )
    .map_err(|e| HttpError::ValidationError(e.to_string()))?;

    block_repo.insert(&block).await?;

    Ok((StatusCode::CREATED, Json(BlockDto::from(block))))
}

/// Get a single block by ID
#[instrument(skip(state))]
pub async fn get_block(
    State(state): State<Arc<HttpState>>,
    Path(block_id): Path<String>,
) -> Result<Json<BlockDto>, HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| HttpError::NotFound(format!("Invalid block ID: {}", block_id)))?;

    let block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Block not found: {}", block_id)))?;

    // Get page name for response
    let page_name = page_repo
        .get_by_id(block.hierarchy.page_id)
        .await
        .ok()
        .flatten()
        .map(|p| p.name);

    Ok(Json(BlockDto::from((block, page_name))))
}

/// Get a block with its children (tree structure)
#[instrument(skip(state))]
pub async fn get_block_tree(
    State(state): State<Arc<HttpState>>,
    Path(block_id): Path<String>,
) -> Result<Json<BlockTreeDto>, HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| HttpError::NotFound(format!("Invalid block ID: {}", block_id)))?;

    let block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Block not found: {}", block_id)))?;

    let children = block_repo
        .get_children(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    let children_count = children.len();
    let child_dtos: Vec<BlockDto> = children.into_iter().map(BlockDto::from).collect();

    Ok(Json(BlockTreeDto {
        block: BlockDto::from(block),
        children: child_dtos,
        children_count,
    }))
}

/// Update an existing block
#[instrument(skip(state))]
pub async fn update_block(
    State(state): State<Arc<HttpState>>,
    Path(block_id): Path<String>,
    Json(req): Json<UpdateBlockRequest>,
) -> Result<Json<BlockDto>, HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let settings_repo = SqliteSettingsRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| HttpError::NotFound(format!("Invalid block ID: {}", block_id)))?;

    let mut block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Block not found: {}", block_id)))?;

    // Get timezone for journal day updates
    let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
    let timezone = TimezoneService::from_tz_string(&user_settings.timezone)
        .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap());

    // Apply updates
    if let Some(content) = req.content {
        block.content.content = content;
    }

    if let Some(parent_id) = req.parent_id {
        if parent_id.is_empty() {
            block.hierarchy.parent_id = None;
        } else {
            block.hierarchy.parent_id = Some(
                Uuid::parse_str(&parent_id)
                    .ok_or_else(|| HttpError::ValidationError("Invalid parent UUID".to_string()))?,
            );
        }
    }

    if let Some(order) = req.order {
        block.hierarchy.order = order;
    }

    if let Some(marker) = req.marker {
        block.task_state.marker = match marker.to_lowercase().as_str() {
            "now" => Some(TaskMarker::Now),
            "later" => Some(TaskMarker::Later),
            "todo" => Some(TaskMarker::Todo),
            "done" => Some(TaskMarker::Done),
            "cancelled" => Some(TaskMarker::Cancelled),
            _ => None,
        };
    }

    if let Some(priority) = req.priority {
        block.task_state.priority = match priority.to_lowercase().as_str() {
            "a" => Some(quilt_domain::value_objects::Priority::A),
            "b" => Some(quilt_domain::value_objects::Priority::B),
            "c" => Some(quilt_domain::value_objects::Priority::C),
            _ => None,
        };
    }

    if let Some(collapsed) = req.collapsed {
        block.collapsed = collapsed;
    }

    block.timestamps.updated_at = chrono::Utc::now();
    block.journal_entry.updated_journal_day = Some(timezone.today_journal_day());

    block_repo
        .update(&block)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    Ok(Json(BlockDto::from(block)))
}

/// Delete a block
#[instrument(skip(state))]
pub async fn delete_block(
    State(state): State<Arc<HttpState>>,
    Path(block_id): Path<String>,
) -> Result<StatusCode, HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| HttpError::NotFound(format!("Invalid block ID: {}", block_id)))?;

    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Block not found: {}", block_id)))?;

    block_repo
        .delete(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create a task block (block with todo marker)
#[instrument(skip(state))]
pub async fn create_task(
    State(state): State<Arc<HttpState>>,
    Json(req): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let settings_repo = SqliteSettingsRepository::new(state.pool.clone());

    // Get user timezone
    let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
    let timezone = TimezoneService::from_tz_string(&user_settings.timezone)
        .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap());

    // Find or create the page
    let page: Page = match page_repo.get_by_name(&req.page_name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = Page::new(quilt_domain::entities::PageCreate {
                name: req.page_name.clone(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            })
            .map_err(|e| HttpError::ValidationError(e.to_string()))?;
            page_repo.insert(&p).await?;
            p
        }
        Err(e) => return Err(HttpError::DatabaseError(e.to_string())),
    };

    let block = Block::new(
        BlockCreate {
            page_id: page.id,
            content: req.content,
            parent_id: None,
            order: 1.0,
            marker: Some(TaskMarker::Todo),
            format: BlockFormat::Markdown,
            properties: Default::default(),
        },
        &timezone,
    )
    .map_err(|e| HttpError::ValidationError(e.to_string()))?;

    block_repo.insert(&block).await?;

    Ok((StatusCode::CREATED, Json(BlockDto::from(block))))
}

/// Get backlinks for a block
#[instrument(skip(state))]
pub async fn get_backlinks(
    State(state): State<Arc<HttpState>>,
    Path(block_id): Path<String>,
) -> Result<Json<Vec<BlockDto>>, HttpError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| HttpError::NotFound(format!("Invalid block ID: {}", block_id)))?;

    // Verify block exists
    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Block not found: {}", block_id)))?;

    let backlinks = block_repo
        .get_backlinks(uuid)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    let dtos: Vec<BlockDto> = backlinks.into_iter().map(BlockDto::from).collect();

    Ok(Json(dtos))
}

/// Mount block routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/api/blocks", axum::routing::get(query_blocks).post(create_block))
        .route(
            "/api/blocks/{id}",
            axum::routing::get(get_block)
                .put(update_block)
                .delete(delete_block),
        )
        .route("/api/blocks/{id}/tree", axum::routing::get(get_block_tree))
        .route("/api/blocks/{id}/backlinks", axum::routing::get(get_backlinks))
        .route("/api/tasks", axum::routing::post(create_task))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::blocks::{
        BlockDto, BlocksQuery, CreateBlockRequest, UpdateBlockRequest,
    };

    #[test]
    fn test_blocks_query_defaults() {
        let query = BlocksQuery { dsl: None, limit: None };

        assert_eq!(query.dsl(), "(all)");
        assert_eq!(query.limit(), 100);
    }

    #[test]
    fn test_blocks_query_with_values() {
        let query = BlocksQuery {
            dsl: Some("(page \"Test\")".to_string()),
            limit: Some(50),
        };

        assert_eq!(query.dsl(), "(page \"Test\")");
        assert_eq!(query.limit(), 50);
    }

    #[test]
    fn test_blocks_query_limit_capped() {
        // Test that limit is respected
        let query = BlocksQuery {
            dsl: None,
            limit: Some(200),
        };

        assert_eq!(query.limit(), 200);
    }

    #[test]
    fn test_create_block_request_deserialization() {
        let json = r#"{"pageName":"Test Page","content":"Hello world"}"#;
        let req: CreateBlockRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.page_name, "Test Page");
        assert_eq!(req.content, "Hello world");
        assert!(req.parent_id.is_none());
    }

    #[test]
    fn test_create_block_request_with_parent() {
        let json = r#"{"pageName":"Test Page","content":"Hello","parentId":"123e4567-e89b-12d3-a456-426614174000"}"#;
        let req: CreateBlockRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.page_name, "Test Page");
        assert_eq!(req.content, "Hello");
        assert_eq!(req.parent_id, Some("123e4567-e89b-12d3-a456-426614174000".to_string()));
    }

    #[test]
    fn test_update_block_request_deserialization() {
        let json = r#"{"content":"Updated content","marker":"now","priority":"a"}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.content, Some("Updated content".to_string()));
        assert_eq!(req.marker, Some("now".to_string()));
        assert_eq!(req.priority, Some("a".to_string()));
        assert!(req.parent_id.is_none());
        assert!(req.order.is_none());
        assert!(req.collapsed.is_none());
    }

    #[test]
    fn test_update_block_request_with_collapsed() {
        let json = r#"{"collapsed":true}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.collapsed, Some(true));
    }

    #[test]
    fn test_update_block_request_clear_parent() {
        let json = r#"{"parentId":""}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();

        // Empty string means clear parent
        assert_eq!(req.parent_id, Some("".to_string()));
    }

    #[test]
    fn test_block_dto_serialization() {
        let dto = BlockDto {
            id: "test-id".to_string(),
            page_id: "page-id".to_string(),
            page_name: Some("Test Page".to_string()),
            content: "Test content".to_string(),
            marker: Some("now".to_string()),
            priority: Some("a".to_string()),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"pageName\":\"Test Page\""));
        assert!(json.contains("\"content\":\"Test content\""));
    }
}
