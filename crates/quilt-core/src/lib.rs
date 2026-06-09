//! Quilt Core — Portable domain types and outliner logic
//!
//! Shared between Leptos (quilt-ui) and React (quilt-ui-react) frontends.
//! Compiles to both native and WASM targets.
//!
//! # Modules
//! - `graph`: Graph algorithms (analysis, force simulation)
//! - `types`: Domain DTOs (BlockDto, PageDto, etc.)
//! - `outliner`: Block tree operations, undo/redo history
//! - `parser`: Inline semantic parser
//! - `sync`: CRDT-based sync engine

pub mod graph;
pub mod outliner;
pub mod parser;
pub mod query;
pub mod schema;
pub mod scoring;
pub mod search;
pub mod strategy;
pub mod strategy_scoring;
pub mod sync;
pub mod types;

// Concrete strategy-scoring re-exports.
//
// The trait surface lives in `strategy`; the *default concrete*
// implementations (`RelevanceScorer`, `ScoredStrategySelector`) live
// in `strategy_scoring`. Re-exporting the most common ones at the
// crate root saves downstream callers from a deep import path
// (`quilt_core::strategy_scoring::RelevanceScorer`) when they just
// want the default.
pub use strategy_scoring::{
    block_with, RelevanceScorer, ScoredStrategySelector, NEUTRAL_SIGNAL,
    RECENCY_HALF_LIFE_HOURS, WEIGHT_PROPERTY_COMPLETENESS, WEIGHT_RECENCY,
    WEIGHT_SEMANTIC_SIMILARITY, WEIGHT_TYPE_MATCH,
};

#[cfg(target_arch = "wasm32")]
pub mod wasm;
