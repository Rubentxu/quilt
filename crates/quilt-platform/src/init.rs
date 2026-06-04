//! Shared initialization for Quilt application state
//!
//! This module provides common initialization logic that can be shared between
//! different entry points (Tauri desktop app, HTTP server, CLI).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use quilt_application::use_cases::{BlockUseCases, PageUseCases, SearchUseCases};
use quilt_domain::repositories::SettingsRepository;
use quilt_domain::services::TimezoneService;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations, DbPool};
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqliteDeepLinkRepository, SqliteJournalRepository, SqlitePageRepository,
    SqliteSettingsRepository, SqliteTagRepository,
};
use quilt_mcp::handlers::{
    BlockToolHandler, GraphHandler, PageHandler, QueryHandler, ResourceHandler, RetrievalHandler,
    TemplateHandler, TemporalHandler,
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
pub fn ensure_vault_exists(vault_path: &Path) -> Result<PathBuf, VaultError> {
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

/// Initialize MCP server with standard handlers
///
/// This creates the MCP server setup including:
/// - Block, page, graph, query handlers
/// - Search and retrieval handlers
/// - Template and temporal handlers
pub async fn init_mcp_server(
    pool: DbPool,
) -> Result<Arc<McpServer>, Box<dyn std::error::Error + Send + Sync>> {
    // Create repositories
    let block_repo: Arc<dyn quilt_domain::repositories::BlockRepository> =
        Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo: Arc<dyn quilt_domain::repositories::PageRepository> =
        Arc::new(SqlitePageRepository::new(pool.clone()));
    let tag_repo: Arc<dyn quilt_domain::repositories::TagRepository> =
        Arc::new(SqliteTagRepository::new(pool.clone()));
    let deep_link_repo: Arc<dyn quilt_domain::repositories::DeepLinkRepository> =
        Arc::new(SqliteDeepLinkRepository::new(pool.clone()));
    let search_service = Arc::new(SearchService::new(pool.clone()));
    let settings_repo: Arc<dyn quilt_domain::repositories::SettingsRepository> =
        Arc::new(SqliteSettingsRepository::new(pool.clone()));
    let journal_repo: Arc<dyn quilt_domain::repositories::JournalRepository> =
        Arc::new(SqliteJournalRepository::new(pool.clone()));

    // Create use cases
    let block_use_cases: Arc<dyn BlockUseCases> = block_repo.clone();
    let page_use_cases: Arc<dyn PageUseCases> = page_repo.clone();
    let search_use_cases: Arc<dyn SearchUseCases> = search_service.clone();

    // Create handlers
    let block_handler = BlockToolHandler::new(block_use_cases);
    let page_handler = PageHandler::new(page_use_cases);
    let graph_handler = GraphHandler::new(block_repo.clone());
    let query_handler = QueryHandler::new(search_use_cases.clone());
    let retrieval_handler = RetrievalHandler::new(search_use_cases);
    let resource_handler = ResourceHandler::new(block_repo.clone(), page_repo.clone());
    let template_handler = TemplateHandler::new();
    let temporal_handler = TemporalHandler::new(journal_repo.clone());

    // Create MCP server
    let mcp_server = Arc::new(McpServer::new(
        vec![
            Box::new(block_handler),
            Box::new(page_handler),
            Box::new(graph_handler),
            Box::new(query_handler),
            Box::new(retrieval_handler),
            Box::new(template_handler),
            Box::new(temporal_handler),
        ],
        vec![Box::new(resource_handler)],
    ));

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
    pub async fn new(
        vault_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
