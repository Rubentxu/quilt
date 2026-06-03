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
//! - [`protocol::McpRequest`] / [`protocol::McpResponse`]: Request/response types
//! - [`tools`]: Tool definitions for AI agent actions
//! - [`resources`]: Resource definitions for graph data access
//! - [`notifications`]: Event notification support
//! - [`handlers`]: Tool and resource handler implementations
//!
//! # Available Tools
//!
//! Core tools (14 total):
//! - `quilt_query`: Execute a Quilt DSL query
//! - `quilt_search`: Full-text search across blocks
//! - `quilt_create_block`: Create a new content block
//! - `quilt_delete_block`: Delete a block
//! - `quilt_link_blocks`: Create a reference between blocks
//! - `quilt_get_block_tree`: Get a block with its children
//! - `quilt_get_backlinks`: Get blocks that reference a block
//! - `quilt_list_pages`: List all pages
//! - `quilt_get_page_blocks`: Get all blocks on a page
//! - `quilt_get_journal`: Get journal page for a date
//! - `quilt_create_task`: Create a task (block with marker)
//! - `quilt_list_templates`: List all template pages (ADR-0007)
//! - `quilt_get_template_schema`: Get the full schema of one template (ADR-0007)
//! # Resources
//!
//! - `quilt://graph`: Full graph statistics
//! - `quilt://pages`: All pages list
//! - `quilt://journals`: Journal pages list
//! - `quilt://tags`: All tags with usage counts

pub mod handlers;
pub mod notifications;
pub mod protocol;
pub mod resources;
pub mod serialization;
pub mod server;
pub mod tools;

pub use protocol::{McpRequest, McpResponse};
pub use server::McpServer;

// Re-export use_cases for handler construction
pub use quilt_application::use_cases;
