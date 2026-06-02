//! Search-related HTTP handlers

use axum::{
    extract::{Extension, Query},
    Json,
};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;
use quilt_search::{SearchError, SearchService};

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
        })
        .collect();

    Ok(Json(dtos))
}
