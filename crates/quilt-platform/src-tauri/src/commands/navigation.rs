//! Navigation-related Tauri commands
//!
//! These commands handle navigation to pages and blocks within the Quilt app.

use crate::deep_link::DeepLinkTarget;
use crate::state::AppState;
use quilt_domain::entities::Page;
use quilt_domain::repositories::PageRepository;
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

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

/// Create a page repository (helper)
fn create_page_repo(
    pool: &quilt_infrastructure::database::sqlite::connection::DbPool,
) -> SqlitePageRepository {
    SqlitePageRepository::new(pool.clone())
}

/// Navigate to a specific page
///
/// This command updates the last_opened_graph if a graph_id is provided,
/// and emits a navigate-to event to the frontend.
#[tauri::command]
pub async fn navigate_to_page(
    graph_id: Option<String>,
    page_name: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<PageDto, String> {
    // Update last_opened_graph if graph_id is provided
    if let Some(ref gid) = graph_id {
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(gid.clone());
    }

    // Get the page
    let page_repo = create_page_repo(&state.pool);
    let page = page_repo
        .get_by_name(&page_name)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Page not found: {}", page_name))?;

    // Emit navigate-to event
    let target = DeepLinkTarget::Page {
        graph_id,
        page_name: page_name.clone(),
    };
    app.emit("navigate-to", &target)
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    Ok(PageDto::from(page))
}

/// Navigate to a specific block on a page
///
/// This command updates the last_opened_graph if a graph_id is provided,
/// and emits a navigate-to event to the frontend.
#[tauri::command]
pub async fn navigate_to_block(
    graph_id: Option<String>,
    page_name: String,
    block_uuid: String,
    state: State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    // Validate UUID
    let _uuid = uuid::Uuid::parse_str(&block_uuid)
        .map_err(|_| format!("Invalid block UUID: {}", block_uuid))?;

    // Update last_opened_graph if graph_id is provided
    if let Some(ref gid) = graph_id {
        let mut last_graph = state.last_opened_graph.write().await;
        *last_graph = Some(gid.clone());
    }

    // Verify page exists
    let page_repo = create_page_repo(&state.pool);
    let _page = page_repo
        .get_by_name(&page_name)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Page not found: {}", page_name))?;

    // Emit navigate-to event
    let target = DeepLinkTarget::Block {
        graph_id,
        page_name: page_name.clone(),
        block_uuid: _uuid,
    };
    app.emit("navigate-to", &target)
        .map_err(|e| format!("Failed to emit event: {}", e))?;

    Ok(())
}
