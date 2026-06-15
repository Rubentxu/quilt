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
use quilt_domain::references::RefType;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{
    BlockFormat, BlockType, Priority, PropertyValue, TaskMarker, Uuid,
};
use std::collections::HashMap;

/// A block returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub page_name: Option<String>,
    pub content: String,
    /// Visual / semantic kind of the block. Wire format is the
    /// lowercase string form of [`BlockType`] (e.g. `"heading1"`),
    /// matching the TypeScript `BlockType` union in
    /// `quilt-ui/src/shared/types/api.ts`.
    pub block_type: String,
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
            // Map domain `BlockType` → its canonical lowercase string.
            // `block_type.as_str()` is the same value the SQLite column
            // stores and the TypeScript `BlockType` union expects, so
            // a round-trip is byte-identical.
            block_type: block.block_type.as_str().to_string(),
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
        BlockDto::from((block, None))
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
    /// Optional `blockType` change. The frontend sends one of the 11
    /// `BlockType` strings (e.g. `"heading1"`, `"code"`). An unknown
    /// value is rejected with `400 Bad Request` so the slash-command
    /// registry surfaces a clear error to the user. A missing field
    /// (`None`) is a no-op — it does NOT reset to `Paragraph`, which
    /// matches standard `PATCH` semantics.
    pub block_type: Option<String>,
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

/// Map an [`quilt_application::ApplicationError`] onto the HTTP layer's [`AppError`].
pub(crate) fn map_app_error(e: quilt_application::ApplicationError) -> AppError {
    use quilt_application::ApplicationError;
    use quilt_domain::errors::DomainError;
    match e {
        ApplicationError::Validation(msg) => AppError::BadRequest(msg),
        ApplicationError::NotFound(kind, id) => {
            AppError::NotFound(format!("{} not found: {}", kind, id))
        }
        ApplicationError::Domain(d) => match d {
            DomainError::InvalidData(msg) => AppError::BadRequest(msg),
            DomainError::BlockNotFound(id) => {
                AppError::NotFound(format!("Block not found: {}", id))
            }
            DomainError::PageNotFound(id) => AppError::NotFound(format!("Page not found: {}", id)),
            _ => AppError::Internal(d.to_string()),
        },
        ApplicationError::Infrastructure(msg) => AppError::Internal(msg),
    }
}

/// Create router for /api/v1/blocks
pub fn routes() -> Router {
    Router::new()
        .route("/", get(query_blocks).post(create_block))
        .route("/search", get(search_blocks))
        .route("/link", post(link_blocks))
        .route("/by-author", get(list_blocks_by_author))
        .route("/authors", get(list_distinct_authors))
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

    let blocks = if let Some(ref dsl) = params.dsl {
        if dsl.trim().is_empty() {
            // Empty DSL string = return all blocks via "self" query
            state
                .services
                .search
                .query_dsl("self", limit)
                .await
                .map_err(map_app_error)?
        } else {
            // Parse and execute the DSL query
            state
                .services
                .search
                .query_dsl(dsl, limit)
                .await
                .map_err(map_app_error)?
        }
    } else {
        // No DSL param = return all blocks via "self" query
        state
            .services
            .search
            .query_dsl("self", limit)
            .await
            .map_err(map_app_error)?
    };

    let dtos: Vec<BlockDto> = blocks.into_iter().map(|b| BlockDto::from(b)).collect();
    Ok(Json(dtos))
}

/// POST /api/v1/blocks
///
/// Creates a new block on the given page.
#[instrument(skip(state, page_repo, block_repo))]
pub async fn create_block(
    Extension(state): Extension<AppState>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Json(payload): Json<CreateBlockRequest>,
) -> Result<(StatusCode, Json<BlockDto>), AppError> {
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
        .and_then(|s| Uuid::parse_str(s).ok());

    // Calculate order based on preceding_block_id (insert-after semantics)
    // If not provided, fall back to appending at the end.
    let order = if let Some(ref preceding_id) = payload.preceding_block_id {
        let preceding_uuid = Uuid::parse_str(preceding_id)
            .map_err(|_| AppError::BadRequest("Invalid preceding block UUID".into()))?;

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
        // New blocks default to `Paragraph`. The frontend can PATCH
        // the `blockType` field to change it (see `defaultBlockTypeHandler`
        // in `quilt-ui/src/features/outliner-tiptap/slashRegistry.tsx`).
        block_type: BlockType::Paragraph,
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
    let mut page_refs: Vec<(Uuid, RefType)> = Vec::new();
    for name in &parsed.page_names {
        if let Ok(Some(page)) = page_repo.get_by_name(name).await {
            page_refs.push((page.id, RefType::PageRef));
        }
    }

    if let Err(e) = state
        .ref_service
        .on_block_saved(block.id, &block.content, page_refs)
        .await
    {
        tracing::error!(%block.id, error = %e, "Failed to update reference index");
    }

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
    use crate::handlers::blocks::map_app_error;

    let results = state
        .services
        .search
        .search(&params.query, params.limit)
        .await
        .map_err(map_app_error)?;

    let dtos: Vec<crate::handlers::search::SearchResultDto> = results
        .into_iter()
        .map(|r| crate::handlers::search::SearchResultDto {
            block_id: r.block_id,
            page_id: String::new(),
            page_name: r.page_name,
            content: String::new(),
            snippet: r.snippet,
            score: r.score,
            properties: vec![],
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
#[instrument(skip(state, block_repo))]
pub async fn delete_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
) -> Result<StatusCode, AppError> {
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid block UUID: {}", block_id)))?;

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

    // Use the use-case for deletion
    state
        .services
        .block
        .delete(uuid)
        .await
        .map_err(map_app_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// PATCH /api/v1/blocks/:id
///
/// Updates an existing block's content, parent, order, level, or collapsed state.
#[instrument(skip(state, block_repo, page_repo))]
pub async fn update_block(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
    Extension(page_repo): Extension<Arc<dyn PageRepository>>,
    Json(payload): Json<UpdateBlockRequest>,
) -> Result<Json<BlockDto>, AppError> {
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

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
                .map_err(|_| AppError::BadRequest(format!("Invalid parent UUID: {}", pid)))?;
            Some(Some(uuid))
        }
        None => None,
    };

    // Parse `blockType` from its wire string form. We reject unknown
    // values with 400 instead of silently coercing to `Paragraph` —
    // silent coercion would mask bugs in the slash-command registry
    // and could let an attacker write a non-canonical value to the
    // database (the SQLite CHECK on the column would also reject it,
    // but failing fast at the boundary is better).
    let block_type_update = match payload.block_type.as_deref() {
        Some(s) => Some(BlockType::parse_str(s).map_err(|_| {
            AppError::BadRequest(format!(
                "Invalid blockType: '{}'. Expected one of: paragraph, heading1, heading2, heading3, bullet, numbered, todo, quote, code, divider, image",
                s
            ))
        })?),
        None => None,
    };

    // Track whether content changed BEFORE moving payload.content
    let content_changed = payload.content.is_some();

    // Parse optional marker / priority. The frontend's slash
    // registry (see `slashRegistry.tsx`) sends one of the known
    // values from `TaskMarker` / `Priority`. Reject unknowns with
    // 400 instead of silently dropping them — silent drops mask
    // the slash-menu → status-handler path and leave the user
    // wondering why `/todo` doesn't show a TODO badge.
    let marker_update = match payload.marker.as_deref() {
        Some(s) => Some(TaskMarker::parse_str(s).map_err(|_| {
            AppError::BadRequest(format!(
                "Invalid marker: '{}'. Expected one of: now, later, todo, doing, done, cancelled, waiting",
                s
            ))
        })?),
        None => None,
    };
    let priority_update = match payload.priority.as_deref() {
        Some(s) => Some(Priority::parse_str(s).map_err(|_| {
            AppError::BadRequest(format!(
                "Invalid priority: '{}'. Expected one of: A, B, C",
                s
            ))
        })?),
        None => None,
    };

    let update = BlockUpdate {
        content: payload.content,
        parent_id,
        order: payload.order,
        level: payload.level.map(|l| l as u8),
        collapsed: payload.collapsed,
        block_type: block_type_update,
        marker: marker_update,
        priority: priority_update,
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

        let mut page_refs: Vec<(Uuid, RefType)> = Vec::new();
        for name in &parsed.page_names {
            if let Ok(Some(page)) = page_repo.get_by_name(name).await {
                page_refs.push((page.id, RefType::PageRef));
            }
        }

        if let Err(e) = state
            .ref_service
            .on_block_saved(block.id, &block.content, page_refs)
            .await
        {
            tracing::error!(%block.id, error = %e, "Failed to update reference index");
        }
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
#[instrument(skip(state, block_repo))]
pub async fn get_backlinks(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
    Extension(block_repo): Extension<Arc<dyn BlockRepository>>,
) -> Result<Json<Vec<BlockDto>>, AppError> {
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    // Verify the target block exists
    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Block not found: {}", block_id)))?;

    // Use the use-case to get backlinks
    let backlinks = state
        .services
        .block
        .get_backlinks(uuid)
        .await
        .map_err(map_app_error)?;

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
    let blocks = state
        .services
        .block
        .list_by_property("created_by", &params.author, params.limit)
        .await
        .map_err(map_app_error)?;

    let dtos: Vec<BlockDto> = blocks.into_iter().map(BlockDto::from).collect();
    Ok(Json(dtos))
}

/// GET /api/v1/blocks/authors
///
/// Returns the distinct values of the `created_by` property whose
/// value starts with `agent::` (e.g. `agent::claude`, `agent::gemini`,
/// `agent::deepseek`). Result is sorted ASC and excludes NULLs / empty
/// strings.
///
/// This endpoint exists so the agent-activity panel (and any other
/// UI that needs to enumerate "which agents have ever written to the
/// graph") does NOT have to hardcode a list. New agents show up
/// automatically as soon as the first block authored by them is
/// created. S2-02.
#[instrument(skip(state))]
pub async fn list_distinct_authors(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let authors = state
        .services
        .block
        .list_distinct_authors(Some("agent::"))
        .await
        .map_err(map_app_error)?;
    Ok(Json(authors))
}

/// GET /api/v1/blocks/:id/properties
///
/// Returns all properties of a block as a JSON map.
#[instrument(skip(state))]
pub async fn get_block_properties(
    Path(block_id): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<HashMap<String, serde_json::Value>>, AppError> {
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let properties = state
        .services
        .block
        .get_properties(uuid)
        .await
        .map_err(map_app_error)?;

    let properties: HashMap<String, serde_json::Value> = properties
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
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    let prop_value = PropertyValue::from_json(&body.value).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Unsupported property value type for key '{}'",
            body.key
        ))
    })?;

    state
        .services
        .block
        .set_property(uuid, body.key, prop_value)
        .await
        .map_err(map_app_error)?;

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
    let uuid = Uuid::parse_str(&block_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", block_id)))?;

    state
        .services
        .block
        .delete_property(uuid, &key)
        .await
        .map_err(map_app_error)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/blocks/link
#[instrument(skip(state))]
pub async fn link_blocks(
    Extension(state): Extension<AppState>,
    Json(payload): Json<LinkBlocksRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let source_uuid = Uuid::parse_str(&payload.source_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid source UUID: {}", payload.source_id)))?;
    let target_uuid = Uuid::parse_str(&payload.target_id)
        .map_err(|_| AppError::BadRequest(format!("Invalid target UUID: {}", payload.target_id)))?;

    // Use the use-case to create the link (verifies both blocks exist)
    state
        .services
        .block
        .link(source_uuid, target_uuid)
        .await
        .map_err(map_app_error)?;

    // Also update the in-memory ref index for O(1) backlink queries
    if let Err(e) = state
        .ref_service
        .create_link(source_uuid, target_uuid)
        .await
    {
        tracing::warn!(%source_uuid, %target_uuid, error = %e, "Failed to update ref index");
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "source_id": payload.source_id,
            "target_id": payload.target_id,
            "linked": true
        })),
    ))
}

// ── Tests ──────────────────────────────────────────────────────────────
//
// These tests live at the handler boundary (not at the DB or domain
// layer) because the P0 contract is about the WIRE shape — the value
// the frontend receives in the JSON body of a PATCH /blocks/:id
// response, and the value it sends in the request body. Testing at
// the DTO/serde boundary catches mismatches with the TypeScript
// `BlockType` union that pure domain tests would miss.

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::BlockCreate;
    use std::collections::HashMap as StdHashMap;

    fn make_test_block(block_type: BlockType) -> Block {
        let create = BlockCreate {
            page_id: Uuid::new_v4(),
            content: "x".to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type,
            properties: StdHashMap::new(),
        };
        Block::new(create).unwrap()
    }

    /// The DTO's `blockType` JSON field MUST be the lowercase string
    /// form of the variant. This is the contract the TypeScript
    /// `BlockType` union reads.
    #[test]
    fn test_block_dto_serializes_block_type_as_camelcase_lowercase() {
        let dto: BlockDto = make_test_block(BlockType::Heading1).into();
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(
            json["blockType"],
            serde_json::Value::String("heading1".to_string()),
            "expected blockType:\"heading1\" in DTO JSON, got: {}",
            json
        );
    }

    /// Every variant must serialize to its canonical lowercase string.
    /// Catches drift between the Rust enum and the TS union.
    #[test]
    fn test_block_dto_block_type_for_every_variant() {
        let expected = [
            ("paragraph", BlockType::Paragraph),
            ("heading1", BlockType::Heading1),
            ("heading2", BlockType::Heading2),
            ("heading3", BlockType::Heading3),
            ("bullet", BlockType::Bullet),
            ("numbered", BlockType::Numbered),
            ("todo", BlockType::Todo),
            ("quote", BlockType::Quote),
            ("code", BlockType::Code),
            ("divider", BlockType::Divider),
            ("image", BlockType::Image),
        ];
        for (wire, variant) in expected {
            let dto: BlockDto = make_test_block(variant).into();
            let json = serde_json::to_value(&dto).unwrap();
            assert_eq!(
                json["blockType"],
                serde_json::Value::String(wire.to_string()),
                "variant {:?} did not serialize to {:?}",
                variant,
                wire
            );
        }
    }

    /// The PATCH /blocks/:id request must accept `blockType` as a
    /// string. We verify the field is present and deserializes from
    /// the exact wire form the frontend sends.
    #[test]
    fn test_update_block_request_accepts_block_type_string() {
        let json = r#"{"blockType":"heading1","content":"hi"}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.block_type.as_deref(), Some("heading1"));
        assert_eq!(req.content.as_deref(), Some("hi"));
    }

    /// `blockType` is optional on the PATCH body — omitting it must
    /// not break deserialization. The handler treats `None` as
    /// "don't touch the field".
    #[test]
    fn test_update_block_request_block_type_is_optional() {
        let json = r#"{"content":"hi"}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.block_type, None);
        assert_eq!(req.content.as_deref(), Some("hi"));
    }

    /// An unknown `blockType` value must be deserializable at the
    /// DTO layer (it's `Option<String>`) but the handler must reject
    /// it before touching the DB. This test asserts the DTO
    /// accepts any string; the handler's rejection logic is in
    /// `update_block` and is exercised by the integration test
    /// for the request-shape contract.
    #[test]
    fn test_update_block_request_accepts_any_string() {
        let json = r#"{"blockType":"made_up_kind"}"#;
        let req: UpdateBlockRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.block_type.as_deref(), Some("made_up_kind"));
    }

    /// The DTO is also deserializable (it carries no `Deserialize`
    /// invariants but the field must round-trip). Frontend tests
    /// send JSON shaped like the DTO and we need to be able to
    /// parse it back.
    #[test]
    fn test_block_dto_round_trip() {
        let original: BlockDto = make_test_block(BlockType::Code).into();
        let json = serde_json::to_string(&original).unwrap();
        let parsed: BlockDto = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.block_type, original.block_type);
        assert_eq!(parsed.content, original.content);
        assert_eq!(parsed.id, original.id);
    }

    /// A DTO with `page_name = None` (the single-arg `From<Block>`
    /// impl) must still produce the same `blockType` wire form.
    /// The two `From` impls in this file both map the type, but
    /// we'd be sad if one of them forgot.
    #[test]
    fn test_block_dto_block_type_via_tuple_from() {
        let block = make_test_block(BlockType::Quote);
        let dto: BlockDto = BlockDto::from((block, Some("My Page".to_string())));
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["blockType"], "quote");
        assert_eq!(json["pageName"], "My Page");
    }
}
