//! In-memory implementation of [`GlobalAppStateRepository`] for tests
//! and the platform-layer fail-open fallback.
//!
//! Stores state behind a `parking_lot::RwLock`. `Default` produces an
//! empty store; the `with_*` builders pre-seed it.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use quilt_domain::entities::global_app_state::GlobalAppState;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::GlobalAppStateRepository;

/// In-memory `GlobalAppStateRepository`.
///
/// Behaviorally equivalent to the SQLite impl minus persistence. Used
/// in tests and as the fail-open fallback when the platform cannot
/// open the SQLite store.
#[derive(Debug, Default)]
pub struct InMemoryGlobalAppStateRepository {
    state: RwLock<GlobalAppState>,
}

impl InMemoryGlobalAppStateRepository {
    /// Construct a new empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-seed the store with the given state (used in tests).
    pub fn with_state(self: Arc<Self>, state: GlobalAppState) -> Arc<Self> {
        *self.state.write() = state;
        self
    }

    /// Snapshot the current state.
    pub fn snapshot(&self) -> GlobalAppState {
        self.state.read().clone()
    }
}

#[async_trait]
impl GlobalAppStateRepository for InMemoryGlobalAppStateRepository {
    async fn load(&self) -> Result<GlobalAppState, DomainError> {
        Ok(self.state.read().clone())
    }

    async fn set_last_opened_graph(
        &self,
        path: Option<&std::path::Path>,
    ) -> Result<(), DomainError> {
        let mut s = self.state.write();
        s.last_opened_graph = path.map(PathBuf::from);
        Ok(())
    }

    async fn push_recent(&self, path: &std::path::Path) -> Result<(), DomainError> {
        let mut s = self.state.write();
        s.push_recent(path.to_path_buf());
        Ok(())
    }

    async fn set_right_sidebar_visible(&self, visible: Option<bool>) -> Result<(), DomainError> {
        let mut s = self.state.write();
        s.right_sidebar_visible = visible;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_state() {
        let repo = InMemoryGlobalAppStateRepository::new();
        repo.set_last_opened_graph(Some(std::path::Path::new("/a")))
            .await
            .unwrap();
        repo.push_recent(std::path::Path::new("/b")).await.unwrap();
        repo.push_recent(std::path::Path::new("/a")).await.unwrap();
        repo.set_right_sidebar_visible(Some(true)).await.unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.last_opened_graph, Some(PathBuf::from("/a")));
        // /a was pushed last → head; /b second.
        assert_eq!(
            s.recent_graphs,
            vec![PathBuf::from("/a"), PathBuf::from("/b")]
        );
        assert_eq!(s.right_sidebar_visible, Some(true));
    }
}
