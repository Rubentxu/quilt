//! In-memory repository implementations for testing.
//!
//! These adapters store data in HashMaps, providing fast
//! test execution without SQLite dependency.
//!
//! **DEPRECATED**: Use `quilt_test_helpers` crate instead.
//! The wrappers in this module will be removed in a future release.

mod block;
mod page;
mod tag;

// No public re-exports - use quilt_test_helpers instead
