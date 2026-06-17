//! SQLite-backed `GlobalAppStateRepository`.
//!
//! This is the implementation behind the cross-graph state. It lives
//! in `~/.local/share/quilt/global.db` (or
//! `XDG_DATA_HOME/quilt/global.db`), a dedicated SQLite file that
//! is **not** the per-graph `quilt.db`.
//!
//! We use a single dedicated `Mutex<Connection>` instead of a pool
//! because the workload is a tiny number of short writes per session
//! (last opened graph, recents push, sidebar visibility) and a single
//! connection keeps the WAL mode semantics simple.
//!
//! Bootstrap is idempotent: the table is created on first open via
//! the embedded `0001_init.sql` migration content.

use async_trait::async_trait;
use rusqlite::{params, Connection, OpenFlags};
use serde_json::{json, Value as JsonValue};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use quilt_domain::entities::global_app_state::{GlobalAppState, RECENTS_CAP};
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::GlobalAppStateRepository;

const SCHEMA_SQL: &str = include_str!("../../../migrations/global/0001_init.sql");

/// SQLite-backed `GlobalAppStateRepository`.
///
/// Use [`SqliteGlobalAppStateRepository::open`] to create or open a
/// store at the given path; the parent directory is created if missing
/// and the schema is applied.
pub struct SqliteGlobalAppStateRepository {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl SqliteGlobalAppStateRepository {
    /// Open (or create) the global state store at `path`.
    pub fn open(path: &Path) -> Result<Self, DomainError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DomainError::Storage(format!(
                    "failed to create global state dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(|e| {
            DomainError::Storage(format!(
                "failed to open global state at {}: {e}",
                path.display()
            ))
        })?;
        // WAL for read/write concurrency; small DB, this is cheap.
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")
            .map_err(|e| DomainError::Storage(format!("pragma failed: {e}")))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| DomainError::Storage(format!("schema bootstrap failed: {e}")))?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    /// Path this store was opened from (for diagnostics / logs).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn map_err(e: rusqlite::Error) -> DomainError {
    DomainError::Storage(format!("global state: {e}"))
}

fn load_inner(conn: &Connection) -> Result<GlobalAppState, DomainError> {
    let mut stmt = conn
        .prepare(
            "SELECT last_opened_graph, recent_graphs_json, right_sidebar_visible \
             FROM global_app_state WHERE id = 1",
        )
        .map_err(map_err)?;
    let mut rows = stmt.query([]).map_err(map_err)?;
    let Some(row) = rows.next().map_err(map_err)? else {
        return Ok(GlobalAppState::default());
    };
    let last_opened: Option<String> = row.get(0).map_err(map_err)?;
    let recent_json: String = row.get(1).map_err(map_err)?;
    let sidebar: Option<i64> = row.get(2).map_err(map_err)?;

    // last_opened_graph: Option<PathBuf>
    let last_opened_graph = last_opened.map(PathBuf::from);

    // recent_graphs_json: Vec<PathBuf>
    let parsed: JsonValue = serde_json::from_str(&recent_json).map_err(|e| {
        DomainError::Storage(format!("invalid recent_graphs_json: {e}"))
    })?;
    let mut recent_graphs: Vec<PathBuf> = match parsed {
        JsonValue::Array(arr) => arr
            .into_iter()
            .filter_map(|v| v.as_str().map(PathBuf::from))
            .collect(),
        _ => {
            return Err(DomainError::Storage(
                "recent_graphs_json is not an array".to_string(),
            ))
        }
    };
    // Defensive bound: trust the schema but never let the in-memory
    // value exceed the cap.
    recent_graphs.truncate(RECENTS_CAP);

    // right_sidebar_visible: Option<bool>
    let right_sidebar_visible = match sidebar {
        Some(0) => Some(false),
        Some(_) => Some(true),
        None => None,
    };

    Ok(GlobalAppState {
        last_opened_graph,
        recent_graphs,
        right_sidebar_visible,
    })
}

#[async_trait]
impl GlobalAppStateRepository for SqliteGlobalAppStateRepository {
    async fn load(&self) -> Result<GlobalAppState, DomainError> {
        let conn = self.conn.lock().expect("global state mutex poisoned");
        load_inner(&conn)
    }

    async fn set_last_opened_graph(
        &self,
        path: Option<&Path>,
    ) -> Result<(), DomainError> {
        let conn = self.conn.lock().expect("global state mutex poisoned");
        let value: Option<String> = path.map(|p| p.display().to_string());
        conn.execute(
            "UPDATE global_app_state SET last_opened_graph = ? WHERE id = 1",
            params![value],
        )
        .map_err(map_err)?;
        Ok(())
    }

    async fn push_recent(&self, path: &Path) -> Result<(), DomainError> {
        let mut conn = self.conn.lock().expect("global state mutex poisoned");
        let tx = conn.transaction().map_err(map_err)?;
        let current_json: String = {
            let mut stmt = tx
                .prepare("SELECT recent_graphs_json FROM global_app_state WHERE id = 1")
                .map_err(map_err)?;
            let mut rows = stmt.query([]).map_err(map_err)?;
            let Some(row) = rows.next().map_err(map_err)? else {
                return Err(DomainError::Storage(
                    "global_app_state row missing (init bug?)".to_string(),
                ));
            };
            row.get(0).map_err(map_err)?
        };
        let mut current: Vec<PathBuf> = match serde_json::from_str::<JsonValue>(&current_json)
            .map_err(|e| DomainError::Storage(format!("invalid recent_graphs_json: {e}")))?
        {
            JsonValue::Array(arr) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(PathBuf::from))
                .collect(),
            _ => {
                return Err(DomainError::Storage(
                    "recent_graphs_json is not an array".to_string(),
                ))
            }
        };
        // Dedupe (case-sensitive) and push to head.
        current.retain(|p| p.as_path() != path);
        current.insert(0, path.to_path_buf());
        current.truncate(RECENTS_CAP);
        let new_json = serde_json::to_string(&json!(current
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()))
        .map_err(|e| DomainError::Storage(format!("serialize recents: {e}")))?;
        tx.execute(
            "UPDATE global_app_state SET recent_graphs_json = ? WHERE id = 1",
            params![new_json],
        )
        .map_err(map_err)?;
        tx.commit().map_err(map_err)?;
        Ok(())
    }

    async fn set_right_sidebar_visible(
        &self,
        visible: Option<bool>,
    ) -> Result<(), DomainError> {
        let conn = self.conn.lock().expect("global state mutex poisoned");
        let value: Option<i64> = visible.map(|b| if b { 1 } else { 0 });
        conn.execute(
            "UPDATE global_app_state SET right_sidebar_visible = ? WHERE id = 1",
            params![value],
        )
        .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_tmp() -> (tempfile::TempDir, SqliteGlobalAppStateRepository) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("global.db");
        let repo = SqliteGlobalAppStateRepository::open(&path).expect("open");
        (tmp, repo)
    }

    #[tokio::test]
    async fn empty_state_on_fresh_open() {
        let (_tmp, repo) = open_tmp();
        let s = repo.load().await.unwrap();
        assert_eq!(s, GlobalAppState::default());
    }

    #[tokio::test]
    async fn set_last_opened_persists() {
        let (_tmp, repo) = open_tmp();
        repo.set_last_opened_graph(Some(std::path::Path::new("/var/data/g1")))
            .await
            .unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.last_opened_graph, Some(std::path::PathBuf::from("/var/data/g1")));
    }

    #[tokio::test]
    async fn push_recent_dedupes_and_caps() {
        let (_tmp, repo) = open_tmp();
        for i in 0..(RECENTS_CAP + 5) {
            repo.push_recent(std::path::Path::new(&format!("/g{i}")))
                .await
                .unwrap();
        }
        let s = repo.load().await.unwrap();
        assert_eq!(s.recent_graphs.len(), RECENTS_CAP);
        // Most-recent first: /g14 (RECENTS_CAP+5-1)
        assert_eq!(
            s.recent_graphs[0],
            std::path::PathBuf::from(format!("/g{}", RECENTS_CAP + 4))
        );

        // Dedupe: pushing an existing path moves it to the head.
        repo.push_recent(std::path::Path::new("/g5")).await.unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.recent_graphs[0], std::path::PathBuf::from("/g5"));
        // Total count still capped.
        assert_eq!(s.recent_graphs.len(), RECENTS_CAP);
    }

    #[tokio::test]
    async fn sidebar_visibility_round_trips() {
        let (_tmp, repo) = open_tmp();
        repo.set_right_sidebar_visible(Some(false)).await.unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.right_sidebar_visible, Some(false));
        repo.set_right_sidebar_visible(Some(true)).await.unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.right_sidebar_visible, Some(true));
        repo.set_right_sidebar_visible(None).await.unwrap();
        let s = repo.load().await.unwrap();
        assert_eq!(s.right_sidebar_visible, None);
    }

    #[tokio::test]
    async fn clear_last_opened() {
        let (_tmp, repo) = open_tmp();
        repo.set_last_opened_graph(Some(std::path::Path::new("/a")))
            .await
            .unwrap();
        repo.set_last_opened_graph(None).await.unwrap();
        let s = repo.load().await.unwrap();
        assert!(s.last_opened_graph.is_none());
    }

    #[tokio::test]
    async fn reopen_persists_state() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("global.db");
        {
            let repo = SqliteGlobalAppStateRepository::open(&path).unwrap();
            repo.set_last_opened_graph(Some(std::path::Path::new("/x")))
                .await
                .unwrap();
            repo.push_recent(std::path::Path::new("/y")).await.unwrap();
            repo.set_right_sidebar_visible(Some(true)).await.unwrap();
        }
        // Re-open: the state should be exactly as we left it.
        let repo2 = SqliteGlobalAppStateRepository::open(&path).unwrap();
        let s = repo2.load().await.unwrap();
        assert_eq!(s.last_opened_graph, Some(std::path::PathBuf::from("/x")));
        assert_eq!(s.recent_graphs, vec![std::path::PathBuf::from("/y")]);
        assert_eq!(s.right_sidebar_visible, Some(true));
    }
}
