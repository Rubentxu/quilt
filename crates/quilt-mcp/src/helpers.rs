//! Helper functions for MCP server tools.
//!
//! Shared utilities for parsing arguments and converting domain types to JSON.

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

