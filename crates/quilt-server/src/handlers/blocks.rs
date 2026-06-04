//! Block-related HTTP handlers

use axum::{
    Json,
    extract::{Extension, Path, Query},
    http::StatusCode,
};
use axum::{
    Router,
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;
use quilt_application::services::ref_service::parse_refs_from_content;
use quilt_domain::entities::{Block, BlockCreate, BlockUpdate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, PropertyValue, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository,
};
use quilt_search::SearchService;
use std::collections::HashMap;

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
    /// Typed properties — keyed by property name, value is the raw JSON
    /// representation (string, number, boolean, etc.).
    /// See [`PropertyValue::to_json`] / [`PropertyValue::from_json`].
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl From<(Block, Option<String>)> for BlockDto {
    fn from((block, page_name): (Block, Option<String>)) -> Self {
        let properties = block
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
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
            properties,
        }
    }
}

impl From<Block> for BlockDto {
    fn from(block: Block) -> Self {
        let properties = block
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json()))
            .collect();
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
            properties,
        }
    }
}

/// Query params for block listing
#[derive(Debug, Deserialize)]
pub struct QueryBlocksParams {
    pub dsl: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
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
    /// Insert the new block after this block (by calculating order between
    /// this block and its next sibling). Optional — if omitted, the block is
    /// appended at the end (max_order + 1.0).
    pub preceding_block_id: Option<String>,
    /// Initial properties to attach to the block. Each value is a JSON value
    /// (string, number, boolean, array). Used by features like comments
    /// (`type=comment`, `resolved=false`, `created_by=...`).
    #[serde(default)]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    /// Convenience field — when set, the handler will add a `created_by`
    /// property to the block using this value. Follows the convention
    /// `user::<name>` for human authors and `agent::<name>` (e.g.
    /// `agent::claude`) for AI authors. If `properties["created_by"]` is
    /// already present, this field does NOT override it (explicit wins).
    #[serde(default)]
    pub created_by: Option<String>,
}

/// Update block request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UpdateBlockRequest {
    pub content: Option<String>,
    pub marker: Option<String>,
    pub priority: Option<String>,
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

/// Request to set a property value on a block
#[derive(Debug, Deserialize)]
pub struct SetPropertyRequest {
    pub key: String,
    pub value: serde_json::Value,
}

/// Query params for [`list_blocks_by_author`].
#[derive(Debug, Deserialize)]
pub struct ListByAuthorParams {
    /// The author identifier (e.g. `user::alice`, `agent::claude`).
    pub author: String,
    /// Max number of blocks to return. Defaults to 50.
    #[serde(default = "default_by_author_limit")]
    pub limit: usize,
}

fn default_by_author_limit() -> usize {
    50
}

/// Create router for /api/v1/blocks
pub fn routes() -> Router {
    Router::new()
        .route("/", get(query_blocks).post(create_block))
        .route("/search", get(search_blocks))
        .route("/link", post(link_blocks))
        .route("/by-author", get(list_blocks_by_author))
        .route("/:id", delete(delete_block).patch(update_block))
        .route("/:id/backlinks", get(get_backlinks))
        .route(
            "/:id/properties",
            get(get_block_properties).put(set_block_property),
        )
        .route("/:id/properties/:key", delete(delete_block_property))
}

/// GET /api/v1/blocks?dsl=...&limit=...
#[instrument(skip(state))]
pub async fn query_blocks(
    Query(params): Query<QueryBlocksParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BlockDto>>, AppError> {
    let limit = params.limit;
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    let blocks = if let Some(ref dsl) = params.dsl {
        if dsl.trim().is_empty() {
            // Empty DSL string = return all blocks
            let sql = format!(
                "SELECT * FROM blocks ORDER BY created_at DESC LIMIT {}",
                limit
            );
            block_repo
                .query_dsl(&sql, &[])
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
        } else {
            // Parse the DSL query
            let parser = quilt_query::QueryParser;
            let expr = parser
                .parse(dsl)
                .map_err(|e| AppError::BadRequest(format!("Invalid query DSL: {}", e)))?;

            // Generate SQL with proper parameterization
            let executor = quilt_query::QueryExecutor::new();
            let (sql, sql_params) = executor
                .build_sql(&expr, limit)
                .map_err(|e| AppError::BadRequest(format!("Query compile error: {}", e)))?;

            // Convert SqlParam values to strings for the repository
            let str_params: Vec<String> = sql_params.iter().map(|p| p.as_string()).collect();

            block_repo
                .query_dsl(&sql, &str_params)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
        }
    } else {
        // No DSL param = return all blocks
        let sql = format!(
            "SELECT * FROM blocks ORDER BY created_at DESC LIMIT {}",
            limit
        );
        block_repo
            .query_dsl(&sql, &[])
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?
    };

    let dtos: Vec<BlockDto> = blocks.into_iter().map(|b| BlockDto::from(b)).collect();
    Ok(Json(dtos))
}

/// POST /api/v1/blocks
///
/// Creates a new block on the given page.
#[instrument(skip(state))]
pub async fn create_block(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    // Look up the page by name
    let page = page_repo
        .get_by_name(&payload.page_name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", payload.page_name)))?;

    // Determine the next order value
    let existing_blocks = block_repo
        .get_by_page(page.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let max_order = existing_blocks
        .iter()
        .map(|b| b.order)
        .fold(0.0_f64, |a, b| a.max(b));

    let parent_uuid = payload
        .parent_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s));

    // Calculate order based on preceding_block_id (insert-after semantics)
    // If not provided, fall back to appending at the end.
    let order = if let Some(ref preceding_id) = payload.preceding_block_id {
        let preceding_uuid = Uuid::parse_str(preceding_id)
            .ok_or_else(|| AppError::BadRequest("Invalid preceding block UUID".into()))?;

        if let Some(preceding) = existing_blocks.iter().find(|b| b.id == preceding_uuid) {
            // Find the next sibling (same parent, order > preceding.order)
            let next = existing_blocks
                .iter()
                .filter(|b| b.parent_id == preceding.parent_id && b.order > preceding.order)
                .min_by(|a, b| {
                    a.order
                        .partial_cmp(&b.order)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            match next {
                Some(next_block) => (preceding.order + next_block.order) / 2.0,
                None => preceding.order + 1.0,
            }
        } else {
            max_order + 1.0
        }
    } else {
        max_order + 1.0
    };

    // Convert optional payload properties into typed PropertyValues.
    // Skip values that fail to parse (e.g. unsupported types) — they are
    // filtered out instead of failing the whole request.
    let mut properties: HashMap<String, PropertyValue> = HashMap::new();
    if let Some(payload_props) = payload.properties {
        for (key, value) in payload_props {
            if let Some(prop_value) = PropertyValue::from_json(&value) {
                properties.insert(key, prop_value);
            } else {
                tracing::warn!(
                    key = %key,
                    value = %value,
                    "Skipping unsupported property value type on block create"
                );
            }
        }
    }

    // Apply the `created_by` convention unless the caller already set it
    // explicitly inside `properties`. The convention is:
    //   user::<name>  → human author
    //   agent::<name> → AI author (e.g. agent::claude)
    // We trim whitespace and ignore empty strings so the call is
    // forgiving of common client mistakes.
    if let Some(author) = payload.created_by.as_deref() {
        let trimmed = author.trim();
        if !trimmed.is_empty() && !properties.contains_key("created_by") {
            if let Some(value) =
                PropertyValue::from_json(&serde_json::Value::String(trimmed.to_string()))
            {
                properties.insert("created_by".to_string(), value);
            }
        }
    }

    let block = Block::new(BlockCreate {
        page_id: page.id,
        content: payload.content,
        parent_id: parent_uuid,
        order,
        marker: None,
        format: BlockFormat::Markdown,
        properties,
    })
    .map_err(|e| AppError::Internal(e.to_string()))?;

    block_repo
        .insert(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Update reference index — parse content for [[page]] and ((uuid)) refs
    let parsed = parse_refs_from_content(&block.content);

    // Resolve page names to UUIDs via the page repository
    let mut page_map: HashMap<String, Uuid> = HashMap::new();
    for name in &parsed.page_names {
        if let Ok(Some(page)) = page_repo.get_by_name(name).await {
            page_map.insert(name.clone(), page.id);
        }
    }

    // Build a sync resolver using the pre-resolved map
    let resolver = |name: &str| -> Option<Uuid> { page_map.get(name).copied() };

    let mut svc = state.ref_service.write().await;
    if let Err(e) = svc
        .on_block_saved(block.id, &block.content, Some(&resolver))
        .await
    {
        tracing::error!(%block.id, error = %e, "Failed to update reference index");
    }
    drop(svc);

    Ok((
        StatusCode::CREATED,
        Json(BlockDto::from((block, Some(page.name)))),
    ))
}

/// GET /api/v1/blocks/search?query=...&limit=...
#[instrument(skip(state))]
pub async fn search_blocks(
    Query(params): Query<SearchBlocksParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<crate::handlers::search::SearchResultDto>>, AppError> {
    use crate::handlers::search::map_search_error;

    let search_service = SearchService::new(Arc::new(state.pool.clone()));
    let results = search_service
        .search(&params.query, params.limit)
        .await
        .map_err(map_search_error)?;

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
/// Deletes a block by its UUID.
///
/// Returns:
/// - `204 No Content` when the block was deleted successfully (it must be a
///   leaf — blocks with children are rejected to avoid orphaning them).
/// - `404 Not Found` when no block with the given UUID exists.
/// - `409 Conflict` when the block still has children. The caller should
///   delete or re-parent the children first. The error message includes the
///   child count so the UI can surface an actionable message.
#[instrument(skip(state))]
pub async fn delete_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<StatusCode, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid block UUID: {}", block_id)))?;

    // Verify the block exists — surface 404 instead of silently no-op'ing.
    let _block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    // Refuse to orphan any children. The caller must delete or re-parent
    // them first. The message is intentionally specific so the UI can
    // tell the user exactly what to do.
    let children = block_repo
        .get_children(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !children.is_empty() {
        return Err(AppError::Conflict(format!(
            "Cannot delete block {}: it has {} child block(s). \
             Delete or re-parent the children first.",
            block_id,
            children.len()
        )));
    }

    block_repo
        .delete(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v1/blocks/:id
///
/// Updates an existing block's content, parent, order, level, or collapsed state.
#[instrument(skip(state))]
pub async fn update_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Json(payload): Json<UpdateBlockRequest>,
) -> Result<Json<BlockDto>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let mut block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    let parent_id = match payload.parent_id {
        Some(ref pid) if pid.is_empty() => {
            // Empty string means "clear the parent"
            Some(None)
        }
        Some(ref pid) => {
            let uuid = Uuid::parse_str(pid)
                .ok_or_else(|| AppError::BadRequest(format!("Invalid parent UUID: {}", pid)))?;
            Some(Some(uuid))
        }
        None => None,
    };

    // Track whether content changed BEFORE moving payload.content
    let content_changed = payload.content.is_some();

    let update = BlockUpdate {
        content: payload.content,
        parent_id,
        order: payload.order,
        level: payload.level.map(|l| l as u8),
        collapsed: payload.collapsed,
        ..Default::default()
    };

    block
        .update(update)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    block_repo
        .update(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Update reference index if content changed
    if content_changed {
        let parsed = parse_refs_from_content(&block.content);

        let mut page_map: HashMap<String, Uuid> = HashMap::new();
        for name in &parsed.page_names {
            if let Ok(Some(page)) = page_repo.get_by_name(name).await {
                page_map.insert(name.clone(), page.id);
            }
        }

        let resolver = |name: &str| -> Option<Uuid> { page_map.get(name).copied() };

        let mut svc = state.ref_service.write().await;
        if let Err(e) = svc
            .on_block_saved(block.id, &block.content, Some(&resolver))
            .await
        {
            tracing::error!(%block.id, error = %e, "Failed to update reference index");
        }
        drop(svc);
    }

    // Resolve page name for the DTO
    let page = page_repo
        .get_by_id(block.page_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let page_name = page.map(|p| p.name);

    Ok(Json(BlockDto::from((block, page_name))))
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

/// GET /api/v1/blocks/by-author?author=...&limit=...
///
/// Returns the blocks whose `created_by` property matches the given author.
/// Used to power the `/created-by` filter and the agent-activity panel.
#[instrument(skip(state))]
pub async fn list_blocks_by_author(
    Query(params): Query<ListByAuthorParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BlockDto>>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    let blocks = block_repo
        .list_by_property("created_by", &params.author, params.limit)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dtos: Vec<BlockDto> = blocks.into_iter().map(BlockDto::from).collect();
    Ok(Json(dtos))
}

/// GET /api/v1/blocks/:id/properties
///
/// Returns all properties of a block as a JSON map.
#[instrument(skip(state))]
pub async fn get_block_properties(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<HashMap<String, serde_json::Value>>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    let properties: HashMap<String, serde_json::Value> = block
        .properties
        .into_iter()
        .map(|(k, v)| (k, v.to_json()))
        .collect();

    Ok(Json(properties))
}

/// PUT /api/v1/blocks/:id/properties
///
/// Sets a single property on a block. Creates or updates the property
/// identified by `key`. The `value` field must be a JSON value that
/// can be converted into a [`PropertyValue`].
#[instrument(skip(state))]
pub async fn set_block_property(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Json(body): Json<SetPropertyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let mut block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    let prop_value = PropertyValue::from_json(&body.value).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Unsupported property value type for key '{}'",
            body.key
        ))
    })?;

    block.properties.insert(body.key, prop_value);
    block.updated_at = chrono::Utc::now();

    block_repo
        .update(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(serde_json::json!({"updated": true})))
}

/// DELETE /api/v1/blocks/:id/properties/:key
///
/// Removes a property from a block. Returns 204 No Content on success.
#[instrument(skip(state))]
pub async fn delete_block_property(
    Path((block_id, key)): Path<(String, String)>,
    Extension(state): Extension<AppState>,
) -> Result<StatusCode, AppError> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let mut block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    block.properties.remove(&key);
    block.updated_at = chrono::Utc::now();

    block_repo
        .update(&block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
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

    // Verify target block exists
    block_repo
        .get_by_id(target_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Target block not found: {}", payload.target_id))
        })?;

    // Fetch and validate source block (reuse for update)
    let mut source_block = block_repo
        .get_by_id(source_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| {
            AppError::NotFound(format!("Source block not found: {}", payload.source_id))
        })?;

    // Insert the link into the refs table with block_ref type
    let mut ref_service = state.ref_service.write().await;
    ref_service
        .create_link(source_uuid, target_uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    drop(ref_service);

    // Also update the source block's refs field for backward compatibility
    source_block.add_ref(target_uuid);

    block_repo
        .update(&source_block)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "source_id": payload.source_id,
            "target_id": payload.target_id,
            "linked": true
        })),
    ))
}
