//! Page HTTP handlers
//!
//! REST endpoints for page operations:
//! - GET  /api/pages       - List all pages
//! - POST /api/pages       - Create a new page
//! - GET  /api/pages/:name - Get a page by name
//! - GET  /api/journal/:date - Get or create journal page

use std::str::FromStr;
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
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::{BlockReader, BlockRepository, PageReader, PageRepository, PageWriter};
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
use quilt_infrastructure::database::sqlite::repositories::{SqliteBlockRepository, SqlitePageRepository};

/// Page response DTO
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
            name: page.name.clone(),
            title: page.title,
            journal: page.journal,
            journal_day: page.journal_day.map(|d| d.as_i32() as i64),
            created_at: page.created_at.to_rfc3339(),
        }
    }
}

/// Query parameters for listing pages
#[derive(Debug, Deserialize)]
pub struct ListPagesQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Request to create a new page
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePageRequest {
    pub name: String,
    pub title: Option<String>,
}

/// List all pages with pagination
#[instrument(skip(state))]
pub async fn list_pages(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<ListPagesQuery>,
) -> Result<Json<Vec<PageDto>>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let pages = page_repo
        .get_all()
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(pages.len());

    let dtos: Vec<PageDto> = pages
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(PageDto::from)
        .collect();

    Ok(Json(dtos))
}

/// Create a new page
#[instrument(skip(state))]
pub async fn create_page(
    State(state): State<Arc<HttpState>>,
    Json(req): Json<CreatePageRequest>,
) -> Result<(StatusCode, Json<PageDto>), HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    // Check if page already exists
    if page_repo
        .get_by_name(&req.name)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .is_some()
    {
        return Err(HttpError::ValidationError(format!(
            "Page already exists: {}",
            req.name
        )));
    }

    let page = Page::new(PageCreate {
        name: req.name,
        title: req.title,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .map_err(|e| HttpError::ValidationError(e.to_string()))?;

    page_repo
        .insert(&page)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(PageDto::from(page))))
}

/// Get a page by name
#[instrument(skip(state))]
pub async fn get_page(
    State(state): State<Arc<HttpState>>,
    Path(name): Path<String>,
) -> Result<Json<Option<PageDto>>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    Ok(Json(page.map(PageDto::from)))
}

/// Get or create a journal page for a specific date
#[instrument(skip(state))]
pub async fn get_journal(
    State(state): State<Arc<HttpState>>,
    Path(date): Path<String>,
) -> Result<Json<PageDto>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let day = JournalDay::from_str(&date)
        .map_err(|_| HttpError::ValidationError(format!("Invalid date format: {}. Use YYYYMMDD", date)))?;

    let page = match page_repo.get_journal(day).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = Page::new_journal(day, BlockFormat::Markdown)
                .map_err(|e| HttpError::ValidationError(e.to_string()))?;
            page_repo
                .insert(&p)
                .await
                .map_err(|e| HttpError::DatabaseError(e.to_string()))?;
            p
        }
        Err(e) => return Err(HttpError::DatabaseError(e.to_string())),
    };

    Ok(Json(PageDto::from(page)))
}

/// Get recent pages (most recently updated)
#[instrument(skip(state))]
pub async fn get_recent_pages(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<ListPagesQuery>,
) -> Result<Json<Vec<PageDto>>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let limit = params.limit.unwrap_or(20);

    let pages = page_repo
        .get_recent(limit)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();

    Ok(Json(dtos))
}

/// Search pages by name
#[instrument(skip(state))]
pub async fn search_pages(
    State(state): State<Arc<HttpState>>,
    Query(params): Query<ListPagesQuery>,
) -> Result<Json<Vec<PageDto>>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    // Note: The search query would need to come from the query params
    // For now we return all pages as a fallback
    let limit = params.limit.unwrap_or(50);

    let pages = page_repo
        .search("", limit)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();

    Ok(Json(dtos))
}

/// A backlink displayed to the frontend.
///
/// Represents a block that references the target page or block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkDto {
    /// UUID of the source block (the block that contains the reference)
    pub source_block_id: String,
    /// Name of the page that contains the source block
    pub source_page_name: String,
    /// Content preview of the source block (first ~100 chars)
    pub content_preview: String,
}

/// GET /api/pages/{name}/backlinks
///
/// Returns all blocks that reference the given page.
/// Uses the in-memory RefIndex for O(1) lookup, then enriches
/// results with page names and content previews.
#[instrument(skip(state))]
pub async fn get_page_backlinks(
    State(state): State<Arc<HttpState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<BacklinkDto>>, HttpError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = SqliteBlockRepository::new(state.pool.clone());

    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?
        .ok_or_else(|| HttpError::NotFound(format!("Page not found: {}", name)))?;

    // Query the in-memory ref index for O(1) backlinks
    let backlinks = {
        let ref_service = state.ref_service.read().await;
        ref_service.get_backlinks(page.id)
    };

    if backlinks.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // Enrich backlinks with source block content and page names
    let mut dtos = Vec::with_capacity(backlinks.len());
    for (source_id, _ref_type) in &backlinks {
        if let Ok(Some(source_block)) = block_repo.get_by_id(*source_id).await {
            let source_page_name = page_repo
                .get_by_id(source_block.hierarchy.page_id)
                .await
                .ok()
                .flatten()
                .map(|p| p.name)
                .unwrap_or_else(|| "unknown".to_string());

            let content_preview = if source_block.content.content.len() > 100 {
                format!("{}...", &source_block.content.content[..100])
            } else {
                source_block.content.content.clone()
            };

            dtos.push(BacklinkDto {
                source_block_id: source_id.to_string(),
                source_page_name,
                content_preview,
            });
        }
    }

    Ok(Json(dtos))
}

/// Mount page routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/api/pages", axum::routing::get(list_pages).post(create_page))
        .route("/api/pages/{name}", axum::routing::get(get_page))
        .route("/api/pages/{name}/backlinks", axum::routing::get(get_page_backlinks))
        .route("/api/journal/{date}", axum::routing::get(get_journal))
        .route("/api/pages/recent", axum::routing::get(get_recent_pages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::pages::{CreatePageRequest, ListPagesQuery, PageDto};

    #[test]
    fn test_page_dto_serialization() {
        let dto = PageDto {
            id: "test-id".to_string(),
            name: "Test Page".to_string(),
            title: Some("Test Title".to_string()),
            journal: false,
            journal_day: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"name\":\"Test Page\""));
        assert!(json.contains("\"title\":\"Test Title\""));
        assert!(json.contains("\"journal\":false"));
    }

    #[test]
    fn test_page_dto_journal_serialization() {
        let dto = PageDto {
            id: "journal-id".to_string(),
            name: "2024-01-15".to_string(),
            title: None,
            journal: true,
            journal_day: Some(20240115),
            created_at: "2024-01-15T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"journal\":true"));
        assert!(json.contains("\"journalDay\":20240115"));
    }

    #[test]
    fn test_list_pages_query_defaults() {
        let query = ListPagesQuery { limit: None, offset: None };

        assert!(query.limit.is_none());
        assert!(query.offset.is_none());
    }

    #[test]
    fn test_list_pages_query_with_pagination() {
        let query = ListPagesQuery {
            limit: Some(50),
            offset: Some(10),
        };

        assert_eq!(query.limit, Some(50));
        assert_eq!(query.offset, Some(10));
    }

    #[test]
    fn test_create_page_request_deserialization() {
        let json = r#"{"name":"New Page","title":"New Page Title"}"#;
        let req: CreatePageRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.name, "New Page");
        assert_eq!(req.title, Some("New Page Title".to_string()));
    }

    #[test]
    fn test_create_page_request_without_title() {
        let json = r#"{"name":"Simple Page"}"#;
        let req: CreatePageRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.name, "Simple Page");
        assert!(req.title.is_none());
    }
}
