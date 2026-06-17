//! Explicit Graph Space validation (ADR-0030, Slice B)
//!
//! This module is the **pure, side-effect-free** resolver that decides
//! whether a given path is a valid Graph Space layout. It is invoked
//! by the bootstrap chain (`init_graph`) to fail explicitly when:
//!
//! - the directory does not exist
//! - `.quilt/` is missing
//! - `quilt.db` is missing
//! - the database file cannot be opened
//! - the schema does not look like a Quilt graph (minimum marker:
//!   presence of `user_settings` table)
//!
//! The validator **does not** modify the filesystem, **does not** run
//! migrations, and **does not** create the layout. Creation is the
//! caller's responsibility (via `init_graph`).
//!
//! ## Invariant
//!
//! Per ADR-0030 §6, a Graph that fails validation must **not** be
//! auto-repaired or silently recreated. `validate_graph_layout` is the
//! single source of truth for "is this a usable graph?".
//!
//! ## See also
//!
//! - [`quilt_platform::init::init_graph`] — the creating bootstrap that
//!   calls into this module and converts errors to `GraphError`.
//! - [`quilt_server::error::AppError`] — HTTP error mapping.

use std::path::{Path, PathBuf};

use rusqlite::OpenFlags;

/// A successful validation: a fully-resolved Graph Space layout.
///
/// The `db_path` is the canonical database
/// (`<graph-root>/.quilt/quilt.db`); the `quilt_dir` is the canonical
/// `<graph-root>/.quilt/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphLayout {
    /// The graph root directory provided by the caller.
    pub graph_path: PathBuf,
    /// The canonical `.quilt/` directory.
    pub quilt_dir: PathBuf,
    /// The canonical `quilt.db` path.
    pub db_path: PathBuf,
}

impl GraphLayout {
    /// The URL form for sqlx (`mode=rwc`).
    pub fn database_url(&self) -> String {
        format!("sqlite:{}?mode=rwc", self.db_path.display())
    }
}

/// Typed errors for graph layout validation.
///
/// Each variant carries the path that triggered it so handlers can
/// surface a useful, structured response (per `quilt-server` B.4).
#[derive(Debug, thiserror::Error)]
pub enum GraphValidationError {
    /// The graph root directory does not exist.
    #[error("graph directory does not exist: {0}")]
    DirectoryMissing(PathBuf),

    /// The `<graph-root>/.quilt/` directory does not exist.
    #[error(".quilt directory does not exist at {0}")]
    QuiltDirMissing(PathBuf),

    /// The `<graph-root>/.quilt/quilt.db` file does not exist.
    #[error("quilt.db does not exist at {0}")]
    DatabaseMissing(PathBuf),

    /// The database file exists but cannot be opened (corrupt, locked,
    /// permission, etc.).
    #[error("quilt.db exists at {path} but cannot be opened: {reason}")]
    DatabaseUnopenable {
        /// The path that was probed.
        path: PathBuf,
        /// Human-readable reason.
        reason: String,
    },

    /// The database opens but does not look like a Quilt graph
    /// (the `user_settings` minimum marker is absent).
    #[error("quilt.db at {path} has no Quilt schema: {detail}")]
    SchemaIncompatible {
        /// The path that was probed.
        path: PathBuf,
        /// Human-readable detail.
        detail: String,
    },
}

impl GraphValidationError {
    /// Stable identifier for the variant, used as the `validationError`
    /// field in HTTP 422 responses (per `quilt-server` B.4).
    pub fn code(&self) -> &'static str {
        match self {
            GraphValidationError::DirectoryMissing(_) => "DirectoryMissing",
            GraphValidationError::QuiltDirMissing(_) => "QuiltDirMissing",
            GraphValidationError::DatabaseMissing(_) => "DatabaseMissing",
            GraphValidationError::DatabaseUnopenable { .. } => "DatabaseUnopenable",
            GraphValidationError::SchemaIncompatible { .. } => "SchemaIncompatible",
        }
    }

    /// The path that triggered the error, if any.
    pub fn path(&self) -> Option<&Path> {
        match self {
            GraphValidationError::DirectoryMissing(p)
            | GraphValidationError::QuiltDirMissing(p)
            | GraphValidationError::DatabaseMissing(p)
            | GraphValidationError::DatabaseUnopenable { path: p, .. }
            | GraphValidationError::SchemaIncompatible { path: p, .. } => Some(p.as_path()),
        }
    }
}

/// The minimum schema marker: a `user_settings` table must exist.
///
/// ADR-0030 §6 forbids running migrations during validation; this
/// marker is a *read-only* probe. If the table is missing the
/// graph is considered schema-incompatible.
const QUOTED_MARKER_TABLE: &str = "\"user_settings\"";

/// Validate that `path` looks like a usable Graph Space layout.
///
/// **Pure**: no side effects, no migrations, no file creation. The
/// function only inspects the filesystem and opens a read-only
/// `rusqlite` connection to probe the schema marker.
pub fn validate_graph_layout(path: &Path) -> Result<GraphLayout, GraphValidationError> {
    if !path.exists() {
        return Err(GraphValidationError::DirectoryMissing(path.to_path_buf()));
    }
    if !path.is_dir() {
        return Err(GraphValidationError::DirectoryMissing(path.to_path_buf()));
    }

    let quilt_dir = path.join(".quilt");
    if !quilt_dir.exists() {
        return Err(GraphValidationError::QuiltDirMissing(quilt_dir));
    }
    if !quilt_dir.is_dir() {
        return Err(GraphValidationError::QuiltDirMissing(quilt_dir));
    }

    let db_path = quilt_dir.join("quilt.db");
    if !db_path.exists() {
        return Err(GraphValidationError::DatabaseMissing(db_path));
    }

    // Open the file read-only. We use rusqlite directly (not sqlx)
    // because we want a single, fast, side-effect-free schema probe
    // that does NOT participate in the main pool.
    let conn = rusqlite::Connection::open_with_flags(
        &db_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| GraphValidationError::DatabaseUnopenable {
        path: db_path.clone(),
        reason: e.to_string(),
    })?;

    // Probe: a Quilt graph always has a `user_settings` table.
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
            rusqlite::params!["user_settings"],
            |row| row.get(0),
        )
        .map_err(|e| GraphValidationError::DatabaseUnopenable {
            path: db_path.clone(),
            reason: format!("schema probe failed: {e}"),
        })?;
    if exists == 0 {
        return Err(GraphValidationError::SchemaIncompatible {
            path: db_path,
            detail: format!("missing minimum marker table {QUOTED_MARKER_TABLE}"),
        });
    }

    Ok(GraphLayout {
        graph_path: path.to_path_buf(),
        quilt_dir,
        db_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::fs;
    use tempfile::tempdir;

    fn seed_quilt_db(db_path: &Path) {
        let conn = Connection::open(db_path).unwrap();
        // Minimum marker table (real migrations create many more).
        conn.execute_batch("CREATE TABLE user_settings (id INTEGER PRIMARY KEY, timezone TEXT);")
            .unwrap();
    }

    #[test]
    fn missing_directory_returns_directory_missing() {
        let tmp = tempdir().unwrap();
        let p = tmp.path().join("does-not-exist");
        match validate_graph_layout(&p) {
            Err(GraphValidationError::DirectoryMissing(path)) => assert_eq!(path, p),
            other => panic!("expected DirectoryMissing, got {other:?}"),
        }
    }

    #[test]
    fn missing_quilt_dir_returns_quilt_dir_missing() {
        let tmp = tempdir().unwrap();
        match validate_graph_layout(tmp.path()) {
            Err(GraphValidationError::QuiltDirMissing(_)) => {}
            other => panic!("expected QuiltDirMissing, got {other:?}"),
        }
    }

    #[test]
    fn missing_db_returns_database_missing() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".quilt")).unwrap();
        match validate_graph_layout(tmp.path()) {
            Err(GraphValidationError::DatabaseMissing(_)) => {}
            other => panic!("expected DatabaseMissing, got {other:?}"),
        }
    }

    #[test]
    fn unopenable_db_returns_database_unopenable() {
        // A real SQLite file with garbage content triggers the open
        // error path. SQLite is permissive on header parsing; we
        // overwrite the file with a few random bytes.
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        fs::write(&db_path, b"this is not a sqlite file").unwrap();
        match validate_graph_layout(tmp.path()) {
            Err(GraphValidationError::DatabaseUnopenable { path, .. }) => {
                assert_eq!(path, db_path);
            }
            other => panic!("expected DatabaseUnopenable, got {other:?}"),
        }
    }

    #[test]
    fn schema_incompatible_returns_schema_error() {
        // Empty (zero-byte) SQLite file opens cleanly but lacks the
        // marker table.
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        Connection::open(&db_path).unwrap(); // creates an empty db
        match validate_graph_layout(tmp.path()) {
            Err(GraphValidationError::SchemaIncompatible { path, .. }) => {
                assert_eq!(path, db_path);
            }
            other => panic!("expected SchemaIncompatible, got {other:?}"),
        }
    }

    #[test]
    fn valid_layout_returns_graph_layout() {
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        seed_quilt_db(&db_path);

        let layout = validate_graph_layout(tmp.path()).expect("valid");
        assert_eq!(layout.graph_path, tmp.path());
        assert_eq!(layout.quilt_dir, quilt_dir);
        assert_eq!(layout.db_path, db_path);
    }

    #[test]
    fn validation_does_not_create_files() {
        // The validator must not touch the filesystem.
        let tmp = tempdir().unwrap();
        // No .quilt/ at all.
        let _ = validate_graph_layout(tmp.path()); // ignored — error expected
        assert!(!tmp.path().join(".quilt").exists());

        // With .quilt/ but no quilt.db.
        let _ = fs::create_dir_all(tmp.path().join(".quilt"));
        let db_path = tmp.path().join(".quilt").join("quilt.db");
        let _ = validate_graph_layout(tmp.path()); // ignored — error expected
        assert!(!db_path.exists());
    }

    #[test]
    fn code_and_path_for_each_variant() {
        let p = PathBuf::from("/x");
        let cases: Vec<(GraphValidationError, &'static str, &Path)> = vec![
            (
                GraphValidationError::DirectoryMissing(p.clone()),
                "DirectoryMissing",
                &p,
            ),
            (
                GraphValidationError::QuiltDirMissing(p.clone()),
                "QuiltDirMissing",
                &p,
            ),
            (
                GraphValidationError::DatabaseMissing(p.clone()),
                "DatabaseMissing",
                &p,
            ),
            (
                GraphValidationError::DatabaseUnopenable {
                    path: p.clone(),
                    reason: "x".into(),
                },
                "DatabaseUnopenable",
                &p,
            ),
            (
                GraphValidationError::SchemaIncompatible {
                    path: p.clone(),
                    detail: "x".into(),
                },
                "SchemaIncompatible",
                &p,
            ),
        ];
        for (err, expected_code, expected_path) in cases {
            assert_eq!(err.code(), expected_code);
            assert_eq!(err.path(), Some(expected_path));
        }
    }

    #[test]
    fn proptest_validate_never_panics_on_random_paths() {
        use proptest::prelude::*;
        proptest!(|(segments: Vec<String>)| {
            let tmp = tempdir().unwrap();
            let mut p = tmp.path().to_path_buf();
            for s in &segments {
                p = p.join(s);
            }
            // We don't care about the result — we care that it does
            // not panic. Both Ok and a typed Err are acceptable.
            let _ = validate_graph_layout(&p);
        });
    }
}
