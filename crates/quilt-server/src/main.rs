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

mod error;
mod handlers;
mod middleware;
mod routes;
mod state;

use crate::handlers::metrics;
use crate::state::{AppState, RepositoryBundle};
use quilt_application::AppServices;
use quilt_application::services::presets::StaticPresetRegistry;
use quilt_application::services::projection::StaticProjectionRegistry;
use quilt_application::services::ref_service::{RefService, RefServiceTrait};
use quilt_application::use_cases::projection_resolver::ProjectionResolver;
use quilt_application::use_cases::{
    AnnotationUseCases, AnnotationUseCasesImpl, BlockUseCases, BlockUseCasesImpl, PageUseCases,
    PageUseCasesImpl, ResourceUseCases, ResourceUseCasesImpl, SearchUseCasesImpl, TemplateUseCases,
    TemplateUseCasesImpl, TourStateUseCases, TourStateUseCasesImpl,
};
use quilt_domain::canonicalization::PresetRegistry;
use quilt_infrastructure::database::sqlite::SqliteAnnotationRepository;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteGraphSpaceRepository, SqlitePageRepository,
    SqlitePropertyRepository, SqliteRefRepository, SqliteRelationRepository,
    SqliteSchemaRepository, SqliteSettingsRepository, SqliteTagRepository,
    SqliteTourStateRepository,
};
use quilt_platform::init::init_graph;
use quilt_search::SearchService;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging FIRST so that any subsequent
    // tracing::warn! for deprecated env-var usage is captured.
    let log_filter = std::env::var("QUILT_LOG")
        .unwrap_or_else(|_| "quilt_server=info,tower_http=info".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&log_filter)),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Get configuration from environment.
    // Precedence (per ADR-0030, Slice A):
    //   1. QUILT_GRAPH_DIR (canonical)
    //   2. QUILT_VAULT_PATH (deprecated alias — emits tracing::warn! on use)
    //   3. cwd (".")
    let (graph_path, vault_path_was_used) = match std::env::var("QUILT_GRAPH_DIR") {
        Ok(v) => (PathBuf::from(v), false),
        Err(_) => match std::env::var("QUILT_VAULT_PATH") {
            Ok(v) => {
                tracing::warn!(
                    target: "quilt_server::deprecation",
                    "QUILT_VAULT_PATH is deprecated; use QUILT_GRAPH_DIR (will be removed in next minor release, see ADR-0030)"
                );
                (PathBuf::from(v), true)
            }
            Err(_) => (PathBuf::from("."), false),
        },
    };
    // Quiet the warning flag — kept for observability hooks in future.
    let _ = vault_path_was_used;

    let port: u16 = std::env::var("QUILT_PORT")
        .unwrap_or_else(|_| "3737".to_string())
        .parse()
        .expect("QUILT_PORT must be a valid port number");

    info!("Starting Quilt server initialization");
    info!("Graph path: {:?}", graph_path);

    // Initialize Prometheus metrics (if enabled)
    let _metrics_handle = metrics::init_metrics();

    // Open the cross-graph app state (ADR-0030 §5). The factory is
    // fail-open: any disk error falls back to an in-memory store
    // with a `tracing::warn!` so the server starts no matter what.
    let global_state_repo = quilt_platform::global_app_state::open_global_state().await;
    let global_initial = global_state_repo.load().await.unwrap_or_else(|e| {
        tracing::warn!(
            target: "quilt_server",
            "failed to load global state at startup; using defaults: {e}"
        );
        quilt_domain::entities::GlobalAppState::default()
    });
    info!(
        last_opened = ?global_initial.last_opened_graph,
        recents = global_initial.recent_graphs.len(),
        "Cross-graph app state loaded"
    );

    // Canonical graph bootstrap (ADR-0030). Replaces the local ensure_vault_exists.
    let graph_config = init_graph(graph_path)?;
    let pool = create_pool(&graph_config.db_path).await?;
    run_migrations(&pool).await?;

    info!("Graph ready at {:?}", graph_config.graph_path);
    info!("Database pool created");

    // Create all repository instances as Arc<dyn Trait>
    let annotation_repo: Arc<SqliteAnnotationRepository> =
        Arc::new(SqliteAnnotationRepository::new(pool.clone()));
    let block_repo: Arc<SqliteBlockRepository> = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo: Arc<SqlitePageRepository> = Arc::new(SqlitePageRepository::new(pool.clone()));
    let ref_repo: Arc<SqliteRefRepository> = Arc::new(SqliteRefRepository::new(pool.clone()));
    let settings_repo: Arc<SqliteSettingsRepository> =
        Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let graph_space_repo: Arc<SqliteGraphSpaceRepository> =
        Arc::new(SqliteGraphSpaceRepository::new(pool.clone()));
    let tag_repo: Arc<SqliteTagRepository> = Arc::new(SqliteTagRepository::new(pool.clone()));
    let relation_repo: Arc<SqliteRelationRepository> =
        Arc::new(SqliteRelationRepository::new(pool.clone()));
    let schema_repo: Arc<SqliteSchemaRepository> =
        Arc::new(SqliteSchemaRepository::new(pool.clone()));
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
    info!("Reference index rebuilt with {} entries", ref_service.len());
    let ref_service: Arc<dyn RefServiceTrait> = Arc::new(ref_service);

    // Create use cases
    let annotation_use_cases: Arc<dyn AnnotationUseCases> =
        Arc::new(AnnotationUseCasesImpl::new(annotation_repo.clone()));
    let block_use_cases: Arc<dyn BlockUseCases> = Arc::new(BlockUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        ref_service.clone(),
    ));
    let page_use_cases: Arc<dyn PageUseCases> =
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
    let resource_use_cases: Arc<dyn ResourceUseCases> = Arc::new(ResourceUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        tag_repo.clone(),
    ));
    let template_use_cases: Arc<dyn TemplateUseCases> = Arc::new(TemplateUseCasesImpl::new(
        page_repo.clone(),
        block_repo.clone(),
    ));
    let tour_state_use_cases: Arc<dyn TourStateUseCases> =
        Arc::new(TourStateUseCasesImpl::new(tour_state_repo.clone()));
    let search_use_cases = Arc::new(
        SearchUseCasesImpl::new()
            .with_search_service(search_service.clone())
            .with_block_repo(block_repo.clone()),
    );

    let services = AppServices::new(
        annotation_use_cases,
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

    // Bundle all repositories
    let repos = RepositoryBundle::new(
        annotation_repo,
        block_repo,
        page_repo,
        ref_repo,
        settings_repo,
        graph_space_repo,
        tag_repo,
        relation_repo,
        schema_repo,
        property_repo,
        tour_state_repo,
    );

    // Create projection resolver and preset registry
    let projection_resolver = Arc::new(ProjectionResolver::new(StaticProjectionRegistry::v1()));
    let preset_registry: Arc<dyn PresetRegistry> = Arc::new(StaticPresetRegistry::v1());

    let state = AppState::new_with_repos_and_agents_and_global(
        repos,
        search_service,
        search_index,
        ref_service,
        services,
        projection_resolver,
        preset_registry,
        None,
        None,
        Some(global_state_repo),
        global_initial,
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
