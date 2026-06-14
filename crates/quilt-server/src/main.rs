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
use quilt_application::use_cases::{
    BlockUseCases, BlockUseCasesImpl, PageUseCases, PageUseCasesImpl, ResourceUseCases,
    ResourceUseCasesImpl, SearchUseCasesImpl, TemplateUseCases, TemplateUseCasesImpl, TourStateUseCases,
    TourStateUseCasesImpl,
};
use quilt_application::AppServices;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqlitePropertyRepository, SqliteRefRepository,
    SqliteRelationRepository, SqliteSchemaRepository, SqliteSettingsRepository, SqliteTagRepository,
    SqliteTourStateRepository,
};
use quilt_search::SearchService;

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

    // Create all repository instances as Arc<dyn Trait>
    let block_repo: Arc<SqliteBlockRepository> = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo: Arc<SqlitePageRepository> = Arc::new(SqlitePageRepository::new(pool.clone()));
    let ref_repo: Arc<SqliteRefRepository> = Arc::new(SqliteRefRepository::new(pool.clone()));
    let settings_repo: Arc<SqliteSettingsRepository> =
        Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let tag_repo: Arc<SqliteTagRepository> = Arc::new(SqliteTagRepository::new(pool.clone()));
    let relation_repo: Arc<SqliteRelationRepository> =
        Arc::new(SqliteRelationRepository::new(pool.clone()));
    let schema_repo: Arc<SqliteSchemaRepository> = Arc::new(SqliteSchemaRepository::new(pool.clone()));
    let property_repo: Arc<SqlitePropertyRepository> =
        Arc::new(SqlitePropertyRepository::new(pool.clone()));
    let tour_state_repo: Arc<SqliteTourStateRepository> =
        Arc::new(SqliteTourStateRepository::new(pool.clone()));

    // Create search index manager
    let search_index = Arc::new(quilt_search::SearchIndexManager::new(pool.clone()));

    // Create search service
    let search_service: Arc<SearchService> = Arc::new(SearchService::new(Arc::new(pool.clone())));

    // Initialize bidirectional reference service
    let mut ref_service = RefService::new(ref_repo.clone());
    ref_service
        .rebuild_from_repo()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to rebuild reference index: {}", e))?;
    info!(
        "Reference index rebuilt with {} entries",
        ref_service.index().len()
    );
    let ref_service = Arc::new(RwLock::new(ref_service));

    // Create use cases
    let block_use_cases: Arc<dyn BlockUseCases> =
        Arc::new(BlockUseCasesImpl::new(block_repo.clone(), page_repo.clone()));
    let page_use_cases: Arc<dyn PageUseCases> =
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
    let resource_use_cases: Arc<dyn ResourceUseCases> = Arc::new(ResourceUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        tag_repo.clone(),
    ));
    let template_use_cases: Arc<dyn TemplateUseCases> =
        Arc::new(TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
    let tour_state_use_cases: Arc<dyn TourStateUseCases> =
        Arc::new(TourStateUseCasesImpl::new(tour_state_repo.clone()));
    let search_use_cases = Arc::new(
        SearchUseCasesImpl::new()
            .with_search_service(search_service.clone())
            .with_block_repo(block_repo.clone()),
    );

    let services = AppServices::new(
        block_use_cases,
        page_use_cases,
        search_use_cases,
        resource_use_cases,
        template_use_cases,
        tour_state_use_cases,
    );
    let services = Arc::new(services);

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

    let state = AppState::new_with_repos(
        block_repo,
        page_repo,
        ref_repo,
        settings_repo,
        tag_repo,
        relation_repo,
        schema_repo,
        property_repo,
        tour_state_repo,
        search_service,
        search_index,
        ref_service,
        services,
    );

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
