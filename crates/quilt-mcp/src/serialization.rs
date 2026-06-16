//! Serialization helpers for MCP types.

use quilt_domain::entities::Block;
use serde_json::{json, Value};

/// Convert a Block entity to a JSON value for MCP responses.
///
/// This produces a flat JSON object with the key fields of a block,
/// suitable for AI agent consumption.
pub fn block_to_json(block: &Block) -> Value {
    let mut obj = json!({
        "id": block.id.to_string(),
        "page_id": block.page_id.to_string(),
        "content": block.content,
        "order": block.order,
        "level": block.level,
        "collapsed": block.collapsed,
        "created_at": block.created_at.to_rfc3339(),
        "updated_at": block.updated_at.to_rfc3339(),
    });

    if let Some(parent_id) = block.parent_id {
        obj["parent_id"] = json!(parent_id.to_string());
    } else {
        obj["parent_id"] = Value::Null;
    }

    if let Some(marker) = &block.marker {
        obj["marker"] = json!(marker.name());
    } else {
        obj["marker"] = Value::Null;
    }

    if let Some(priority) = &block.priority {
        obj["priority"] = json!(priority.as_property_value());
    } else {
        obj["priority"] = Value::Null;
    }

    obj
}
