//! Page-related Tauri commands

use crate::state::AppState;
use quilt_domain::entities::{Page, PageCreate};
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::{BlockFormat, JournalDay};
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tauri::State;

/// Create a page repository (helper)
pub fn create_page_repo(
    pool: &quilt_infrastructure::database::sqlite::connection::DbPool,
) -> SqlitePageRepository {
    SqlitePageRepository::new(pool.clone())
}

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

/// Get a page by name
#[tauri::command]
pub async fn get_page(name: String, state: State<'_, AppState>) -> Result<Option<PageDto>, String> {
    let page_repo = create_page_repo(&state.pool);

    let page = page_repo
        .get_by_name(&name)
        .await
        .map_err(|e| e.to_string())?;

    Ok(page.map(PageDto::from))
}

/// List all pages
#[tauri::command]
pub async fn list_pages(state: State<'_, AppState>) -> Result<Vec<PageDto>, String> {
    let page_repo = create_page_repo(&state.pool);

    let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;

    Ok(pages.into_iter().map(PageDto::from).collect())
}

/// Get or create a journal page for a date
#[tauri::command]
pub async fn get_journal(date: String, state: State<'_, AppState>) -> Result<PageDto, String> {
    let page_repo = create_page_repo(&state.pool);
    let day = JournalDay::from_str(&date).map_err(|e| e.to_string())?;

    let page = match page_repo.get_journal(day).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = Page::new_journal(day, BlockFormat::Markdown).map_err(|e| e.to_string())?;
            page_repo.insert(&p).await.map_err(|e| e.to_string())?;
            p
        }
        Err(e) => return Err(e.to_string()),
    };

    Ok(PageDto::from(page))
}

/// Create a new page
#[tauri::command]
pub async fn create_page(
    name: String,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<PageDto, String> {
    let page_repo = create_page_repo(&state.pool);

    let page = Page::new(PageCreate {
        name,
        title,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .map_err(|e| e.to_string())?;

    page_repo.insert(&page).await.map_err(|e| e.to_string())?;

    Ok(PageDto::from(page))
}
