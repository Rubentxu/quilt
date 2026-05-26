//! MCP Handlers module
//!
//! Contains presentation-layer traits and implementations for MCP tools and resources.
//! Each handler owns a domain of tools/resources and delegates to application use cases.

use crate::resources::Resource;
use crate::tools::Tool;
use async_trait::async_trait;

/// MCP tool handler trait — each handler owns a domain of tools.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Return tool definitions this handler owns.
    fn tools(&self) -> Vec<Tool>;
    /// Execute a tool by name.
    async fn execute(&self, name: &str, args: &serde_json::Value) -> Result<String, String>;
}

/// MCP resource provider trait — each provider owns a domain of resources.
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    /// Return resource definitions this provider owns.
    fn resources(&self) -> Vec<Resource>;
    /// Read a resource by URI.
    async fn read(&self, uri: &str) -> Result<String, String>;
}

pub mod block;
pub mod page;
pub mod query;
pub mod resource;
