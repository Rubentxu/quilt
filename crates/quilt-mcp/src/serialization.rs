//! Serialization helpers for MCP responses
//!
//! Shared conversion functions between domain types and JSON.

use quilt_application::Block;
use serde_json::Value;

/// Convert a Block to JSON value.
pub fn block_to_json(block: &Block) -> Value {
    serde_json::json!({
        "id": block.id.to_string(),
        "page_id": block.page_id.to_string(),
        "parent_id": block.parent_id.map(|id| id.to_string()),
        "order": block.order,
        "level": block.level,
        "content": block.content,
        "marker": block.marker.as_ref().map(|m| format!("{:?}", m)),
        "priority": block.priority.as_ref().map(|p| format!("{:?}", p)),
        "collapsed": block.collapsed,
        "created_at": block.created_at.to_rfc3339(),
        "updated_at": block.updated_at.to_rfc3339(),
    })
}
