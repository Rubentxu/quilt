//! Quilt Platform
//!
//! This crate provides platform adapters: Tauri desktop shell and CLI.

pub mod cli;
pub mod mcp_transport;
pub mod tauri;
pub mod watcher;

pub use cli::QuiltCLI;
