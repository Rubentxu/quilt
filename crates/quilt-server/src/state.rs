//! Application state for the Axum HTTP server
//!
//! Holds the database pool, MCP server, search index, and other shared resources.

use quilt_application::services::ref_service::RefService;
use quilt_infrastructure::database::sqlite::connection::DbPool;
use quilt_search::SearchIndexManager;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Navigation event sent to WebSocket clients
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEvent {
    pub event_type: String,
    pub target: NavigationTarget,
}

/// Target of a navigation event
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationTarget {
    #[serde(rename = "type")]
    pub target_type: String,
    pub graph_id: Option<String>,
    pub page_name: String,
    pub block_uuid: Option<String>,
}

impl NavigationEvent {
    /// Create a page navigation event
    pub fn page(graph_id: Option<String>, page_name: String) -> Self {
        Self {
            event_type: "navigate-to".to_string(),
            target: NavigationTarget {
                target_type: "Page".to_string(),
                graph_id,
                page_name,
                block_uuid: None,
            },
        }
    }

    /// Create a block navigation event
    pub fn block(graph_id: Option<String>, page_name: String, block_uuid: String) -> Self {
        Self {
            event_type: "navigate-to".to_string(),
            target: NavigationTarget {
                target_type: "Block".to_string(),
                graph_id,
                page_name,
                block_uuid: Some(block_uuid),
            },
        }
    }
}

/// Application state shared across all HTTP handlers
///
/// This is passed to handlers via `Extension<AppState>` in Axum.
#[derive(Clone)]
pub struct AppState {
    /// SQLite database connection pool
    pub pool: DbPool,
    /// Settings repository for user preferences
    pub settings_repo:
        quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository,
    /// Search index manager for FTS5 index maintenance
    #[allow(dead_code)]
    pub search_index: Arc<SearchIndexManager>,
    /// Broadcast sender for navigation events (WebSocket)
    pub navigation_tx: broadcast::Sender<NavigationEvent>,
    /// Last opened graph ID (for deep link navigation)
    pub last_opened_graph: Arc<RwLock<Option<String>>>,
    /// Bidirectional reference service for O(1) backlink queries
    pub ref_service: Arc<RwLock<RefService>>,
}

impl AppState {
    /// Create a new AppState
    ///
    /// Initializes database pool, search index, reference service.
    #[allow(dead_code)]
    pub fn new(
        pool: DbPool,
        search_index: Arc<SearchIndexManager>,
        ref_service: Arc<RwLock<RefService>>,
    ) -> Self {
        // Create broadcast channel for navigation events
        let (navigation_tx, _) = broadcast::channel(100);

        let settings_repo =
            quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository::new(
                pool.clone(),
            );

        Self {
            pool,
            settings_repo,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
        }
    }

    /// Broadcast a navigation event to all WebSocket subscribers
    pub fn broadcast_navigation(&self, event: NavigationEvent) -> anyhow::Result<()> {
        self.navigation_tx
            .send(event)
            .map_err(|_| anyhow::anyhow!("No active WebSocket subscribers"))?;
        Ok(())
    }
}
