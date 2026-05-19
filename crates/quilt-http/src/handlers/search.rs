//! Search HTTP handlers
//!
//! REST endpoints for search operations:
//! - GET /api/search - Full-text search across blocks

use std::sync::Arc;

use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::error::HttpError;
use crate::state::HttpState;
use quilt_search::{SearchEngine, SearchService};

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

impl From<quilt_search::SearchResult> for SearchResultDto {
    fn from(result: quilt_search::SearchResult) -> Self {
        Self {
            block_id: result.block_id,
            page_id: result.page_id,
            page_name: result.page_name,
            content: result.content,
            snippet: result.snippet,
            score: result.score,
        }
    }
}

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

impl SearchQuery {
    fn limit(&self) -> usize {
        self.limit.unwrap_or(20)
    }
}

/// Full-text search across all blocks
#[instrument(skip(state))]
pub async fn search(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResultDto>>, HttpError> {
    let search_service = SearchService::new(state.pool.clone());

    let results = search_service
        .search(&params.q, params.limit())
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    let dtos: Vec<SearchResultDto> = results.into_iter().map(SearchResultDto::from).collect();

    Ok(Json(dtos))
}

/// Fuzzy search with prefix matching
#[instrument(skip(state))]
pub async fn fuzzy_search(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResultDto>>, HttpError> {
    let search_service = SearchService::new(state.pool.clone());

    let results = search_service
        .fuzzy_search(&params.q, params.limit())
        .await
        .map_err(|e| HttpError::InternalError(e.to_string()))?;

    let dtos: Vec<SearchResultDto> = results.into_iter().map(SearchResultDto::from).collect();

    Ok(Json(dtos))
}

/// Mount search routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/api/search", axum::routing::get(search))
        .route("/api/search/fuzzy", axum::routing::get(fuzzy_search))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::search::SearchResultDto;

    #[test]
    fn test_search_query_defaults() {
        let query = SearchQuery { q: "test".to_string(), limit: None };

        assert_eq!(query.q, "test");
        assert_eq!(query.limit(), 20);
    }

    #[test]
    fn test_search_query_with_limit() {
        let query = SearchQuery {
            q: "test".to_string(),
            limit: Some(50),
        };

        assert_eq!(query.q, "test");
        assert_eq!(query.limit(), 50);
    }

    #[test]
    fn test_search_result_dto_serialization() {
        let dto = SearchResultDto {
            block_id: "block-123".to_string(),
            page_id: "page-456".to_string(),
            page_name: "Test Page".to_string(),
            content: "This is test content".to_string(),
            snippet: "This is test...".to_string(),
            score: 0.95,
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"blockId\":\"block-123\""));
        assert!(json.contains("\"pageId\":\"page-456\""));
        assert!(json.contains("\"pageName\":\"Test Page\""));
        assert!(json.contains("\"content\":\"This is test content\""));
        assert!(json.contains("\"snippet\":\"This is test...\""));
        assert!(json.contains("\"score\":0.95"));
    }
}
