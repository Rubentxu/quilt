//! Quilt Platform
//!
//! This crate provides platform adapters: CLI and bootstrap helpers
//! for the Graph Space lifecycle (ADR-0030).

pub mod cli;
pub mod global_app_state;
pub mod graph_validation;
pub mod init;

// Re-export the in-memory global-state fallback so the
// `open_global_state` factory doesn't have to depend on
// `quilt-test-helpers` (which is a dev-only crate).
pub use quilt_test_helpers::InMemoryGlobalAppStateRepository;

// Re-export the SQLite-backed global state repo so callers can also
// open it directly when they want strict control over the path.
pub use quilt_infrastructure::database::sqlite::SqliteGlobalAppStateRepository;

pub use cli::QuiltCLI;
