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

use crate::error::AppError;
use crate::state::AppState;
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::{BlockFormat, JournalDay};
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
