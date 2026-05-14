//! Quilt Tauri application library
//!
//! This module provides the Tauri 2 desktop application shell for Quilt.

pub mod commands;
pub mod deep_link;
pub mod state;

use crate::deep_link::DeepLinkParser;
use crate::state::AppState;
use commands::{
    argument_map, cognitive_available, cognitive_mirror, configure_ai_provider,
    create_block, create_page, create_task, delete_block, get_availability, get_ai_status,
    get_backlinks, get_block_tree, get_journal, get_page, link_blocks, list_pages, mental_model,
    morning_briefing, navigate_to_block, navigate_to_page, query_agent, query_blocks, search_blocks,
    serendipity,
};
use metrics::{describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusBuilder;
use quilt_cognitive::{
    create_ai_client, AgentMemory, ArgumentCartographer, CognitiveMirror,
    CounterfactualExplorer, KnowledgeEvolutionTracker, MentalModelGardener, MorningBriefing,
    SerendipityEngine,
};
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::services::TimezoneService;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteDeepLinkRepository, SqliteJournalRepository, SqlitePageRepository,
    SqliteSettingsRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_search::{SearchIndexManager, SearchService};
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Menu action identifiers for frontend event handling
const MENU_ACTION_NEW_PAGE: &str = "menu:new-page";
const MENU_ACTION_OPEN_GRAPH: &str = "menu:open-graph";
const MENU_ACTION_EXIT: &str = "menu:exit";
const MENU_ACTION_TOGGLE_SIDEBAR: &str = "menu:toggle-sidebar";
const MENU_ACTION_ZOOM_IN: &str = "menu:zoom-in";
const MENU_ACTION_ZOOM_OUT: &str = "menu:zoom-out";
const MENU_ACTION_RESET_ZOOM: &str = "menu:reset-zoom";
const MENU_ACTION_ABOUT: &str = "menu:about";
const MENU_ACTION_DOCS: &str = "menu:docs";

/// Build the application menu bar
///
/// Creates menus for File, Edit, View, and Help with standard actions.
/// Menu items emit events to the frontend for handling.
fn build_menu(app: &tauri::AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, tauri::Error> {
    use tauri::menu::{MenuBuilder, SubmenuBuilder, PredefinedMenuItem};

    // File menu: New Page, Open Graph, separator, Exit
    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_NEW_PAGE, "New Page", true, None::<&str>)?)
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_OPEN_GRAPH, "Open Graph...", true, None::<&str>)?)
        .separator()
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_EXIT, "Exit", true, Some("CmdOrCtrl+Q"))?)
        .build()?;

    // Edit menu: Undo, Redo, separator, Cut, Copy, Paste
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, Some("Undo"))?)
        .item(&PredefinedMenuItem::redo(app, Some("Redo"))?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, Some("Cut"))?)
        .item(&PredefinedMenuItem::copy(app, Some("Copy"))?)
        .item(&PredefinedMenuItem::paste(app, Some("Paste"))?)
        .item(&PredefinedMenuItem::select_all(app, Some("Select All"))?)
        .build()?;

    // View menu: Toggle Sidebar, separator, Zoom controls
    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_TOGGLE_SIDEBAR, "Toggle Sidebar", true, Some("CmdOrCtrl+B"))?)
        .separator()
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_ZOOM_IN, "Zoom In", true, Some("CmdOrCtrl+Plus"))?)
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_ZOOM_OUT, "Zoom Out", true, Some("CmdOrCtrl+Minus"))?)
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_RESET_ZOOM, "Reset Zoom", true, Some("CmdOrCtrl+0"))?)
        .build()?;

    // Help menu: About Quilt, Documentation
    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_ABOUT, "About Quilt", true, None::<&str>)?)
        .item(&tauri::menu::MenuItem::with_id(app, MENU_ACTION_DOCS, "Documentation", true, None::<&str>)?)
        .build()?;

    // Build the full menu bar
    let menu = MenuBuilder::new(app)
        .item(&file_menu)
        .item(&edit_menu)
        .item(&view_menu)
        .item(&help_menu)
        .build()?;

    Ok(menu)
}

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
    let settings_repo = Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let journal_repo = Arc::new(SqliteJournalRepository::new(pool.clone()));

    // Create timezone service from user settings (fallback to UTC)
    let user_settings = settings_repo.get_user_settings().await.unwrap_or_default();
    let timezone_service = Arc::new(
        TimezoneService::from_tz_string(&user_settings.timezone)
            .unwrap_or_else(|_| TimezoneService::from_tz_string("UTC").unwrap()),
    );

    // Create AI client for cognitive engines using default config
    let ai_config = quilt_cognitive::AIConfig::default();
    let ai_client: Arc<dyn quilt_cognitive::AIClient> = Arc::from(create_ai_client(&ai_config));

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

    // Create MorningBriefing service (aggregates all cognitive engines)
    let morning_briefing = Arc::new(MorningBriefing::new(
        Some(cognitive_mirror.clone()),
        Some(serendipity_engine.clone()),
        Some(knowledge_evolution_tracker.clone()),
        Some(page_repo.clone()),
        Some(block_repo.clone()),
    ));

    // Create MCP server with cognitive engines
    let mcp_server = Arc::new(
        McpServer::new(block_repo, page_repo.clone(), tag_repo, deep_link_repo, search_service, timezone_service)
            .with_cognitive(
            Some(cognitive_mirror),
            Some(serendipity_engine),
            Some(agent_memory),
            Some(argument_cartographer),
            Some(mental_model_gardener),
            Some(counterfactual_explorer),
            Some(knowledge_evolution_tracker),
        )
            .with_morning_briefing(morning_briefing)
            .with_journal_repo(journal_repo)
            .with_settings_repo(settings_repo),
    );

    info!("MCP server initialized");

    // Create search index manager for FileWatcher/EventBridge
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));

    Ok(AppState::new(pool, mcp_server, search_index, ai_client))
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
            // Build and set the menu bar
            let menu = build_menu(app.handle())?;
            app.set_menu(menu)?;

            // Handle menu events
            app.on_menu_event(move |app, event| {
                let id = event.id().as_ref();
                info!("Menu action triggered: {}", id);

                match id {
                    MENU_ACTION_NEW_PAGE => {
                        if let Err(e) = app.emit("menu-action", "new-page") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_OPEN_GRAPH => {
                        if let Err(e) = app.emit("menu-action", "open-graph") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_EXIT => {
                        info!("Exit menu action, closing application");
                        app.exit(0);
                    }
                    MENU_ACTION_TOGGLE_SIDEBAR => {
                        if let Err(e) = app.emit("menu-action", "toggle-sidebar") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_ZOOM_IN => {
                        if let Err(e) = app.emit("menu-action", "zoom-in") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_ZOOM_OUT => {
                        if let Err(e) = app.emit("menu-action", "zoom-out") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_RESET_ZOOM => {
                        if let Err(e) = app.emit("menu-action", "reset-zoom") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_ABOUT => {
                        if let Err(e) = app.emit("menu-action", "about") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    MENU_ACTION_DOCS => {
                        if let Err(e) = app.emit("menu-action", "docs") {
                            tracing::warn!("Failed to emit menu action: {}", e);
                        }
                    }
                    _ => {
                        tracing::warn!("Unknown menu action: {}", id);
                    }
                }
            });

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
            configure_ai_provider,
            get_ai_status,
        ])
        .run(tauri::generate_context!())?;

    Ok(())
}
