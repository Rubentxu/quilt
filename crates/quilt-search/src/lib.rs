//! Quilt Search
//!
//! This crate provides full-text search capabilities using FTS5.
//!
//! # Architecture
//!
//! - [`SearchService`]: High-level API with caching, FTS5 querying, fuzzy search
//! - [`SearchIndexManager`]: Index maintenance (rebuild, incremental, per-block)
//! - [`SearchResult`]: Unified result type with snippet and score

pub mod indexing;
pub mod search;

pub use indexing::{IndexHealth, SearchIndex, SearchIndexManager};
pub use search::{SearchResult, SearchService};
