//! Quilt HTTP Server
//!
//! HTTP REST API and WebSocket endpoints for web deployment.
//! Replaces Tauri's IPC with HTTP endpoints and WebSocket for MCP proxy.
//!
//! # Architecture
//!
//! - `state.rs` - Shared application state (database pool, services)
//! - `error.rs` - HTTP error types and response mapping
//! - `handlers/` - Route handlers for blocks, pages, search, graph, cognitive, events
//! - `mcp_ws.rs` - WebSocket MCP proxy endpoint
//! - `polling.rs` - File system polling service for change detection
//!
//! # Endpoints
//!
//! - `GET /health` - Health check
//! - `GET /api/blocks` - Query blocks
//! - `POST /api/blocks` - Create block
//! - `GET /api/blocks/:id` - Get block
//! - `GET /api/pages` - List pages
//! - `GET /api/pages/:name` - Get page
//! - `GET /api/search` - Search
//! - `GET /api/graph` - Graph data
//! - `GET /api/events` - SSE event stream
//! - `WS /ws/mcp` - MCP WebSocket proxy

pub mod error;
pub mod handlers;
pub mod mcp_ws;
pub mod polling;
pub mod server;
pub mod state;

pub use state::HttpState;
pub use server::run_http_server;
