//! Application state for Tauri
//!
//! Holds the database pool and MCP server reference for use by Tauri commands.

use quilt_infrastructure::database::sqlite::connection::DbPool;
use quilt_mcp::McpServer;
use quilt_search::SearchIndexManager;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Application state managed by Tauri
///
/// This struct is stored in Tauri app state and accessed by command handlers.
pub struct AppState {
    /// SQLite database connection pool
    pub pool: DbPool,
    /// MCP server for agent interactions
    pub mcp_server: Arc<McpServer>,
    /// Search index manager for FTS5 index maintenance
    pub search_index: Arc<SearchIndexManager>,
    /// Last opened graph ID (for deep link navigation)
    pub last_opened_graph: RwLock<Option<String>>,
}

impl AppState {
    /// Create a new AppState
    pub fn new(pool: DbPool, mcp_server: Arc<McpServer>, search_index: Arc<SearchIndexManager>) -> Self {
        Self {
            pool,
            mcp_server,
            search_index,
            last_opened_graph: RwLock::new(None),
        }
    }
}
