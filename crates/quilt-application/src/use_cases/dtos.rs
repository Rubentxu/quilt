//! DTOs for application-to-adapter communication
//!
//! These DTOs are the canonical shape shared across all adapters (HTTP, MCP, etc.).
//! Use case implementations return domain entities; adapters convert to these DTOs.

use serde::{Deserialize, Serialize};

use quilt_domain::entities::Block;

/// Block DTO — canonical shape shared across all adapters.
///
/// All adapters (quilt-server, quilt-mcp, etc.) MUST use this definition.
/// Convert from domain `Block` using `From` impl in the adapter layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub order: f64,
    pub level: u8,
    pub marker: Option<String>,
    pub priority: Option<String>,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub properties: serde_json::Value,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

impl From<Block> for BlockDto {
    fn from(block: Block) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.page_id.to_string(),
            parent_id: block.parent_id.map(|p| p.to_string()),
            content: block.content,
            order: block.order,
            level: block.level,
            marker: block.marker.map(|m| m.as_property_value().to_string()),
            priority: block.priority.map(|p| p.as_property_value().to_string()),
            collapsed: block.collapsed,
            properties: {
                let map: serde_json::Map<String, serde_json::Value> = block
                    .properties
                    .into_iter()
                    .map(|(k, v)| (k, v.to_json()))
                    .collect();
                serde_json::Value::Object(map)
            },
            refs: block.refs.into_iter().map(|r| r.to_string()).collect(),
            created_at: block.created_at.to_rfc3339(),
            updated_at: block.updated_at.to_rfc3339(),
        }
    }
}
