//! MCP Resources definitions

use serde::{Deserialize, Serialize};

/// A resource that the MCP server exposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}
