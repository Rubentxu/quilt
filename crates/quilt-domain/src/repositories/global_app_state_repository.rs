//! Repository port for the cross-graph app state.
//!
//! See [`super::super::entities::global_app_state::GlobalAppState`] for
//! the entity. The implementation lives in `quilt-infrastructure`
//! (SQLite-backed) and `quilt-test-helpers` (in-memory).

use std::path::Path;

use async_trait::async_trait;

use crate::entities::global_app_state::GlobalAppState;
use crate::errors::DomainError;

/// Persistence port for the cross-graph app state.
///
/// Implementations must be safe to call concurrently. Errors propagate
/// as [`DomainError::Storage`] or [`DomainError::Database`].
#[async_trait]
pub trait GlobalAppStateRepository: Send + Sync {
    /// Load the current state.
    ///
    /// Returns [`GlobalAppState::default`] if no row is present
    /// (fresh install). Returns the stored value otherwise.
    async fn load(&self) -> Result<GlobalAppState, DomainError>;

    /// Persist `path` as the most recently opened graph.
    ///
    /// `None` clears the field. Does not touch `recent_graphs`.
    async fn set_last_opened_graph(
        &self,
        path: Option<&Path>,
    ) -> Result<(), DomainError>;

    /// Push `path` to the head of the recents list (deduped,
    /// most-recent-first, capped).
    async fn push_recent(&self, path: &Path) -> Result<(), DomainError>;

    /// Persist the right sidebar visibility preference.
    ///
    /// `None` clears the preference.
    async fn set_right_sidebar_visible(
        &self,
        visible: Option<bool>,
    ) -> Result<(), DomainError>;
}
