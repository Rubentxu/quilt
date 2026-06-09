//! Search-related HTTP handlers

use axum::{
    Json,
    extract::{Extension, Query},
};
use axum::{Router, routing::get};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_search::{SearchError, SearchHitProperty, SearchService};

/// One structured property carried inside a [`SearchResultDto`].
///
/// S1-04: mirrors the frontend `BlockProperty` type
/// (`quilt-ui/src/shared/types/api.ts`). The `value` is a JSON value
/// (string | number | boolean | null) and `property_type` is rendered
/// as `type` on the wire to match the frontend camelCase naming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPropertyDto {
    pub key: String,
    pub value: serde_json::Value,
    /// The block's structured property type ("string" | "number" |
    /// "boolean" | "date" | "select" | "page_ref" | "default"). The
    /// Rust layer only knows it has a raw `serde_json::Value` — the
    /// frontend infers the type at render time. Default = "default"
    /// when we have no information to give it.
    #[serde(rename = "type", default = "default_property_type")]
    pub property_type: String,
}

fn default_property_type() -> String {
    "default".to_string()
}

/// Search result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: String,
    pub score: f64,
    /// Structured properties projected from the blocks table (S1-04).
    /// Empty when the block has no properties (or its BLOB was
    /// malformed). The frontend's `blockMatchesFilter` uses this
    /// instead of regex-matching raw content.
    #[serde(default)]
    pub properties: Vec<BlockPropertyDto>,
}

/// Convert a `SearchHitProperty` (quilt-search) into a
/// `BlockPropertyDto` (wire shape). We pick a `property_type` based on
/// the JSON value shape: strings become "string", numbers "number",
/// booleans "boolean", null "default" (with the key preserved). The
/// frontend uses this to decide how to render the value.
pub fn to_block_property_dto(p: SearchHitProperty) -> BlockPropertyDto {
    let property_type = match &p.value {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "default",
        // Arrays / objects aren't valid Logseq property values, but we
        // still surface the key so the user can see it.
        _ => "default",
    }
    .to_string();
    BlockPropertyDto {
        key: p.key,
        value: p.value,
        property_type,
    }
}

/// Query params for search
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_search_limit")]
    pub limit: usize,
}

fn default_search_limit() -> usize {
    50
}

/// Create router for /api/v1/search
pub fn routes() -> Router {
    Router::new().route("/", get(search))
}

/// Map a `SearchError` to the appropriate HTTP `AppError`.
///
/// - `EmptyQuery` → 400 Bad Request (the input produced no FTS5 tokens)
/// - everything else → 500 Internal (DB failure, cache lock poisoned, etc.)
///
/// Exposed as `pub` so other handler modules (e.g. `blocks::search_blocks`)
/// can reuse the same mapping logic and keep error semantics consistent
/// across endpoints.
pub fn map_search_error(e: SearchError) -> AppError {
    match e {
        SearchError::EmptyQuery => AppError::BadRequest("Empty or invalid query".to_string()),
        other => AppError::Internal(format!("Search error: {}", other)),
    }
}

/// GET /api/v1/search?q=...&limit=...
#[instrument(skip(state))]
pub async fn search(
    Query(params): Query<SearchParams>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<SearchResultDto>>, AppError> {
    let search_service = SearchService::new(Arc::new(state.pool.clone()));

    let results = search_service
        .search(&params.q, params.limit)
        .await
        .map_err(map_search_error)?;

    let dtos: Vec<SearchResultDto> = results
        .into_iter()
        .map(|r| SearchResultDto {
            block_id: r.block_id,
            page_id: String::new(),
            page_name: r.page_name,
            content: r.content,
            snippet: r.snippet,
            score: r.score,
            properties: r
                .properties
                .into_iter()
                .map(to_block_property_dto)
                .collect(),
        })
        .collect();

    Ok(Json(dtos))
}
