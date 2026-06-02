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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod error;
mod handlers;
mod middleware;
mod routes;
mod state;

use crate::handlers::metrics;
use crate::state::AppState;
use quilt_application::services::ref_service::RefService;
#[cfg(feature = "cognitive")]
use quilt_cognitive::AIClient;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;

/// Ensure the vault directory structure exists (.quilt folder and quilt.db)
fn ensure_vault_exists(vault_path: &Path) -> Result<PathBuf, anyhow::Error> {
    let quilt_dir = vault_path.join(".quilt");
    let db_path = quilt_dir.join("quilt.db");

    if !quilt_dir.exists() {
        std::fs::create_dir_all(&quilt_dir)?;
        tracing::info!("Created .quilt directory at {:?}", quilt_dir);
    }

    if !db_path.exists() {
        std::fs::write(&db_path, "")?;
        tracing::info!("Created database file at {:?}", db_path);
    }

    Ok(db_path)
}

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

    // Initialize vault directory and database pool
    let db_path = ensure_vault_exists(&vault_path)?;
    let pool = create_pool(&db_path).await?;
    run_migrations(&pool).await?;

    info!("Vault ready at {:?}", vault_path);
    info!("Database pool created");

    // Create AppState
    let search_index = Arc::new(quilt_search::SearchIndexManager::new(pool.clone()));

    // Initialize bidirectional reference service
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let mut ref_service = RefService::new(ref_repo);
    ref_service
        .rebuild_from_repo()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to rebuild reference index: {}", e))?;
    info!(
        "Reference index rebuilt with {} entries",
        ref_service.index().len()
    );
    let ref_service = Arc::new(RwLock::new(ref_service));

    // Generate or load API key for Bearer token auth
    let api_key = match std::env::var("QUILT_API_KEY") {
        Ok(key) if !key.is_empty() => {
            info!("Using API key from QUILT_API_KEY env var");
            key
        }
        _ => {
            use uuid::Uuid;
            let key = Uuid::new_v4().to_string();
            info!("Generated API key: {key} — set QUILT_API_KEY env var for a custom key");
            key
        }
    };
    middleware::auth::init(api_key);

    // Build state — with or without AI client depending on feature flag
    #[cfg(feature = "cognitive")]
    let ai_client: Arc<dyn AIClient> = Arc::new(quilt_cognitive::ai_client::MockAIClient::new());

    let state = {
        #[cfg(feature = "cognitive")]
        {
            AppState::with_ai_client(pool, search_index, ai_client, ref_service)
        }
        #[cfg(not(feature = "cognitive"))]
        {
            AppState::new(pool, search_index, ref_service)
        }
    };

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
