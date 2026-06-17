//! Shared initialization for Quilt application state
//!
//! This module provides common initialization logic that can be shared between
//! different entry points (Tauri desktop app, HTTP server, CLI).
//!
//! # Graph Space model (ADR-0030)
//!
//! Quilt operates on a **Graph Space** model: a user-chosen directory
//! (`<graph-root>`) hosts Quilt's canonical persistence under
//! `<graph-root>/.quilt/quilt.db`.
//!
//! The canonical entry point is [`init_graph`]; it resolves the layout
//! and the canonical database path without modifying the filesystem
//! (in its [`ensure_graph_layout`] form) or with creation (in the
//! [`init_graph`] form).
//!
//! For backwards compatibility (one release window), the legacy
//! `Vault*` symbols are kept as deprecated aliases that delegate to
//! `Graph*`. They will be removed in the next minor release.

use std::path::{Path, PathBuf};

pub use crate::graph_validation::{GraphLayout, GraphValidationError};

/// Configuration for graph initialization
///
/// A `GraphConfig` represents a resolved Graph Space: the user-chosen
/// `<graph-root>` directory and the canonical database path inside it.
///
/// Per ADR-0030, the canonical location is `<graph-root>/.quilt/quilt.db`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphConfig {
    /// Path to the graph root directory (user-chosen).
    pub graph_path: PathBuf,
    /// Path to the canonical database file (`<graph-root>/.quilt/quilt.db`).
    pub db_path: PathBuf,
}

impl GraphConfig {
    /// Get the database URL for SQLx
    pub fn database_url(&self) -> String {
        format!("sqlite:{}?mode=rwc", self.db_path.display())
    }

    /// The canonical `.quilt/` directory inside the graph root.
    pub fn quilt_dir(&self) -> PathBuf {
        self.graph_path.join(".quilt")
    }
}

/// Graph Space setup errors
///
/// These are pure bootstrap errors: the filesystem layout could not be
/// resolved, the directory could not be created, the layout failed
/// validation, or migrations failed. The set of error variants is
/// stable; adding new variants is a breaking change.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Failed to create graph directory: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("Failed to serialize graph config: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(#[from] anyhow::Error),
    /// The graph layout is not a valid Quilt graph (per ADR-0030 §6).
    /// Surfaces a typed validation error without auto-repair.
    #[error("Graph validation failed: {0}")]
    Validation(#[from] GraphValidationError),
}

/// Resolve the canonical graph layout from a graph root directory.
///
/// Returns the path to the canonical database (`<graph-root>/.quilt/quilt.db`).
/// **Does not** create any directory or file — it only computes the path.
///
/// Use [`init_graph`] for the creating variant (used only by `POST /api/v1/graphs/create`).
pub fn ensure_graph_layout(graph_path: &Path) -> PathBuf {
    graph_path.join(".quilt").join("quilt.db")
}

/// Initialize a Graph Space from a path, creating the necessary layout.
///
/// This is the **canonical** entry point for server, CLI and MCP bootstrap.
/// It creates the `.quilt/` directory and an empty `quilt.db` file if they
/// don't exist, and returns a [`GraphConfig`] with the resolved paths.
///
/// Existing Graph Spaces: this is a no-op when the layout already exists.
/// The filesystem is only touched on first creation.
///
/// **Note**: this function does **not** validate an existing layout
/// against the schema marker. If the file already exists but is not a
/// Quilt graph, the function will return its path without complaint.
/// Use [`init_graph_validated`] for the strict "fail explicitly"
/// behaviour required by ADR-0030 §6.
pub fn init_graph(graph_path: PathBuf) -> Result<GraphConfig, GraphError> {
    let db_path = ensure_graph_layout_exists(&graph_path)?;
    Ok(GraphConfig {
        graph_path,
        db_path,
    })
}

/// Initialize a Graph Space **with strict validation**.
///
/// This variant:
/// 1. If the layout does not exist → creates it (no validation needed).
/// 2. If the layout exists → runs [`validate_graph_layout`] and
///    surfaces a [`GraphValidationError`] if the layout is broken
///    (missing `.quilt/`, missing `quilt.db`, unopenable database,
///    or schema-incompatible).
///
/// This is the **preferred** bootstrap for server, CLI and MCP
/// startup, per ADR-0030 §6 (no silent auto-repair).
pub fn init_graph_validated(graph_path: PathBuf) -> Result<GraphConfig, GraphError> {
    // If the graph root does not exist OR .quilt/ is missing,
    // we follow the same create-on-first-use semantics as
    // `init_graph` (this is "fresh create" not "auto-repair").
    if !graph_path.join(".quilt").exists() {
        let db_path = ensure_graph_layout_exists(&graph_path)?;
        return Ok(GraphConfig {
            graph_path,
            db_path,
        });
    }
    // Layout exists: validate strictly.
    let layout = crate::graph_validation::validate_graph_layout(&graph_path)?;
    Ok(GraphConfig {
        graph_path: layout.graph_path,
        db_path: layout.db_path,
    })
}

/// Internal: ensure the layout exists. Kept private so callers cannot
/// accidentally re-create on the side; use [`init_graph`] for the public API.
fn ensure_graph_layout_exists(graph_path: &Path) -> Result<PathBuf, GraphError> {
    let quilt_dir = graph_path.join(".quilt");
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

/// Create database pool and run migrations.
///
/// This is the canonical "ready-to-use pool" factory used by every entry point.
pub async fn create_db_pool(db_path: &Path) -> Result<sqlx::Pool<sqlx::Sqlite>, GraphError> {
    let pool = quilt_infrastructure::database::sqlite::connection::create_pool(db_path).await?;
    quilt_infrastructure::database::sqlite::connection::run_migrations(&pool).await?;
    Ok(pool)
}

// ── Deprecated aliases (one release window) ─────────────────────────────────
//
// These symbols are kept alive for one minor release to preserve
// compatibility with downstream code that still uses the old
// "vault" terminology. They will be removed in the next minor release.

/// **Deprecated.** Use [`GraphConfig`] instead.
///
/// This type alias is kept alive for one release. It will be removed
/// in the next minor release. See ADR-0030.
#[deprecated(
    since = "0.2.0",
    note = "use GraphConfig; will be removed in the next minor release (see ADR-0030)"
)]
pub type VaultConfig = GraphConfig;

/// **Deprecated.** Use [`GraphError`] instead.
#[deprecated(
    since = "0.2.0",
    note = "use GraphError; will be removed in the next minor release (see ADR-0030)"
)]
pub type VaultError = GraphError;

/// **Deprecated.** Use [`ensure_graph_layout`] for non-creating resolution,
/// or [`init_graph`] for the creating variant.
#[deprecated(
    since = "0.2.0",
    note = "use ensure_graph_layout or init_graph; will be removed in the next minor release (see ADR-0030)"
)]
pub fn ensure_vault_exists(vault_path: &Path) -> Result<PathBuf, GraphError> {
    // Old behaviour auto-created the directory; for the deprecation
    // window we still call the creating variant, so existing callers
    // keep their current semantics.
    ensure_graph_layout_exists(vault_path)
}

/// **Deprecated.** Use [`init_graph`] (or [`init_graph_validated`] for
/// strict behaviour) instead.
///
/// Logs a `tracing::warn!` to stderr on every call so callers notice
/// the deprecation before the symbol is removed.
#[deprecated(
    since = "0.2.0",
    note = "use init_graph; will be removed in the next minor release (see ADR-0030)"
)]
pub fn init_vault(vault_path: PathBuf) -> Result<VaultConfig, VaultError> {
    tracing::warn!(
        target: "quilt_platform::deprecation",
        "init_vault is deprecated; use init_graph instead (will be removed in next minor release)"
    );
    Ok(init_graph(vault_path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn ensure_graph_layout_resolves_canonical_path() {
        let tmp = tempdir().unwrap();
        let layout = ensure_graph_layout(tmp.path());
        assert_eq!(layout, tmp.path().join(".quilt").join("quilt.db"));
    }

    #[test]
    fn init_graph_creates_layout_in_empty_dir() {
        let tmp = tempdir().unwrap();
        let graph_path = tmp.path().to_path_buf();

        let cfg = init_graph(graph_path.clone()).expect("init_graph should succeed");

        assert_eq!(cfg.graph_path, graph_path);
        assert_eq!(cfg.db_path, tmp.path().join(".quilt").join("quilt.db"));
        assert!(cfg.quilt_dir().exists(), ".quilt/ should be created");
        assert!(cfg.db_path.exists(), "quilt.db should be created");
    }

    #[test]
    fn init_graph_is_idempotent() {
        let tmp = tempdir().unwrap();
        let graph_path = tmp.path().to_path_buf();

        let cfg1 = init_graph(graph_path.clone()).unwrap();
        let cfg2 = init_graph(graph_path).unwrap();

        assert_eq!(cfg1, cfg2);
    }

    #[test]
    fn graph_config_database_url_uses_mode_rwc() {
        let cfg = GraphConfig {
            graph_path: PathBuf::from("/tmp/g"),
            db_path: PathBuf::from("/tmp/g/.quilt/quilt.db"),
        };
        let url = cfg.database_url();
        assert!(url.starts_with("sqlite:"));
        assert!(url.contains("mode=rwc"));
        assert!(url.contains("quilt.db"));
    }

    #[test]
    fn graph_config_quilt_dir_is_graph_path_dot_quilt() {
        let cfg = GraphConfig {
            graph_path: PathBuf::from("/var/data/g"),
            db_path: PathBuf::from("/var/data/g/.quilt/quilt.db"),
        };
        assert_eq!(cfg.quilt_dir(), PathBuf::from("/var/data/g/.quilt"));
    }

    #[test]
    fn init_graph_validated_creates_layout_when_missing() {
        let tmp = tempdir().unwrap();
        let cfg = init_graph_validated(tmp.path().to_path_buf())
            .expect("missing layout should be created");
        assert_eq!(cfg.db_path, tmp.path().join(".quilt").join("quilt.db"));
    }

    #[test]
    fn init_graph_validated_succeeds_on_valid_layout() {
        // First, seed a valid graph layout (with the user_settings table).
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        std::fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE user_settings (id INTEGER PRIMARY KEY, timezone TEXT);",
            )
            .unwrap();
        }

        let cfg = init_graph_validated(tmp.path().to_path_buf()).expect("valid layout should pass");
        assert_eq!(cfg.db_path, db_path);
    }

    #[test]
    fn init_graph_validated_fails_on_schema_incompatible() {
        // Empty SQLite file (no user_settings) → should fail.
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        std::fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        Connection::open(&db_path).unwrap(); // creates an empty db

        match init_graph_validated(tmp.path().to_path_buf()) {
            Err(GraphError::Validation(GraphValidationError::SchemaIncompatible {
                path, ..
            })) => {
                assert_eq!(path, db_path);
            }
            other => panic!("expected SchemaIncompatible, got {other:?}"),
        }
    }

    #[test]
    fn init_graph_validated_fails_on_missing_quilt_db() {
        // .quilt/ exists but no quilt.db → fail explicitly.
        let tmp = tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".quilt")).unwrap();
        match init_graph_validated(tmp.path().to_path_buf()) {
            Err(GraphError::Validation(GraphValidationError::DatabaseMissing(_))) => {}
            other => panic!("expected DatabaseMissing, got {other:?}"),
        }
    }

    #[test]
    fn init_graph_validated_does_not_overwrite_existing_db() {
        // Pre-existing valid graph must survive strict validation.
        let tmp = tempdir().unwrap();
        let quilt_dir = tmp.path().join(".quilt");
        std::fs::create_dir_all(&quilt_dir).unwrap();
        let db_path = quilt_dir.join("quilt.db");
        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE user_settings (id INTEGER PRIMARY KEY, timezone TEXT); \
                 INSERT INTO user_settings (id, timezone) VALUES (1, 'pre-existing-marker');",
            )
            .unwrap();
        }

        init_graph_validated(tmp.path().to_path_buf()).expect("valid existing graph must validate");
        // The DB must still be a valid SQLite file with our marker.
        let conn = Connection::open(&db_path).unwrap();
        let tz: String = conn
            .query_row("SELECT timezone FROM user_settings WHERE id = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(tz, "pre-existing-marker");
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_ensure_vault_exists_delegates_and_creates() {
        let tmp = tempdir().unwrap();
        let result =
            ensure_vault_exists(tmp.path()).expect("deprecation wrapper should still work");
        assert_eq!(result, tmp.path().join(".quilt").join("quilt.db"));
        assert!(result.exists());
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_init_vault_returns_graph_config() {
        let tmp = tempdir().unwrap();
        let cfg: VaultConfig = init_vault(tmp.path().to_path_buf()).unwrap();
        assert_eq!(cfg.db_path, tmp.path().join(".quilt").join("quilt.db"));
    }

    #[test]
    fn proptest_paths_resolve_canonical_layout() {
        use proptest::prelude::*;
        proptest!(|(
            segments in proptest::collection::vec(
                prop_oneof![
                    proptest::string::string_regex("[a-zA-Z0-9_.-]{1,16}").unwrap(),
                    proptest::string::string_regex("[\\p{L}\\p{N} _.-]{1,8}").unwrap(),
                ],
                1..5
            )
        )| {
            let tmp = tempdir().unwrap();
            let mut p = tmp.path().to_path_buf();
            for seg in &segments {
                p = p.join(seg);
                std::fs::create_dir_all(&p).unwrap();
            }
            let layout = ensure_graph_layout(&p);
            prop_assert_eq!(layout, p.join(".quilt").join("quilt.db"));
        });
    }
}
