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
pub mod sync;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
