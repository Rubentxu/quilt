//! Application state for the Axum HTTP server
//!
//! Holds the database pool, MCP server, search index, and other shared resources.

use quilt_application::services::ref_service::RefService;
#[cfg(feature = "cognitive")]
use quilt_cognitive::AIClient;
#[cfg(feature = "cognitive")]
use quilt_cognitive::{
    ai_client::MockAIClient, ArgumentCartographer, CognitiveMirror, MorningBriefing,
    SerendipityEngine,
};
use quilt_infrastructure::database::sqlite::connection::DbPool;
use quilt_search::SearchIndexManager;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

/// Navigation event sent to WebSocket clients
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEvent {
    pub event_type: String,
    pub target: NavigationTarget,
}

/// Target of a navigation event
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationTarget {
    #[serde(rename = "type")]
    pub target_type: String,
    pub graph_id: Option<String>,
    pub page_name: String,
    pub block_uuid: Option<String>,
}

impl NavigationEvent {
    /// Create a page navigation event
    pub fn page(graph_id: Option<String>, page_name: String) -> Self {
        Self {
            event_type: "navigate-to".to_string(),
            target: NavigationTarget {
                target_type: "Page".to_string(),
                graph_id,
                page_name,
                block_uuid: None,
            },
        }
    }

    /// Create a block navigation event
    pub fn block(graph_id: Option<String>, page_name: String, block_uuid: String) -> Self {
        Self {
            event_type: "navigate-to".to_string(),
            target: NavigationTarget {
                target_type: "Block".to_string(),
                graph_id,
                page_name,
                block_uuid: Some(block_uuid),
            },
        }
    }
}

/// Application state shared across all HTTP handlers
///
/// This is passed to handlers via `Extension<AppState>` in Axum.
#[derive(Clone)]
pub struct AppState {
    /// SQLite database connection pool
    pub pool: DbPool,
    /// Settings repository for user preferences
    pub settings_repo:
        quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository,
    /// Search index manager for FTS5 index maintenance
    #[allow(dead_code)]
    pub search_index: Arc<SearchIndexManager>,
    /// Broadcast sender for navigation events (WebSocket)
    pub navigation_tx: broadcast::Sender<NavigationEvent>,
    /// Last opened graph ID (for deep link navigation)
    pub last_opened_graph: Arc<RwLock<Option<String>>>,
    /// Bidirectional reference service for O(1) backlink queries
    pub ref_service: Arc<RwLock<RefService>>,
    /// AI client for cognitive engines — can be reconfigured at runtime
    #[cfg(feature = "cognitive")]
    pub ai_client: Arc<RwLock<Arc<dyn AIClient>>>,
    /// CognitiveMirror engine for analyzing block reference graphs
    #[cfg(feature = "cognitive")]
    pub cognitive_mirror: Option<Arc<CognitiveMirror>>,
    /// SerendipityEngine for discovering unexpected connections
    #[cfg(feature = "cognitive")]
    pub serendipity_engine: Option<Arc<SerendipityEngine>>,
    /// MorningBriefing for daily cognitive summaries
    #[cfg(feature = "cognitive")]
    pub morning_briefing: Option<Arc<MorningBriefing>>,
    /// ArgumentCartographer for mapping argument structures
    #[cfg(feature = "cognitive")]
    pub argument_cartographer: Option<Arc<ArgumentCartographer>>,
}

impl AppState {
    /// Create a new AppState
    ///
    /// Initializes database pool, search index, reference service.
    /// When the `cognitive` feature is enabled, also creates a default mock AI client.
    #[allow(dead_code)]
    pub fn new(
        pool: DbPool,
        search_index: Arc<SearchIndexManager>,
        ref_service: Arc<RwLock<RefService>>,
    ) -> Self {
        // Create broadcast channel for navigation events
        let (navigation_tx, _) = broadcast::channel(100);

        #[cfg(feature = "cognitive")]
        let ai_client: Arc<dyn AIClient> = Arc::new(MockAIClient::new());

        let settings_repo =
            quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository::new(
                pool.clone(),
            );

        Self {
            pool,
            settings_repo,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
            #[cfg(feature = "cognitive")]
            ai_client: Arc::new(RwLock::new(ai_client)),
            #[cfg(feature = "cognitive")]
            cognitive_mirror: None,
            #[cfg(feature = "cognitive")]
            serendipity_engine: None,
            #[cfg(feature = "cognitive")]
            morning_briefing: None,
            #[cfg(feature = "cognitive")]
            argument_cartographer: None,
        }
    }

    /// Create a new AppState with an externally-provided AI client
    ///
    /// When the `cognitive` feature is enabled, allows injecting a specific
    /// AI client implementation (e.g., from main.rs).
    #[cfg(feature = "cognitive")]
    pub fn with_ai_client(
        pool: DbPool,
        search_index: Arc<SearchIndexManager>,
        ai_client: Arc<dyn AIClient>,
        ref_service: Arc<RwLock<RefService>>,
    ) -> Self {
        let (navigation_tx, _) = broadcast::channel(100);
        let settings_repo =
            quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository::new(
                pool.clone(),
            );
        Self {
            pool,
            settings_repo,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
            ai_client: Arc::new(RwLock::new(ai_client)),
            cognitive_mirror: None,
            serendipity_engine: None,
            morning_briefing: None,
            argument_cartographer: None,
        }
    }

    /// Create a new AppState with cognitive engines
    ///
    /// Full constructor that includes all cognitive engines.
    #[cfg(feature = "cognitive")]
    pub fn with_cognitive(
        pool: DbPool,
        search_index: Arc<SearchIndexManager>,
        ai_client: Arc<dyn AIClient>,
        ref_service: Arc<RwLock<RefService>>,
        cognitive_mirror: Arc<CognitiveMirror>,
        serendipity_engine: Arc<SerendipityEngine>,
        morning_briefing: Arc<MorningBriefing>,
        argument_cartographer: Arc<ArgumentCartographer>,
    ) -> Self {
        let (navigation_tx, _) = broadcast::channel(100);
        let settings_repo =
            quilt_infrastructure::database::sqlite::repositories::SqliteSettingsRepository::new(
                pool.clone(),
            );
        Self {
            pool,
            settings_repo,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
            ai_client: Arc::new(RwLock::new(ai_client)),
            cognitive_mirror: Some(cognitive_mirror),
            serendipity_engine: Some(serendipity_engine),
            morning_briefing: Some(morning_briefing),
            argument_cartographer: Some(argument_cartographer),
        }
    }

    /// Broadcast a navigation event to all WebSocket subscribers
    pub fn broadcast_navigation(&self, event: NavigationEvent) -> anyhow::Result<()> {
        self.navigation_tx
            .send(event)
            .map_err(|_| anyhow::anyhow!("No active WebSocket subscribers"))?;
        Ok(())
    }
}
