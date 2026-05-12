//! Quilt Tauri application library
//!
//! This module provides the Tauri 2 desktop application shell for Quilt.

pub mod commands;
pub mod deep_link;
pub mod state;

use crate::deep_link::DeepLinkParser;
use crate::state::AppState;
use commands::{
    argument_map, cognitive_available, cognitive_mirror, create_block, create_page, create_task,
    delete_block, get_availability, get_backlinks, get_block_tree, get_journal, get_page,
    link_blocks, list_pages, mental_model, morning_briefing, navigate_to_block, navigate_to_page,
    query_agent, query_blocks, search_blocks, serendipity,
};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use quilt_cognitive::{
    AgentMemory, ArgumentCartographer, CognitiveMirror, CounterfactualExplorer,
    KnowledgeEvolutionTracker, MentalModelGardener, MockAIClient, SerendipityEngine,
};
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteDeepLinkRepository, SqlitePageRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_search::{SearchIndexManager, SearchService};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging for the application
fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Create and configure the Tauri app state
///
/// This function creates the database pool, runs migrations, and sets up
/// the MCP server for the Tauri application.
pub async fn create_app_state(
    db_path: std::path::PathBuf,
) -> Result<AppState, Box<dyn std::error::Error>> {
    info!("Creating database pool at {:?}", db_path);

    // Create database pool
    let pool = create_pool(&db_path).await?;

    // Run migrations
    info!("Running database migrations");
    run_migrations(&pool).await?;

    // Create repositories
    let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
    let tag_repo = Arc::new(SqliteTagRepository::new(pool.clone()));
    let deep_link_repo = Arc::new(SqliteDeepLinkRepository::new(pool.clone()));
    let search_service = Arc::new(SearchService::new(pool.clone()));

    // Create AI client for cognitive engines
    let ai_client: Arc<dyn quilt_cognitive::AIClient> = Arc::new(MockAIClient::new());

    // Create AgentMemory first (needed by MentalModelGardener)
    let agent_memory = Arc::new(AgentMemory::new(block_repo.clone(), ai_client.clone()));

    // Create all cognitive engines
    let cognitive_mirror = Arc::new(CognitiveMirror::new(block_repo.clone(), ai_client.clone()));
    let serendipity_engine = Arc::new(SerendipityEngine::new(
        block_repo.clone(),
        ai_client.clone(),
    ));
    let argument_cartographer = Arc::new(ArgumentCartographer::new(
        block_repo.clone(),
        ai_client.clone(),
    ));
    let counterfactual_explorer = Arc::new(CounterfactualExplorer::new(
        block_repo.clone(),
        ai_client.clone(),
    ));
    let knowledge_evolution_tracker = Arc::new(KnowledgeEvolutionTracker::new(
        block_repo.clone(),
        ai_client.clone(),
    ));
    let mental_model_gardener = Arc::new(MentalModelGardener::new(
        block_repo.clone(),
        agent_memory.clone(),
        ai_client.clone(),
    ));

    // Create MCP server with cognitive engines
    let mcp_server = Arc::new(
        McpServer::new(block_repo, page_repo.clone(), tag_repo, deep_link_repo, search_service)
            .with_cognitive(
            Some(cognitive_mirror),
            Some(serendipity_engine),
            Some(agent_memory),
            Some(argument_cartographer),
            Some(mental_model_gardener),
            Some(counterfactual_explorer),
            Some(knowledge_evolution_tracker),
        ),
    );

    info!("MCP server initialized");

    // Create search index manager for FileWatcher/EventBridge
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));

    Ok(AppState::new(pool, mcp_server, search_index))
}

/// Run the Tauri application
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    // Initialize metrics recorder with Prometheus exporter
    PrometheusBuilder::new()
        .install()
        .expect("Failed to install metrics exporter");

    describe_gauge!("quilt_pages_total", "Total number of pages");
    describe_gauge!("quilt_blocks_total", "Total number of blocks");
    gauge!("quilt_pages_total", 0.0);
    gauge!("quilt_blocks_total", 0.0);

    info!("Starting Quilt Tauri application");

    // Use a simple blocking approach since we can't use async in setup
    let rt = tokio::runtime::Runtime::new()?;

    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Handle deep link from second instance
            // argv[0] is the app path, argv[1] should be the deep link URL
            if argv.len() > 1 {
                let url = &argv[1];
                if url.starts_with("quilt://") {
                    info!("Received deep link from second instance: {}", url);
                    match DeepLinkParser::parse(url) {
                        Ok(target) => {
                            if let Err(e) = app.emit("navigate-to", &target) {
                                tracing::warn!("Failed to emit navigate-to event: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse deep link from second instance: {}", e);
                        }
                    }
                }
            }
        }))
        .setup(move |app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data dir");

            let state = rt.block_on(async { create_app_state(app_data_dir.clone()).await })?;
            info!("App state created, managing in Tauri");
            app.manage(state);

            // Start file watcher and event bridge for external change detection
            let graph_path = app_data_dir.clone();
            std::thread::spawn(move || {
                let rt =
                    tokio::runtime::Runtime::new().expect("Failed to create runtime for watcher");
                rt.block_on(async {
                    use quilt_application::event_bridge::EventBridge;
                    use quilt_platform::watcher::FileWatcher;

                    let mut watcher = FileWatcher::new(vec![graph_path.clone()]);
                    match watcher.start_async().await {
                        Ok(sender) => {
                            info!("File watcher started for {:?}", graph_path);
                            let search_index = quilt_search::SearchIndexManager::new(
                                quilt_infrastructure::database::sqlite::connection::create_pool(
                                    &graph_path.join("quilt.db"),
                                )
                                .await
                                .expect("Failed to create pool for watcher"),
                            );
                            let receiver = sender.subscribe();
                            let bridge = EventBridge::new(search_index, receiver);
                            if let Err(e) = bridge.run().await {
                                tracing::error!("EventBridge error: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to start file watcher: {}", e);
                        }
                    }
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            query_blocks,
            create_block,
            search_blocks,
            get_block_tree,
            delete_block,
            link_blocks,
            get_backlinks,
            create_task,
            get_page,
            list_pages,
            get_journal,
            create_page,
            query_agent,
            cognitive_mirror,
            cognitive_available,
            serendipity,
            argument_map,
            mental_model,
            morning_briefing,
            get_availability,
            navigate_to_page,
            navigate_to_block,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
