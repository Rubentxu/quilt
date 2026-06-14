//! Application state for the Axum HTTP server
//!
//! Holds the database pool, MCP server, search index, and other shared resources.

use axum::extract::FromRef;
use quilt_application::services::ref_service::RefService;
use quilt_application::AppServices;

use quilt_domain::repositories::{
    BlockRepository, PageRepository, PropertyRepository, RefRepository,
    RelationRepository, SchemaRepository, SettingsRepository, TagRepository, TourStateRepository,
};
use quilt_search::SearchService;
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
/// Individual repositories are accessible via `Extension<Arc<dyn T>>` using
/// the `FromRef` implementations.
#[derive(Clone)]
pub struct AppState {
    /// Block repository for block data access
    pub block_repo: Arc<dyn BlockRepository>,
    /// Page repository for page data access
    pub page_repo: Arc<dyn PageRepository>,
    /// Ref repository for reference data access
    pub ref_repo: Arc<dyn RefRepository>,
    /// Settings repository for user preferences
    pub settings_repo: Arc<dyn SettingsRepository>,
    /// Tag repository for tag data access
    pub tag_repo: Arc<dyn TagRepository>,
    /// Relation repository for property relations
    pub relation_repo: Arc<dyn RelationRepository>,
    /// Schema repository for property schemas
    pub schema_repo: Arc<dyn SchemaRepository>,
    /// Property repository for property definitions
    pub property_repo: Arc<dyn PropertyRepository>,
    /// Tour state repository for tour dismissals
    pub tour_state_repo: Arc<dyn TourStateRepository>,
    /// Search service for full-text search
    pub search_service: Arc<SearchService>,
    /// Application services (use cases) - wrapped in Arc so AppState can be Clone
    pub services: Arc<AppServices>,
    /// Search index manager for FTS5 index maintenance
    #[allow(dead_code)]
    pub search_index: Arc<SearchIndexManager>,
    /// Broadcast sender for navigation events (WebSocket)
    pub navigation_tx: broadcast::Sender<NavigationEvent>,
    /// Last opened graph ID (for deep link navigation)
    pub last_opened_graph: Arc<RwLock<Option<String>>>,
    /// Bidirectional reference service for O(1) backlink queries
    pub ref_service: Arc<RwLock<RefService>>,
}

// ── FromRef implementations ─────────────────────────────────────────────────
//
// These allow handlers to extract Arc<dyn Repository> directly from AppState
// using `Extension<Arc<dyn T>>` without depending on concrete implementations.

impl FromRef<AppState> for Arc<dyn BlockRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.block_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PageRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.page_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn RefRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.ref_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SettingsRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.settings_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TagRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.tag_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn RelationRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.relation_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SchemaRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.schema_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PropertyRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.property_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TourStateRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.tour_state_repo.clone()
    }
}

impl FromRef<AppState> for Arc<SearchService> {
    fn from_ref(state: &AppState) -> Self {
        state.search_service.clone()
    }
}

impl AppState {
    /// Create a new AppState with all repositories wired up
    ///
    /// This is the composition root for the HTTP server. Repositories are
    /// passed as `Arc<dyn Trait>` to enable dynamic dispatch and follow
    /// the dependency inversion principle (handlers depend on interfaces).
    #[allow(dead_code)]
    pub fn new_with_repos(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        ref_repo: Arc<dyn RefRepository>,
        settings_repo: Arc<dyn SettingsRepository>,
        tag_repo: Arc<dyn TagRepository>,
        relation_repo: Arc<dyn RelationRepository>,
        schema_repo: Arc<dyn SchemaRepository>,
        property_repo: Arc<dyn PropertyRepository>,
        tour_state_repo: Arc<dyn TourStateRepository>,
        search_service: Arc<SearchService>,
        search_index: Arc<SearchIndexManager>,
        ref_service: Arc<RwLock<RefService>>,
        services: Arc<AppServices>,
    ) -> Self {
        // Create broadcast channel for navigation events
        let (navigation_tx, _) = broadcast::channel(100);

        Self {
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
            services,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
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
