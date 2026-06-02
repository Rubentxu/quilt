//! Quilt Search
//!
//! This crate provides full-text search capabilities using FTS5.
//!
//! # Architecture
//!
//! - [`SearchService`]: High-level API with caching, FTS5 querying, fuzzy search
//! - [`SearchIndex`]: Index maintenance (rebuild, incremental, per-block)
//! - [`SearchResult`]: Unified result type with snippet and score
//! - [`sanitize`]: Pure FTS5 query sanitization (SRP, OCP, DIP)

pub mod indexing;
pub mod sanitize;
pub mod search;

pub use indexing::SearchIndex;
pub use indexing::SearchIndex as SearchIndexManager;
pub use sanitize::{
    build_fts5_match_query, sanitize_fts5_query, QuoteStrategy, SanitizationStrategy,
};
pub use search::{SearchError, SearchResult, SearchService, SearchServiceTrait};
