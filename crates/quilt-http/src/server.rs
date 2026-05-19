//! HTTP Server Runtime
//!
//! This module provides the HTTP server runtime that can be shared between
//! different entry points.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use axum::{routing::get, Json, Router};
use sqlx::SqlitePool;
use tokio::signal;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::mcp_ws::ws_mcp_handler;
use crate::polling::{self, FileChangeEvent, PollingConfig};
use crate::state::HttpState;

/// Health check response
#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Health check endpoint
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Create the Axum router with all routes
fn create_app(state: Arc<HttpState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_check))
        .merge(handlers::blocks_routes())
        .merge(handlers::pages_routes())
        .merge(handlers::search_routes())
        .merge(handlers::graph_routes())
        .merge(handlers::cognitive_routes())
        .merge(handlers::events_routes())
        .route("/ws/mcp", axum::routing::get(ws_mcp_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

/// Run the HTTP server with the given configuration
///
/// This function is used by both the main binary and the CLI entry point.
pub async fn run_http_server(
    pool: SqlitePool,
    vault_path: PathBuf,
    mcp_server: Option<Arc<quilt_mcp::McpServer>>,
    host: &str,
    port: u16,
) -> anyhow::Result<()> {
    // Create HTTP state with SSE broadcaster
    let state = Arc::new(HttpState::new(pool, vault_path.clone(), mcp_server));

    // Start polling service if vault path exists
    if vault_path.exists() {
        let poll_config = PollingConfig::default();
        let (file_event_tx, _) = broadcast::channel::<FileChangeEvent>(100);

        // Subscribe to file change events and forward to SSE
        let sse_broadcaster = state.sse_broadcaster.clone();
        let mut file_rx = file_event_tx.subscribe();

        // Spawn task to forward file events to SSE
        tokio::spawn(async move {
            loop {
                match file_rx.recv().await {
                    Ok(event) => {
                        let sse_event = crate::handlers::events::SseEvent::from(event);
                        if let Err(e) = sse_broadcaster.send(sse_event) {
                            tracing::debug!("Failed to forward file event to SSE: {}", e);
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(_) => continue,
                }
            }
        });

        // Create and start polling service
        let polling_service = polling::create(vault_path.clone(), file_event_tx, poll_config);
        let polling_service_clone = polling_service.clone();
        tokio::spawn(async move {
            polling_service_clone.run().await;
        });

        tracing::info!("File polling service started for {:?}", vault_path);
    }

    // Create app
    let app = create_app(state);

    // Start server
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .context("Invalid socket address")?;
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Graceful shutdown handling
    let serve = axum::serve(listener, app);
    let serve = serve.with_graceful_shutdown(async {
        match signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Received Ctrl+C, shutting down gracefully...");
            }
            Err(e) => {
                tracing::error!("Failed to listen for Ctrl+C: {}", e);
            }
        }
    });

    serve.await?;

    tracing::info!("Server shut down");
    Ok(())
}