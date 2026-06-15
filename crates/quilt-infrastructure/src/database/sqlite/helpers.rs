//! Shared helper functions for SQLite repository implementations.
//!
//! These helpers handle conversion between database representations
//! (blobs, timestamps) and domain types (Uuid, DateTime, etc.).

use chrono::{DateTime, Utc};
use std::collections::HashMap;

use quilt_domain::errors::DomainError;
use quilt_domain::value_objects::PropertyValue;
use quilt_domain::value_objects::{BlockFormat, Priority, TaskMarker, Uuid};

// ── UUID ↔ Blob conversions ─────────────────────────────────────────────

/// Convert a UUID to a blob (16 bytes) for SQLite storage.
pub(crate) fn uuid_to_blob(id: &Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

/// Convert a blob (16 bytes) to a UUID.
pub(crate) fn blob_to_uuid(blob: &[u8]) -> Result<Uuid, DomainError> {
    let bytes: [u8; 16] = blob.try_into().map_err(|_| {
        DomainError::InvalidData(format!("Invalid UUID blob length: {}", blob.len()))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

/// Convert an optional blob to an optional UUID.
pub(crate) fn optional_blob_to_uuid(blob: Option<&[u8]>) -> Result<Option<Uuid>, DomainError> {
    match blob {
        Some(b) if !b.is_empty() => Ok(Some(blob_to_uuid(b)?)),
        _ => Ok(None),
    }
}

// ── Timestamp ↔ DateTime conversions ────────────────────────────────────

/// Convert a Unix timestamp (milliseconds) to a UTC DateTime.
pub(crate) fn ts_to_datetime(ts: i64) -> DateTime<Utc> {
    // The chrono's `timestamp_opt` (seconds, not millis) branch was
    // dead code: real values arrive in millis and never satisfy it.
    // The millis path can still fail for out-of-range timestamps
    // (year > 9999) — we fall back to `Utc::now` so the row is
    // materialised rather than dropped, matching the prior behaviour.
    // A real out-of-range value would be a data corruption; callers
    // that need a Result should wrap this call.
    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

/// Convert a DateTime to a Unix timestamp (milliseconds).
pub(crate) fn datetime_to_ts(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp()
}

// ── TaskMarker conversions ───────────────────────────────────────────────

pub(crate) fn parse_marker(s: &str) -> Option<TaskMarker> {
    match s.to_lowercase().as_str() {
        "now" => Some(TaskMarker::Now),
        "later" => Some(TaskMarker::Later),
        "todo" => Some(TaskMarker::Todo),
        "doing" => Some(TaskMarker::Doing),
        "done" => Some(TaskMarker::Done),
        "cancelled" => Some(TaskMarker::Cancelled),
        "waiting" => Some(TaskMarker::Waiting),
        _ => None,
    }
}

pub(crate) fn marker_to_str(m: &TaskMarker) -> &'static str {
    match m {
        TaskMarker::Now => "now",
        TaskMarker::Later => "later",
        TaskMarker::Todo => "todo",
        TaskMarker::Doing => "doing",
        TaskMarker::Done => "done",
        TaskMarker::Cancelled => "cancelled",
        TaskMarker::Waiting => "waiting",
    }
}

// ── Priority conversions ────────────────────────────────────────────────

pub(crate) fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "a" => Some(Priority::A),
        "b" => Some(Priority::B),
        "c" => Some(Priority::C),
        _ => None,
    }
}

pub(crate) fn priority_to_str(p: &Priority) -> &'static str {
    match p {
        Priority::A => "A",
        Priority::B => "B",
        Priority::C => "C",
    }
}

// ── BlockFormat conversions ─────────────────────────────────────────────

pub(crate) fn parse_format(s: &str) -> BlockFormat {
    match s {
        "org" => BlockFormat::Org,
        _ => BlockFormat::Markdown,
    }
}

pub(crate) fn format_to_str(f: &BlockFormat) -> &'static str {
    match f {
        BlockFormat::Markdown => "markdown",
        BlockFormat::Org => "org",
    }
}

// ── Properties conversions ──────────────────────────────────────────────

/// Parse a properties blob (JSON) into a HashMap.
pub(crate) fn parse_properties(blob: &[u8]) -> HashMap<String, PropertyValue> {
    if blob.is_empty() || blob == b"{}" {
        return HashMap::new();
    }
    serde_json::from_slice::<HashMap<String, serde_json::Value>>(blob)
        .ok()
        .map(|map| {
            map.into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        PropertyValue::from_json(&v)
                            .unwrap_or(PropertyValue::String(v.to_string())),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Serialize properties HashMap to a JSON string for SQLite storage.
pub(crate) fn properties_to_blob(props: &HashMap<String, PropertyValue>) -> String {
    let map: HashMap<String, serde_json::Value> = props
        .iter()
        .map(|(k, v)| (k.clone(), v.to_json()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

// ── UUID list conversions ───────────────────────────────────────────────

/// Parse a UUID list blob (JSON array of UUID strings) into a Vec.
pub(crate) fn parse_uuid_list(blob: &[u8]) -> Vec<Uuid> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob)
        .ok()
        .map(|v| v.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect())
        .unwrap_or_default()
}

/// Serialize a UUID list to a JSON string.
pub(crate) fn uuid_list_to_blob(ids: &[Uuid]) -> String {
    let arr: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

// ── Tag list conversions ────────────────────────────────────────────────

/// Parse a tag list blob (JSON array of strings) into a Vec.
pub(crate) fn parse_tag_list(blob: &[u8]) -> Vec<String> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob).unwrap_or_default()
}

/// Serialize a tag list to a JSON string.
pub(crate) fn tag_list_to_blob(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_marker_case_insensitive() {
        // Lowercase variants
        assert_eq!(parse_marker("todo"), Some(quilt_domain::value_objects::TaskMarker::Todo));
        assert_eq!(parse_marker("doing"), Some(quilt_domain::value_objects::TaskMarker::Doing));
        assert_eq!(parse_marker("done"), Some(quilt_domain::value_objects::TaskMarker::Done));
        assert_eq!(parse_marker("now"), Some(quilt_domain::value_objects::TaskMarker::Now));
        assert_eq!(parse_marker("later"), Some(quilt_domain::value_objects::TaskMarker::Later));
        assert_eq!(parse_marker("cancelled"), Some(quilt_domain::value_objects::TaskMarker::Cancelled));
        assert_eq!(parse_marker("waiting"), Some(quilt_domain::value_objects::TaskMarker::Waiting));

        // PascalCase variants (legacy DB data - grace period)
        assert_eq!(parse_marker("TODO"), Some(quilt_domain::value_objects::TaskMarker::Todo));
        assert_eq!(parse_marker("DOING"), Some(quilt_domain::value_objects::TaskMarker::Doing));
        assert_eq!(parse_marker("DONE"), Some(quilt_domain::value_objects::TaskMarker::Done));
        assert_eq!(parse_marker("Now"), Some(quilt_domain::value_objects::TaskMarker::Now));
        assert_eq!(parse_marker("WAITING"), Some(quilt_domain::value_objects::TaskMarker::Waiting));

        // Unknown
        assert_eq!(parse_marker("unknown"), None);
    }
}
