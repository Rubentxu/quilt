//! Quilt MCP - Model Context Protocol layer
//!
//! This crate provides the MCP server, tools, resources, and notifications
//! for AI agent integration with the Quilt knowledge graph.
//!
//! # Architecture
//!
//! The MCP server bridges AI agents with the Quilt domain layer:
//!
//! - [`McpServer`]: Main server handling MCP requests
//! - [`server::McpRequest`] / [`server::McpResponse`]: Request/response types
//! - [`tools`]: Tool definitions for AI agent actions
//! - [`resources`]: Resource definitions for graph data access
//! - [`notifications`]: Event notification support
//! - [`errors`]: MCP protocol error codes and types
//!
//! # Available Tools
//!
//! - `logseq_query`: Execute a Logseq DSL query
//! - `create_block`: Create a new content block
//! - `search`: Full-text search across blocks
//! - `get_block_tree`: Get a block with its children
//! - `get_page_blocks`: Get all blocks on a page
//! - `list_pages`: List all pages
//! - `get_journal`: Get journal page for a date
//! - `create_task`: Create a task (block with marker)
//! - `link_blocks`: Create a reference between blocks
//! - `get_backlinks`: Get blocks that reference a block
//! - `delete_block`: Delete a block

pub mod errors;
pub mod hooks;
pub mod notifications;
pub mod plugin;
pub mod resources;
pub mod server;
pub mod tools;

pub use errors::{McpError, McpErrorCode};
pub use hooks::{
    HookDispatcher, HookError, HookEvent, HookEventKind, HookFilter, HookResult, HookSubscription,
    Priority,
};
pub use plugin::{Plugin, PluginContext, PluginError, PluginManifest, PluginRegistry};
pub use server::McpServer;
