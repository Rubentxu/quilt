//! HTTP Server State
//!
//! Shared state that is cloned for each request via `State` extractor.

use std::path::PathBuf;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};

use crate::handlers::events::SseBroadcaster;
use quilt_application::services::ref_service::RefService;

/// Shared application state for HTTP handlers.
///
/// This struct holds the database connection pool, services, and references.
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

    /// Bidirectional reference service for O(1) backlink queries
    pub ref_service: std::sync::Arc<RwLock<RefService>>,
}

impl HttpState {
    /// Create a new HttpState instance
    pub fn new(
        pool: SqlitePool,
        vault_path: PathBuf,
        mcp_server: Option<std::sync::Arc<quilt_mcp::McpServer>>,
        ref_service: std::sync::Arc<RwLock<RefService>>,
    ) -> Self {
        // Create SSE broadcaster with capacity for 1000 events
        let (sender, _) = broadcast::channel(1000);
        let sse_broadcaster = SseBroadcaster::from_sender(sender);

        Self {
            pool,
            vault_path,
            mcp_server,
            sse_broadcaster,
            ref_service,
        }
    }
}