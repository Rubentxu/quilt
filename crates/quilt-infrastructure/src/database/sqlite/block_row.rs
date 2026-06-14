//! Block row parsing — converts SQLite rows to domain Block entities.
//!
//! This module provides the [`BlockRow`] struct and conversion logic
//! for mapping database rows to domain entities. It lives in the
//! infrastructure layer because it knows about the database schema.
//!
//! # Architecture
//!
//! - [`BlockRow`]: Intermediate representation with raw database fields
//! - [`BlockRow::from_row()`]: Parse SQL row → BlockRow
//! - [`BlockRow::to_block()`]: Convert BlockRow → domain Block
//!
//! This separation allows:
//! - Single source of truth for row↔entity mapping
//! - Reuse across repositories and query services
//! - Testability of conversion logic independent of DB access

use chrono::{DateTime, TimeZone, Utc};
use sqlx::Row;
use std::collections::HashMap;

use quilt_domain::content::BlockContent;
use quilt_domain::errors::DomainError;
use quilt_domain::value_objects::{BlockFormat, Priority, PropertyValue, TaskMarker, Uuid};

// ── UUID Conversion ───────────────────────────────────────────────────

/// Convert a 16-byte blob to a UUID.
pub fn blob_to_uuid(blob: &[u8]) -> Result<Uuid, DomainError> {
    let bytes: [u8; 16] = blob.try_into().map_err(|_| {
        DomainError::InvalidData(format!("Invalid UUID blob length: {}", blob.len()))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

/// Convert an optional non-empty blob to an optional UUID.
pub fn optional_blob_to_uuid(blob: Option<&[u8]>) -> Result<Option<Uuid>, DomainError> {
    match blob {
        Some(b) if !b.is_empty() => Ok(Some(blob_to_uuid(b)?)),
        _ => Ok(None),
    }
}

// ── Timestamp Helpers ─────────────────────────────────────────────────

pub(crate) fn ts_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now))
}

pub(crate) fn optional_ts_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.map(ts_to_datetime)
}

// ── Block Row ─────────────────────────────────────────────────────────

/// Raw block data from the database.
///
/// This struct holds all columns from the `blocks` table before
/// conversion to the domain [`Block`] entity.
pub struct BlockRow {
    pub(crate) id: Vec<u8>,
    pub(crate) page_id: Vec<u8>,
    pub(crate) parent_id: Option<Vec<u8>>,
    pub(crate) order_index: f64,
    pub(crate) level: i64,
    pub(crate) format: String,
    pub(crate) marker: Option<String>,
    pub(crate) priority: Option<String>,
    pub(crate) content: String,
    pub(crate) properties: Vec<u8>,
    pub(crate) scheduled: Option<i64>,
    pub(crate) deadline: Option<i64>,
    pub(crate) start_time: Option<i64>,
    pub(crate) repeated: Option<i64>,
    pub(crate) logbook: Option<i64>,
    pub(crate) collapsed: i64,
    pub(crate) created_at: i64,
    pub(crate) updated_at: i64,
    pub(crate) refs: Vec<u8>,
    pub(crate) tags: Vec<u8>,
    pub(crate) journal_day: Option<i32>,
    pub(crate) updated_journal_day: Option<i32>,
    #[allow(dead_code)]
    pub(crate) deleted_at: Option<i64>,
}

impl BlockRow {
    /// Parse a [`BlockRow`] from a SQLite row.
    pub fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
        Ok(Self {
            id: row.get("id"),
            page_id: row.get("page_id"),
            parent_id: row.get("parent_id"),
            order_index: row.get("order_index"),
            level: row.get("level"),
            format: row.get("format"),
            marker: row.get("marker"),
            priority: row.get("priority"),
            content: row.get("content"),
            properties: row.get("properties"),
            scheduled: row.get("scheduled"),
            deadline: row.get("deadline"),
            start_time: row.get("start_time"),
            repeated: row.get("repeated"),
            logbook: row.get("logbook"),
            collapsed: row.get("collapsed"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            refs: row.get("refs"),
            tags: row.get("tags"),
            journal_day: row.get("journal_day"),
            updated_journal_day: row.get("updated_journal_day"),
            deleted_at: row.get("deleted_at"),
        })
    }

    /// Convert this [`BlockRow`] into a domain [`Block`] entity.
    pub fn to_block(&self) -> Result<Block, DomainError> {
        Ok(Block {
            id: blob_to_uuid(&self.id)?,
            page_id: blob_to_uuid(&self.page_id)?,
            parent_id: optional_blob_to_uuid(self.parent_id.as_deref())?,
            order: self.order_index,
            level: self.level as u8,
            format: parse_format(&self.format),
            marker: self.marker.as_deref().and_then(parse_marker),
            priority: self.priority.as_deref().and_then(parse_priority),
            content: parse_block_content(&self.content)?,
            properties: parse_properties(&self.properties),
            refs: parse_uuid_list(&self.refs),
            tags: parse_tag_list(&self.tags),
            scheduled: optional_ts_to_datetime(self.scheduled),
            deadline: optional_ts_to_datetime(self.deadline),
            start_time: optional_ts_to_datetime(self.start_time),
            repeated: optional_ts_to_datetime(self.repeated),
            logbook: optional_ts_to_datetime(self.logbook),
            collapsed: self.collapsed != 0,
            created_at: ts_to_datetime(self.created_at),
            updated_at: ts_to_datetime(self.updated_at),
            journal_day: self.journal_day,
            updated_journal_day: self.updated_journal_day,
        })
    }
}

// ── Internal Parsing Helpers ─────────────────────────────────────────

pub(crate) fn parse_marker(s: &str) -> Option<TaskMarker> {
    match s {
        "now" => Some(TaskMarker::Now),
        "later" => Some(TaskMarker::Later),
        "todo" => Some(TaskMarker::Todo),
        "done" => Some(TaskMarker::Done),
        "cancelled" => Some(TaskMarker::Cancelled),
        _ => None,
    }
}

pub(crate) fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "a" => Some(Priority::A),
        "b" => Some(Priority::B),
        "c" => Some(Priority::C),
        _ => None,
    }
}

pub(crate) fn parse_format(s: &str) -> BlockFormat {
    match s {
        "org" => BlockFormat::Org,
        _ => BlockFormat::Markdown,
    }
}

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

pub(crate) fn parse_uuid_list(blob: &[u8]) -> Vec<Uuid> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob)
        .ok()
        .map(|v| v.iter().filter_map(|s| Uuid::parse_str(s).ok()).collect())
        .unwrap_or_default()
}

pub(crate) fn parse_tag_list(blob: &[u8]) -> Vec<String> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob).unwrap_or_default()
}

/// Parse block content from JSON string.
///
/// Handles both legacy plain text (for migration) and new BlockContent JSON format.
pub fn parse_block_content(s: &str) -> Result<BlockContent, DomainError> {
    // Empty string -> empty content
    if s.is_empty() {
        return Ok(BlockContent::empty());
    }

    // Try to parse as JSON first (new format)
    if let Ok(content) = serde_json::from_str::<BlockContent>(s) {
        return Ok(content);
    }

    // Fallback: treat as plain text (legacy migration case)
    // This allows old data with plain text content to still work
    Ok(BlockContent::from_text(s))
}

/// Serialize block content to JSON string for database storage.
pub fn serialize_block_content(content: &BlockContent) -> String {
    serde_json::to_string(content).unwrap_or_else(|_| r#"{"segments":[]}"#.to_string())
}

// Re-export Block for convenience
pub use quilt_domain::entities::Block;
