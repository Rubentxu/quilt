//! Quilt MCP Server Binary
//!
//! Standalone MCP server that exposes the Quilt knowledge graph via
//! the Model Context Protocol over stdio.
//!
//! Usage:
//!     quilt-mcp
//!
//! ## Environment (ADR-0030)
//!
//! - `QUILT_GRAPH_DIR` — canonical, points at the graph root directory.
//!   The canonical database is `<graph-root>/.quilt/quilt.db`.
//! - `QUILT_DB_PATH` — **deprecated**, kept for one release. If set,
//!   the parent of the `.db` is used as the graph root and a warning
//!   is emitted to stderr.

use anyhow::Result;
use quilt_cognitive::{
    AgentMemory, ArgumentCartographer, CognitiveMirror, CounterfactualExplorer,
    KnowledgeEvolutionTracker, MentalModelGardener, MockAIClient, SerendipityEngine,
};
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::services::TimezoneService;
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteDeepLinkRepository, SqliteJournalRepository, SqlitePageRepository,
    SqliteSettingsRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_platform::init::init_graph;
use quilt_platform::mcp_transport::StdioTransport;
use quilt_search::SearchService;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // Resolve the canonical graph root, with deprecation fallback
    // for `QUILT_DB_PATH`. ADR-0030:
    //   1. QUILT_GRAPH_DIR (canonical)
    //   2. QUILT_DB_PATH  (deprecated: parent used as graph root)
    //   3. cwd
    let graph_path = match std::env::var("QUILT_GRAPH_DIR") {
        Ok(v) => PathBuf::from(v),
        Err(_) => match std::env::var("QUILT_DB_PATH") {
            Ok(v) => {
                eprintln!(
                    "warning: QUILT_DB_PATH is deprecated; use QUILT_GRAPH_DIR \
                     (will be removed in next minor release, see ADR-0030)"
                );
                let p = PathBuf::from(v);
                if p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("db"))
                    .unwrap_or(false)
                {
                    p.parent().map(|x| x.to_path_buf()).unwrap_or(p)
                } else {
                    p
                }
            }
            Err(_) => PathBuf::from("."),
        },
    };

    // Canonical graph bootstrap. Per ADR-0030 the layout is created on
    // first open (no auto-create if missing — fail explicitly).
    let graph_config = match init_graph(graph_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: failed to initialize Graph Space: {e}");
            eprintln!("Hint: ensure the graph directory exists and is writable.");
            std::process::exit(1);
        }
    };

    // Create connection pool and run migrations
    let pool = connection::create_pool(&graph_config.db_path).await?;
    connection::run_migrations(&pool).await?;

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

    // Create MCP server with all cognitive engines
    let mcp_server = Arc::new(
        McpServer::new(
            block_repo,
            page_repo,
            tag_repo,
            deep_link_repo,
            search_service,
            timezone_service,
        )
        .with_cognitive(
            Some(cognitive_mirror),
            Some(serendipity_engine),
            Some(agent_memory),
            Some(argument_cartographer),
            Some(mental_model_gardener),
            Some(counterfactual_explorer),
            Some(knowledge_evolution_tracker),
        )
        .with_journal_repo(journal_repo)
        .with_settings_repo(settings_repo),
    );

    // Run the MCP server
    StdioTransport::serve(mcp_server).await?;

    Ok(())
}
