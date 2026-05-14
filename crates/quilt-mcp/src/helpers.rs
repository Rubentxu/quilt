//! Helper functions for MCP server tools.
//!
//! Shared utilities for parsing arguments and converting domain types to JSON.

use quilt_domain::entities::{Block, DeepLink};
use quilt_domain::value_objects::{TaskMarker, Uuid};

/// Parse a required UUID parameter from JSON args.
pub fn parse_uuid(args: &serde_json::Value, key: &str) -> Result<Uuid, String> {
    let s = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing '{}' parameter", key))?;
    Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))
}

/// Parse an optional UUID parameter from JSON args.
pub fn parse_optional_uuid(args: &serde_json::Value, key: &str) -> Result<Option<Uuid>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(Some(
            Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))?,
        )),
        _ => Ok(None),
    }
}

/// Parse an optional TaskMarker from JSON args.
pub fn parse_optional_marker(
    args: &serde_json::Value,
    key: &str,
) -> Result<Option<TaskMarker>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some("now") => Ok(Some(TaskMarker::Now)),
        Some("later") => Ok(Some(TaskMarker::Later)),
        Some("todo") => Ok(Some(TaskMarker::Todo)),
        Some("done") => Ok(Some(TaskMarker::Done)),
        Some("cancelled") => Ok(Some(TaskMarker::Cancelled)),
        Some("") => Ok(None),
        None => Ok(None),
        Some(other) => Err(format!("Invalid marker: {}", other)),
    }
}

/// Convert a Block entity to JSON value.
pub fn block_to_json(block: &Block) -> serde_json::Value {
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

/// Convert a DeepLink entity to JSON value.
pub fn deep_link_to_json(link: &DeepLink) -> serde_json::Value {
    serde_json::json!({
        "id": link.id.to_string(),
        "source_id": link.source_id.to_string(),
        "source_type": link.source_type.as_str(),
        "target_id": link.target_id.map(|id| id.to_string()),
        "target_page_name": link.target_page_name,
        "link_type": link.link_type.as_str(),
        "external_url": link.external_url,
        "link_text": link.link_text,
        "context": link.context,
        "created_at": link.created_at.to_rfc3339(),
    })
}
