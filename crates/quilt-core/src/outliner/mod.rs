//! Outliner core — block tree operations
//!
//! Pure domain logic with no framework coupling.
//! Compiles to WASM for use from React via wasm-bindgen.

pub mod history;
pub mod page;
pub mod tree;
// pub mod selection; // HYBRID — algorithm extracted, Leptos reactivity stays in quilt-ui
// pub mod drag; // HYBRID — algorithm extracted, Leptos reactivity stays in quilt-ui
