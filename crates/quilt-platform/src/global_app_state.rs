//! Global app state — path resolution + fail-open factory (ADR-0030, Slice C).
//!
//! This module is the platform-layer wrapper that:
//!
//! 1. Resolves the canonical `global.db` path under the user's
//!    platform data dir (XDG on Linux, Apple `~/Library/Application Support`
//!    on macOS, `%APPDATA%` on Windows via the `dirs` crate).
//! 2. Provides an `open_global_state` factory that returns an
//!    `Arc<dyn GlobalAppStateRepository>` and **fails open** on
//!    disk errors: the server never blocks startup on global state
//!    — it falls back to an in-memory `InMemoryGlobalAppStateRepository`
//!    and logs a `tracing::warn!`.
//!
//! This is the "best effort, always available" guarantee that
//! ADR-0030 §5 requires. The global state is convenient, not
//! critical; the canonical truth is the per-graph `quilt.db`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use quilt_domain::repositories::GlobalAppStateRepository;
use thiserror::Error;
use tracing::warn;

use crate::{InMemoryGlobalAppStateRepository, SqliteGlobalAppStateRepository};

/// Errors that can occur while resolving the global state path or
/// opening the underlying SQLite store. The factory function
/// `open_global_state` swallows these and falls back to in-memory.
#[derive(Debug, Error)]
pub enum GlobalStateError {
    #[error("failed to determine platform data directory")]
    NoDataDir,
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("storage error: {0}")]
    Storage(String),
}

/// Returns the canonical global state database path.
///
/// Default: `<dirs::data_dir()>/quilt/global.db`. The parent directory
/// is created on first use by `open_global_state`.
///
/// Override via the `QUILT_GLOBAL_DB_PATH` env var (useful for tests
/// and CI containers that don't have a writable `data_dir`).
pub fn global_db_path() -> PathBuf {
    if let Ok(p) = std::env::var("QUILT_GLOBAL_DB_PATH") {
        return PathBuf::from(p);
    }
    match dirs::data_dir() {
        Some(mut p) => {
            p.push("quilt");
            p.push("global.db");
            p
        }
        None => {
            // Last-ditch fallback: relative to the current working dir.
            // This is rare (no $HOME on Linux) but keeps the function total.
            PathBuf::from(".quilt-global.db")
        }
    }
}

/// Open the global state, falling back to an in-memory store on any
/// disk error. The fallback is **fail-open**: the server starts no
/// matter what, but persistence of the global state is lost until
/// the underlying issue is fixed.
pub async fn open_global_state() -> Arc<dyn GlobalAppStateRepository> {
    let path = global_db_path();
    match open_sqlite(&path) {
        Ok(repo) => {
            tracing::info!(
                target: "quilt_platform::global_app_state",
                "global state store opened at {}",
                path.display()
            );
            Arc::new(repo)
        }
        Err(e) => {
            warn!(
                target: "quilt_platform::global_app_state",
                "global state disk store unavailable at {} ({}); falling back to in-memory — state will not persist across restarts",
                path.display(),
                e
            );
            Arc::new(InMemoryGlobalAppStateRepository::default())
        }
    }
}

/// Internal: open the SQLite store, surfacing the underlying error.
fn open_sqlite(path: &Path) -> Result<SqliteGlobalAppStateRepository, GlobalStateError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| GlobalStateError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    SqliteGlobalAppStateRepository::open(path).map_err(|e| GlobalStateError::Storage(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Path resolution is global-state dependent; gate env mutations
    // with a static mutex so tests stay parallel-safe.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn global_db_path_default_uses_data_dir() {
        let _g = ENV_LOCK.lock().unwrap();
        // Clear override so we get the default branch.
        // SAFETY: env mutations in tests are guarded by ENV_LOCK.
        unsafe { std::env::remove_var("QUILT_GLOBAL_DB_PATH") };
        let p = global_db_path();
        // On Linux the path ends with /quilt/global.db.
        assert!(p.ends_with("quilt/global.db") || p.ends_with("quilt\\global.db"));
    }

    #[test]
    fn global_db_path_honors_override() {
        let _g = ENV_LOCK.lock().unwrap();
        let override_path = "/tmp/quilt-test-global.db";
        // SAFETY: see above.
        unsafe { std::env::set_var("QUILT_GLOBAL_DB_PATH", override_path) };
        let p = global_db_path();
        assert_eq!(p, PathBuf::from(override_path));
        unsafe { std::env::remove_var("QUILT_GLOBAL_DB_PATH") };
    }

    #[tokio::test]
    async fn open_global_state_succeeds_on_writable_path() {
        let _g = ENV_LOCK.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("global.db");
        // SAFETY: see above.
        unsafe { std::env::set_var("QUILT_GLOBAL_DB_PATH", &p) };
        let repo = open_global_state().await;
        // Round-trip a write to confirm the store is real.
        repo.set_last_opened_graph(Some(std::path::Path::new("/x")))
            .await
            .unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.last_opened_graph, Some(std::path::PathBuf::from("/x")));
        unsafe { std::env::remove_var("QUILT_GLOBAL_DB_PATH") };
    }

    #[tokio::test]
    async fn open_global_state_fails_open_on_unwritable_path() {
        let _g = ENV_LOCK.lock().unwrap();
        // Use a path that points to a file as a directory, which will
        // fail mkdir. The factory should swallow it and return an
        // in-memory store.
        let tmp = tempfile::tempdir().unwrap();
        let blocker = tmp.path().join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let bad = blocker.join("quilt").join("global.db");
        // SAFETY: see above.
        unsafe { std::env::set_var("QUILT_GLOBAL_DB_PATH", &bad) };
        let repo = open_global_state().await;
        // Should not panic; the in-memory fallback should accept writes.
        repo.set_last_opened_graph(Some(std::path::Path::new("/y")))
            .await
            .unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.last_opened_graph, Some(std::path::PathBuf::from("/y")));
        unsafe { std::env::remove_var("QUILT_GLOBAL_DB_PATH") };
    }
}
