//! MCP Tools definitions

use serde::{Deserialize, Serialize};

/// A tool that the MCP server exposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
