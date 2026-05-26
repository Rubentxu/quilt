//! Page-related HTTP handlers

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;

use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
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

/// Create page request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePageRequest {
    pub name: String,
    pub title: Option<String>,
}

/// Create router for /api/v1/pages
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_pages).post(create_page))
        .route("/journal/:date", get(get_journal))
        .route("/:name", get(get_page))
        .route("/:name/blocks", get(get_page_blocks))
        .route("/:name/backlinks", get(get_page_backlinks))
}

/// GET /api/v1/pages
#[instrument(skip(state))]
pub async fn list_pages(
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<PageDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let pages = page_repo
        .get_all()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let dtos: Vec<PageDto> = pages.into_iter().map(PageDto::from).collect();

    Ok(Json(dtos))
}

/// GET /api/v1/pages/:name
#[instrument(skip(state))]
pub async fn get_page(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<PageDto>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    match page {
        Some(p) => Ok(Json(PageDto::from(p))),
        None => Err(AppError::NotFound(format!("Page not found: {}", name))),
    }
}

/// POST /api/v1/pages
#[instrument(skip(state))]
pub async fn create_page(
    Extension(state): Extension<AppState>,
    Json(payload): Json<CreatePageRequest>,
) -> Result<(StatusCode, Json<PageDto>), AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());

    let page = Page::new(PageCreate {
        name: payload.name,
        title: payload.title,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .map_err(|e| AppError::Internal(e.to_string()))?;

    page_repo
        .insert(&page)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(PageDto::from(page))))
}

/// GET /api/v1/pages/journal/:date
#[instrument(skip(state))]
pub async fn get_journal(
    Path(date): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<PageDto>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let day = JournalDay::from_str(&date)
        .map_err(|e| AppError::BadRequest(format!("Invalid journal date: {}", e)))?;

    let page = match page_repo.get_journal(day).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = Page::new_journal(day, BlockFormat::Markdown)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            page_repo
                .insert(&p)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?;
            p
        }
        Err(e) => return Err(AppError::Internal(e.to_string())),
    };

    Ok(Json(PageDto::from(page)))
}

/// A backlink DTO returned by the page backlinks endpoint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkDto {
    /// UUID of the source block (the block that contains the reference)
    pub source_block_id: String,
    /// Name of the page that contains the source block
    pub source_page_name: String,
    /// Content preview of the source block (first ~100 chars)
    pub content_preview: String,
}

/// GET /api/v1/pages/:name/backlinks
///
/// Returns all blocks that reference the given page.
/// Uses the in-memory RefIndex for O(1) lookup.
#[instrument(skip(state))]
pub async fn get_page_backlinks(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<BacklinkDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(state.pool.clone());

    // Look up the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

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
        let block = block_repo
            .get_by_id(*source_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if let Some(source_block) = block {
            let source_page_name = page_repo
                .get_by_id(source_block.page_id)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .map(|p| p.name)
                .unwrap_or_else(|| "unknown".to_string());

            let plain_text = source_block.content.as_plain_text();
            let content_preview = if plain_text.len() > 100 {
                format!("{}...", &plain_text[..100])
            } else {
                plain_text
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

/// GET /api/v1/pages/:name/blocks
/// Returns all blocks for a page
#[instrument(skip(state))]
pub async fn get_page_blocks(
    Path(name): Path<String>,
    Extension(state): Extension<AppState>,
) -> Result<Json<Vec<crate::handlers::blocks::BlockDto>>, AppError> {
    let page_repo = SqlitePageRepository::new(state.pool.clone());
    let block_repo = quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository::new(state.pool.clone());

    // Find the page by name
    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("Page not found: {}", name)))?;

    // Get all blocks for this page
    let blocks = block_repo
        .get_by_page(page.id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let block_dtos: Vec<crate::handlers::blocks::BlockDto> = blocks
        .into_iter()
        .map(|b| (b, Some(page.name.clone())).into())
        .collect();

    Ok(Json(block_dtos))
}
