//! MCP Protocol types for request/response handling.
//!
//! These types define the wire protocol for the Model Context Protocol.

use serde::{Deserialize, Serialize};

use crate::resources::Resource;
use crate::tools::Tool;

// ── Request types ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize { params: InitializeParams },
    #[serde(rename = "tools/list")]
    ListTools,
    #[serde(rename = "tools/call")]
    CallTool { params: CallToolParams },
    #[serde(rename = "resources/list")]
    ListResources,
    #[serde(rename = "resources/read")]
    ReadResource { params: ReadResourceParams },
    #[serde(rename = "notifications_enabled")]
    EnableNotifications,
}

#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<Roots>,
    pub sampling: Option<Sampling>,
}

#[derive(Debug, Deserialize)]
pub struct Roots {
    pub list: bool,
}

#[derive(Debug, Deserialize)]
pub struct Sampling {}

#[derive(Debug, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

// ── Response types ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
pub enum McpResponse {
    #[serde(rename = "initialize")]
    Initialize(InitializeResult),
    #[serde(rename = "tools/list")]
    ToolsList(ToolsListResult),
    #[serde(rename = "tools/call")]
    ToolsCall(ToolsCallResult),
    #[serde(rename = "resources/list")]
    ResourcesList(ResourcesListResult),
    #[serde(rename = "resources/read")]
    ResourcesRead(ResourceReadResult),
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolCapabilities,
    pub resources: ResourceCapabilities,
    pub notifications: NotificationCapabilities,
}

#[derive(Debug, Serialize)]
pub struct ToolCapabilities {
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ResourceCapabilities {
    pub subscribe: bool,
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationCapabilities {}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize)]
pub struct ToolsCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: Resource },
}

#[derive(Debug, Serialize)]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize)]
pub struct ResourceReadResult {
    pub contents: Vec<ResourceContent>,
}

#[derive(Debug, Serialize)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
}

// ── Graph View DTOs ───────────────────────────────────────────────────

/// A node in the knowledge graph (represents a page)
#[derive(Debug, Clone, Serialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub journal: bool,
}

/// An edge in the knowledge graph (represents a reference between pages)
#[derive(Debug, Clone, Serialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
}

/// Complete graph data with nodes and edges
#[derive(Debug, Clone, Serialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    pub last_updated: String,
}
