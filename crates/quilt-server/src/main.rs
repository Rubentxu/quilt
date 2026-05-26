//! Quilt Server — HTTP API Server with Axum
//!
//! This binary starts an Axum HTTP server that serves:
//! - REST API at `/api/v1/*`
//! - WebSocket at `/ws`
//! - Health check at `/health`
//! - Prometheus metrics at `/metrics` (when QUILT_METRICS=true)
//! - Embedded frontend at `/*`

use anyhow::Result;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::RwLock;

mod error;
mod handlers;
mod routes;
mod state;

use crate::handlers::metrics;
use crate::state::AppState;
use quilt_application::services::ref_service::RefService;
use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;

#[tokio::main]
async fn main() -> Result<()> {
    // Get configuration from environment
    let vault_path = std::env::var("QUILT_GRAPH_DIR")
        .or_else(|_| std::env::var("QUILT_VAULT_PATH"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let port: u16 = std::env::var("QUILT_PORT")
        .unwrap_or_else(|_| "3737".to_string())
        .parse()
        .expect("QUILT_PORT must be a valid port number");

    // Initialize logging with configurable level
    let log_filter = std::env::var("QUILT_LOG")
        .unwrap_or_else(|_| "quilt_server=info,tower_http=info".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_filter)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Quilt server initialization");
    info!("Vault path: {:?}", vault_path);

    // Initialize Prometheus metrics (if enabled)
    let _metrics_handle = metrics::init_metrics();

    // Initialize state using the platform initialization
    let init = quilt_platform::init::HttpServerInit::new(vault_path)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize: {}", e))?;

    info!("Vault ready at {:?}", init.vault_config.vault_path);
    info!("Database pool created");

    // Create AppState
    let search_index = Arc::new(quilt_search::SearchIndexManager::new(init.pool.clone()));
    let ai_client: Arc<dyn quilt_cognitive::AIClient> =
        Arc::new(quilt_cognitive::ai_client::MockAIClient::new());

    // Initialize bidirectional reference service
    let ref_repo = Arc::new(SqliteRefRepository::new(init.pool.clone()));
    let mut ref_service = RefService::new(ref_repo);
    ref_service
        .rebuild_from_repo()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to rebuild reference index: {}", e))?;
    info!("Reference index rebuilt with {} entries", ref_service.index().len());
    let ref_service = Arc::new(RwLock::new(ref_service));

    let state = AppState::new(init.pool, search_index, ai_client, ref_service);

    // Create router
    let app = routes::create_app(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    info!(
        port = port,
        "Starting Quilt server on http://localhost:{port}"
    );
    info!("API endpoints available at /api/v1/*");
    info!("WebSocket available at /ws");
    info!("Health check at /health");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

// Required for SearchIndexManager::new
use std::sync::Arc;
