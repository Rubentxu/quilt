//! Shared initialization for Quilt application state
//!
//! This module provides common initialization logic that can be shared between
//! different entry points (Tauri desktop app, HTTP server, CLI).

use std::path::PathBuf;
use std::sync::Arc;

use quilt_cognitive::{
    AgentMemory, ArgumentCartographer, CognitiveMirror, CounterfactualExplorer,
    KnowledgeEvolutionTracker, MentalModelGardener, MorningBriefing, SerendipityEngine,
};
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::services::TimezoneService;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations, DbPool};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteDeepLinkRepository, SqliteJournalRepository,
    SqlitePageRepository, SqliteSettingsRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_search::SearchService;

/// Configuration for vault initialization
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Path to the vault directory
    pub vault_path: PathBuf,
    /// Path to the database file (inside vault/.quilt/)
    pub db_path: PathBuf,
}

impl VaultConfig {
    /// Get the database URL for SQLx
    pub fn database_url(&self) -> String {
        format!("sqlite:{}?mode=rwc", self.db_path.display())
    }
}

/// Vault setup errors
#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("Failed to create vault directory: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("Failed to serialize vault config: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(#[from] anyhow::Error),
}

/// Ensure the vault directory structure exists (.quilt folder and quilt.db)
pub fn ensure_vault_exists(vault_path: &PathBuf) -> Result<PathBuf, VaultError> {
    let quilt_dir = vault_path.join(".quilt");
    let db_path = quilt_dir.join("quilt.db");

    // Create .quilt directory if it doesn't exist
    if !quilt_dir.exists() {
        std::fs::create_dir_all(&quilt_dir)?;
        tracing::info!("Created .quilt directory at {:?}", quilt_dir);
    }

    // Create empty database file if it doesn't exist
    // (sqlx will run migrations to create tables)
    if !db_path.exists() {
        std::fs::write(&db_path, "")?;
        tracing::info!("Created database file at {:?}", db_path);
    }

    Ok(db_path)
}

/// Initialize vault from a path, creating necessary structure
pub fn init_vault(vault_path: PathBuf) -> Result<VaultConfig, VaultError> {
    let db_path = ensure_vault_exists(&vault_path)?;

    Ok(VaultConfig {
        vault_path,
        db_path,
    })
}

/// Create database pool and run migrations
pub async fn create_db_pool(db_path: &PathBuf) -> Result<DbPool, VaultError> {
    let pool = create_pool(db_path).await?;
    run_migrations(&pool).await?;
    Ok(pool)
}

/// Initialize MCP server with all cognitive engines
///
/// This creates the full MCP server setup including:
/// - All repositories (blocks, pages, tags, deep_links, settings, journal)
/// - Search service
/// - Timezone service
/// - AI client and all cognitive engines
/// - Morning briefing aggregation
pub async fn init_mcp_server(pool: DbPool) -> Result<Arc<McpServer>, Box<dyn std::error::Error + Send + Sync>> {
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
    let ai_client: Arc<dyn quilt_cognitive::AIClient> = Arc::from(quilt_cognitive::create_ai_client(&ai_config));

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
    let cognitive_services: Arc<dyn quilt_cognitive::MorningBriefingServices> =
        Arc::new(quilt_cognitive::DefaultMorningBriefingServices::new(
            cognitive_mirror.clone(),
            serendipity_engine.clone(),
            knowledge_evolution_tracker.clone(),
        ));
    let morning_briefing = Arc::new(MorningBriefing::new(
        cognitive_services,
        Some(page_repo.clone()),
        Some(block_repo.clone()),
    ));

    // Create MCP server with cognitive engines
    let mcp_server = Arc::new(
        McpServer::new(
            block_repo,
            page_repo.clone(),
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
        .with_morning_briefing(morning_briefing)
        .with_journal_repo(journal_repo)
        .with_settings_repo(settings_repo),
    );

    tracing::info!("MCP server initialized");

    Ok(mcp_server)
}

/// Shared application initialization for HTTP server
pub struct HttpServerInit {
    /// Database pool
    pub pool: DbPool,
    /// MCP server
    pub mcp_server: Arc<McpServer>,
    /// Vault configuration
    pub vault_config: VaultConfig,
}

impl HttpServerInit {
    /// Initialize HTTP server state from vault path
    pub async fn new(vault_path: PathBuf) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Initialize vault
        let vault_config = init_vault(vault_path)?;

        tracing::info!("Vault ready at {:?}", vault_config.vault_path);

        // Create database pool
        let pool = create_db_pool(&vault_config.db_path).await?;
        tracing::info!("Database pool created");

        // Initialize MCP server
        let mcp_server = init_mcp_server(pool.clone()).await?;

        Ok(Self {
            pool,
            mcp_server,
            vault_config,
        })
    }
}