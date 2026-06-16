//! Application state for the Axum HTTP server
//!
//! Holds the database pool, MCP server, search index, and other shared resources.

use axum::extract::FromRef;
use quilt_application::AppServices;
use quilt_application::services::ref_service::RefServiceTrait;
use quilt_application::use_cases::projection_resolver::ProjectionResolver;

use quilt_domain::canonicalization::PresetRegistry;
use quilt_domain::repositories::{
    BlockRepository, PageRepository, PropertyRepository, RefRepository, RelationRepository,
    SchemaRepository, SettingsRepository, TagRepository, TourStateRepository,
};
use quilt_search::SearchIndexManager;
use quilt_search::SearchService;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

/// Bundles all repository traits into a single cloneable struct.
///
/// This reduces boilerplate in `AppState` by grouping the 9 repository Arc fields
/// into one field, while still preserving ISP (each repository is a separate trait).
#[derive(Clone)]
pub struct RepositoryBundle {
    pub block: Arc<dyn BlockRepository>,
    pub page: Arc<dyn PageRepository>,
    pub ref_repo: Arc<dyn RefRepository>,
    pub settings: Arc<dyn SettingsRepository>,
    pub tag: Arc<dyn TagRepository>,
    pub relation: Arc<dyn RelationRepository>,
    pub schema: Arc<dyn SchemaRepository>,
    pub property: Arc<dyn PropertyRepository>,
    pub tour_state: Arc<dyn TourStateRepository>,
}

impl RepositoryBundle {
    #[allow(dead_code)]
    pub fn new(
        block: Arc<dyn BlockRepository>,
        page: Arc<dyn PageRepository>,
        ref_repo: Arc<dyn RefRepository>,
        settings: Arc<dyn SettingsRepository>,
        tag: Arc<dyn TagRepository>,
        relation: Arc<dyn RelationRepository>,
        schema: Arc<dyn SchemaRepository>,
        property: Arc<dyn PropertyRepository>,
        tour_state: Arc<dyn TourStateRepository>,
    ) -> Self {
        Self {
            block,
            page,
            ref_repo,
            settings,
            tag,
            relation,
            schema,
            property,
            tour_state,
        }
    }
}

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
    /// All repositories bundled into a single cloneable struct
    pub repos: RepositoryBundle,
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
    pub ref_service: Arc<dyn RefServiceTrait>,
    /// Projection resolver for block projection resolution
    pub projection_resolver: Arc<ProjectionResolver>,
    /// Preset registry for property presets
    pub preset_registry: Arc<dyn PresetRegistry>,
    /// Agent lifecycle (CG-5). `None` in unit tests that
    /// do not exercise the agent room surface; the HTTP
    /// handlers treat `None` as an Internal error.
    pub agent_lifecycle: Option<Arc<quilt_analysis::agent_room::AgentLifecycle>>,
    /// Agent registry (CG-5). The lookup table of
    /// registered `AgentExecutor` implementations.
    pub agent_registry: Option<Arc<quilt_analysis::agent_room::AgentRegistry>>,
}

// ── FromRef implementations ─────────────────────────────────────────────────
//
// These allow handlers to extract Arc<dyn Repository> directly from AppState
// using `Extension<Arc<dyn T>>` without depending on concrete implementations.

impl FromRef<AppState> for Arc<dyn BlockRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.block.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PageRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.page.clone()
    }
}

impl FromRef<AppState> for Arc<dyn RefRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.ref_repo.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SettingsRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.settings.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TagRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.tag.clone()
    }
}

impl FromRef<AppState> for Arc<dyn RelationRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.relation.clone()
    }
}

impl FromRef<AppState> for Arc<dyn SchemaRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.schema.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PropertyRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.property.clone()
    }
}

impl FromRef<AppState> for Arc<dyn TourStateRepository> {
    fn from_ref(state: &AppState) -> Self {
        state.repos.tour_state.clone()
    }
}

impl FromRef<AppState> for Arc<SearchService> {
    fn from_ref(state: &AppState) -> Self {
        state.search_service.clone()
    }
}

impl FromRef<AppState> for Arc<dyn RefServiceTrait> {
    fn from_ref(state: &AppState) -> Self {
        state.ref_service.clone()
    }
}

impl FromRef<AppState> for Arc<ProjectionResolver> {
    fn from_ref(state: &AppState) -> Self {
        state.projection_resolver.clone()
    }
}

impl FromRef<AppState> for Arc<dyn PresetRegistry> {
    fn from_ref(state: &AppState) -> Self {
        state.preset_registry.clone()
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
        repos: RepositoryBundle,
        search_service: Arc<SearchService>,
        search_index: Arc<SearchIndexManager>,
        ref_service: Arc<dyn RefServiceTrait>,
        services: Arc<AppServices>,
        projection_resolver: Arc<ProjectionResolver>,
        preset_registry: Arc<dyn PresetRegistry>,
    ) -> Self {
        Self::new_with_repos_and_agents(
            repos,
            search_service,
            search_index,
            ref_service,
            services,
            projection_resolver,
            preset_registry,
            None,
            None,
        )
    }

    /// Create a new AppState with the agent room wired in
    /// (CG-5). Pass `None` for `agent_lifecycle` and
    /// `agent_registry` to disable the agent room surface
    /// (the handlers return 500 in that case — a unit test
    /// that does not exercise the surface keeps working).
    #[allow(dead_code)]
    pub fn new_with_repos_and_agents(
        repos: RepositoryBundle,
        search_service: Arc<SearchService>,
        search_index: Arc<SearchIndexManager>,
        ref_service: Arc<dyn RefServiceTrait>,
        services: Arc<AppServices>,
        projection_resolver: Arc<ProjectionResolver>,
        preset_registry: Arc<dyn PresetRegistry>,
        agent_lifecycle: Option<Arc<quilt_analysis::agent_room::AgentLifecycle>>,
        agent_registry: Option<Arc<quilt_analysis::agent_room::AgentRegistry>>,
    ) -> Self {
        // Create broadcast channel for navigation events
        let (navigation_tx, _) = broadcast::channel(100);

        Self {
            repos,
            search_service,
            services,
            search_index,
            navigation_tx,
            last_opened_graph: Arc::new(RwLock::new(None)),
            ref_service,
            projection_resolver,
            preset_registry,
            agent_lifecycle,
            agent_registry,
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
