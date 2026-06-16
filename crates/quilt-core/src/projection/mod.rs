//! WASM projection module — client-side block projection resolution.
//!
//! Mirrors the server's `quilt-application::services::projection` (slice #4)
//! but operates on `BlockDto` + `serde_json::Value` instead of `Block` +
//! `PropertyValue`. No dependency on `quilt-domain` or `quilt-application` —
//! this module is a self-contained port that compiles to `wasm32-unknown-unknown`.
//!
//! # Module map
//!
//! | File             | Contents                                                          |
//! |------------------|-------------------------------------------------------------------|
//! | `view.rs`        | `WasmProjectionView`, `WasmDecoration`, `WasmLinkView`, enums     |
//! | `predicate.rs`   | `WasmPropertyPredicate` (9 variants, operates on JSON values)     |
//! | `resolver.rs`    | `WasmProjectionResolver` + 6 V1 contracts registry                |
//! | `parity_tests.rs`| 18-row parity test (WASM output == server reference impl)         |
//! | `contracts/`     | 6 V1 contract re-implementations (task, heading, media, date, link, default) |
//!
//! # V1 contract priorities
//!
//! | Priority     | Contract   |
//! |--------------|------------|
//! | 100          | `task`     |
//! | 150          | `heading`  |
//! | 200          | `media`    |
//! | 250          | `date`     |
//! | 300          | `link`     |
//! | `u32::MAX`   | `default`  |
//!
//! See [`resolver::WasmProjectionResolver::v1`] for the canonical registry.

pub mod contracts;
pub mod predicate;
pub mod resolver;
pub mod view;

#[cfg(test)]
mod parity_tests;

// Public surface
pub use predicate::WasmPropertyPredicate;
pub use resolver::{WasmContract, WasmProjectionResolver};
pub use view::{
    WasmDecoration, WasmDecorationKind, WasmLinkKind, WasmLinkView, WasmProjectionConflict,
    WasmProjectionView,
};
