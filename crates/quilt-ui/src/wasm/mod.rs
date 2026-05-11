//! WASM bindings module for quilt-ui
//!
//! This module provides JavaScript-accessible functions via wasm-bindgen
//! for browser/edge runtime communication with the MCP server.

pub mod bindings;
pub mod client;
pub mod signals;

pub use bindings::*;
pub use client::McpClient;
