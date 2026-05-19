//! HTTP Server State
//!
//! Shared state that is cloned for each request via `State` extractor.

use std::path::PathBuf;

use sqlx::SqlitePool;
use tokio::sync::broadcast;

use crate::handlers::events::SseBroadcaster;

/// Shared application state for HTTP handlers.
///
/// This struct holds the database connection pool and references to services.
/// It is wrapped in `Arc` and cloned for each request.
#[derive(Clone)]
pub struct HttpState {
    /// SQLite database connection pool
    pub pool: SqlitePool,

    /// Path to the vault directory
    pub vault_path: PathBuf,

    /// MCP server instance for cognitive endpoints (None until properly initialized)
    pub mcp_server: Option<std::sync::Arc<quilt_mcp::McpServer>>,

    /// SSE event broadcaster for real-time updates
    pub sse_broadcaster: SseBroadcaster,
}

impl HttpState {
    /// Create a new HttpState instance
    pub fn new(
        pool: SqlitePool,
        vault_path: PathBuf,
        mcp_server: Option<std::sync::Arc<quilt_mcp::McpServer>>,
    ) -> Self {
        // Create SSE broadcaster with capacity for 1000 events
        let (sender, _) = broadcast::channel(1000);
        let sse_broadcaster = SseBroadcaster::from_sender(sender);

        Self {
            pool,
            vault_path,
            mcp_server,
            sse_broadcaster,
        }
    }
}