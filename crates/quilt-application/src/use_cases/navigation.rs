//! Navigation use cases.
//!
//! Thin glue over the navigation transport concerns:
//! - Resolving a page (creating a placeholder DTO when missing).
//! - Updating the in-memory "last opened graph" pointer.
//! - Broadcasting a [`NavigationEvent`] to WebSocket subscribers.
//!
//! The HTTP handler is the only place that wires the broadcast sender
//! and the `last_opened_graph` `RwLock`; the use case accepts a
//! [`NavigationPort`] trait that abstracts over those two pieces so
//! the algorithm is testable without a running `AppState`.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::entities::Page;
use quilt_domain::repositories::PageRepository;
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Navigation target — what the caller is navigating to.
#[derive(Debug, Clone)]
pub enum NavTarget {
    /// Navigate to a page by name. `graph_id` is optional and updates
    /// the last-opened-graph pointer when present.
    Page {
        /// The page name to navigate to.
        page_name: String,
        /// Optional graph id to record as the most recently opened.
        graph_id: Option<String>,
    },
    /// Navigate to a specific block within a page.
    Block {
        /// The page name the block lives on.
        page_name: String,
        /// UUID of the block to focus.
        block_uuid: Uuid,
        /// Optional graph id to record as the most recently opened.
        graph_id: Option<String>,
    },
}

/// Outcome of a navigation. The HTTP layer translates this into JSON
/// (and a `Page` placeholder for missing pages).
#[derive(Debug, Clone)]
pub struct NavigationOutcome {
    /// The page that was navigated to (or `None` when only the block
    /// coordinates were supplied).
    pub page: Option<Page>,
}

/// The transport-specific port the use case depends on.
///
/// Implemented by the HTTP layer to fan-out the navigation event to
/// WebSocket subscribers and update the `last_opened_graph` pointer.
#[async_trait]
pub trait NavigationPort: Send + Sync {
    /// Record the graph id as the most recently opened (no-op when
    /// `None`).
    async fn record_last_opened_graph(&self, graph_id: Option<String>);
    /// Broadcast a navigation event to all subscribers. Failures
    /// (e.g. no subscribers) are logged but not surfaced as errors —
    /// navigation is best-effort.
    async fn broadcast(&self, target: NavTarget);
}

/// Use cases for navigation.
#[async_trait]
pub trait NavigationUseCases: Send + Sync {
    /// Navigate to a page or block. Resolves the target page when
    /// relevant, broadcasts the navigation event, and returns the
    /// resolved page (or `None` for block-only navigation).
    async fn navigate(&self, target: NavTarget) -> Result<NavigationOutcome, ApplicationError>;
}

/// Implementation of [`NavigationUseCases`] for any
/// [`PageRepository`] + [`NavigationPort`].
pub struct NavigationUseCasesImpl<PR: PageRepository, NP: NavigationPort> {
    page_repo: Arc<PR>,
    port: Arc<NP>,
}

impl<PR: PageRepository, NP: NavigationPort> NavigationUseCasesImpl<PR, NP> {
    /// Create a new use-case instance.
    pub fn new(page_repo: Arc<PR>, port: Arc<NP>) -> Self {
        Self { page_repo, port }
    }
}

#[async_trait]
impl<PR: PageRepository + 'static, NP: NavigationPort + 'static> NavigationUseCases
    for NavigationUseCasesImpl<PR, NP>
{
    #[instrument(skip(self))]
    async fn navigate(&self, target: NavTarget) -> Result<NavigationOutcome, ApplicationError> {
        match &target {
            NavTarget::Page { graph_id, .. } => {
                self.port.record_last_opened_graph(graph_id.clone()).await;
            }
            NavTarget::Block { graph_id, .. } => {
                self.port.record_last_opened_graph(graph_id.clone()).await;
            }
        }

        let page = match &target {
            NavTarget::Page { page_name, .. } => self
                .page_repo
                .get_by_name(page_name)
                .await
                .map_err(ApplicationError::Domain)?,
            NavTarget::Block { page_name, .. } => self
                .page_repo
                .get_by_name(page_name)
                .await
                .map_err(ApplicationError::Domain)?,
        };

        self.port.broadcast(target).await;

        Ok(NavigationOutcome { page })
    }
}
