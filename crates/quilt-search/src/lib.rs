//! Quilt Search
//!
//! This crate provides full-text search capabilities using FTS5.
//!
//! # Architecture
//!
//! - [`SearchService`]: High-level API with caching, FTS5 querying, fuzzy search
//! - [`SearchIndex`]: Index maintenance (rebuild, incremental, per-block)
//! - [`SearchResult`]: Unified result type with snippet and score

pub mod indexing;
pub mod search;

pub use indexing::SearchIndex;
pub use search::{SearchError, SearchResult, SearchService, SearchServiceTrait};
