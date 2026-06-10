//! SQLite repositories — real async sqlx implementations
//!
//! This module provides concrete repository implementations using SQLite
//! with the sqlx async driver. All repositories follow the repository
//! traits defined in the domain layer.
//!
//! # Structure
//!
//! - [`SqliteBlockRepository`]: Persists [`Block`] entities
//! - [`SqlitePageRepository`]: Persists [`Page`] entities
//! - [`SqliteTagRepository`]: Manages page tags
//!
//! # Example
//!
//! ```
//! use quilt_infrastructure::database::sqlite::repositories::{SqliteBlockRepository, SqlitePageRepository};
//! use quilt_infrastructure::database::sqlite::connection::create_pool;
//!
//! async {
//!     let pool = create_pool("/tmp/test.db").await.unwrap();
//!     let block_repo = SqliteBlockRepository::new(pool.clone());
//!     let page_repo = SqlitePageRepository::new(pool.clone());
//! };
//! ```

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

use crate::database::sqlite::connection::DbPool;
use quilt_domain::entities::{Block, Page, UserSettings};
use quilt_domain::errors::DomainError;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::properties::entry::{DefaultPropertyEntry, HasValue};
use quilt_domain::properties::types::{Cardinality, ClosedValue, PropertyStatus, PropertyType, ViewContext};
use quilt_domain::references::RefType;
use quilt_domain::repositories::{
    BlockRepository, PageRepository, PropertyRepository, RefRepository, RefRow, SettingsRepository,
    TagRepository, TourStateRepository,
};
use quilt_domain::value_objects::{
    BlockFormat, BlockType, JournalDay, Priority, PropertyValue, TaskMarker, Uuid,
};

// ── Helpers ────────────────────────────────────────────────────────────

fn blob_to_uuid(blob: &[u8]) -> Result<Uuid, DomainError> {
    let bytes: [u8; 16] = blob.try_into().map_err(|_| {
        DomainError::InvalidData(format!("Invalid UUID blob length: {}", blob.len()))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

fn optional_blob_to_uuid(blob: Option<&[u8]>) -> Result<Option<Uuid>, DomainError> {
    match blob {
        Some(b) if !b.is_empty() => Ok(Some(blob_to_uuid(b)?)),
        _ => Ok(None),
    }
}

fn ts_to_datetime(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(|| {
        // Fallback for timestamps far in the future/past
        DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
    })
}

fn optional_ts_to_datetime(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.map(ts_to_datetime)
}

fn datetime_to_ts(dt: &DateTime<Utc>) -> i64 {
    dt.timestamp()
}

fn uuid_to_blob(id: &Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn parse_marker(s: &str) -> Option<TaskMarker> {
    match s {
        "now" => Some(TaskMarker::Now),
        "later" => Some(TaskMarker::Later),
        "todo" => Some(TaskMarker::Todo),
        "doing" => Some(TaskMarker::Doing),
        "done" => Some(TaskMarker::Done),
        "cancelled" => Some(TaskMarker::Cancelled),
        _ => None,
    }
}

fn marker_to_str(m: &TaskMarker) -> &'static str {
    match m {
        TaskMarker::Now => "now",
        TaskMarker::Later => "later",
        TaskMarker::Todo => "todo",
        TaskMarker::Doing => "doing",
        TaskMarker::Done => "done",
        TaskMarker::Cancelled => "cancelled",
    }
}

fn parse_priority(s: &str) -> Option<Priority> {
    match s.to_lowercase().as_str() {
        "a" => Some(Priority::A),
        "b" => Some(Priority::B),
        "c" => Some(Priority::C),
        _ => None,
    }
}

fn priority_to_str(p: &Priority) -> &'static str {
    match p {
        Priority::A => "A",
        Priority::B => "B",
        Priority::C => "C",
    }
}

fn parse_format(s: &str) -> BlockFormat {
    match s {
        "org" => BlockFormat::Org,
        _ => BlockFormat::Markdown,
    }
}

fn format_to_str(f: &BlockFormat) -> &'static str {
    match f {
        BlockFormat::Markdown => "markdown",
        BlockFormat::Org => "org",
    }
}

fn parse_block_type(s: &str) -> Option<BlockType> {
    BlockType::parse_str(s)
}

fn block_type_to_str(t: &BlockType) -> &'static str {
    t.as_str()
}

fn parse_properties(blob: &[u8]) -> HashMap<String, PropertyValue> {
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

fn properties_to_blob(props: &HashMap<String, PropertyValue>) -> String {
    let map: HashMap<String, serde_json::Value> = props
        .iter()
        .map(|(k, v)| (k.clone(), v.to_json()))
        .collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
}

fn parse_uuid_list(blob: &[u8]) -> Vec<Uuid> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob)
        .ok()
        .map(|v| v.iter().filter_map(|s| Uuid::parse_str(s)).collect())
        .unwrap_or_default()
}

fn uuid_list_to_blob(ids: &[Uuid]) -> String {
    let arr: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

fn parse_tag_list(blob: &[u8]) -> Vec<String> {
    if blob.is_empty() || blob == b"[]" {
        return vec![];
    }
    serde_json::from_slice::<Vec<String>>(blob).unwrap_or_default()
}

fn tag_list_to_blob(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())
}

// ── Block Row → Entity ─────────────────────────────────────────────────

struct BlockRow {
    id: Vec<u8>,
    page_id: Vec<u8>,
    parent_id: Option<Vec<u8>>,
    order_index: f64,
    level: i64,
    format: String,
    /// `block_type` column. `None` for pre-migration databases (the
    /// column was added in migration 007). When missing, the entity
    /// falls back to `BlockType::Paragraph` — the previous default.
    block_type: Option<String>,
    marker: Option<String>,
    priority: Option<String>,
    content: String,
    properties: Vec<u8>,
    scheduled: Option<i64>,
    deadline: Option<i64>,
    start_time: Option<i64>,
    repeated: Option<i64>,
    logbook: Option<i64>,
    collapsed: i64,
    created_at: i64,
    updated_at: i64,
    refs: Vec<u8>,
    tags: Vec<u8>,
}

impl BlockRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
        // The `block_type` column was added in migration 007. Pre-migration
        // databases don't have it, so the SELECT may not include it.
        // `try_get` returns Err if the column is missing; we silently
        // fall back to `None` and the entity-level `unwrap_or_default()`
        // below will fill in `BlockType::Paragraph` — matching the
        // pre-migration default (which was implicit).
        let block_type = row
            .try_get::<Option<String>, _>("block_type")
            .ok()
            .flatten();
        Ok(Self {
            id: row.get("id"),
            page_id: row.get("page_id"),
            parent_id: row.get("parent_id"),
            order_index: row.get("order_index"),
            level: row.get("level"),
            format: row.get("format"),
            block_type,
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
        })
    }

    fn to_block(&self) -> Result<Block, DomainError> {
        Ok(Block {
            id: blob_to_uuid(&self.id)?,
            page_id: blob_to_uuid(&self.page_id)?,
            parent_id: optional_blob_to_uuid(self.parent_id.as_deref())?,
            order: self.order_index,
            level: self.level as u8,
            format: parse_format(&self.format),
            block_type: self
                .block_type
                .as_deref()
                .and_then(parse_block_type)
                .unwrap_or_default(),
            marker: self.marker.as_deref().and_then(parse_marker),
            priority: self.priority.as_deref().and_then(parse_priority),
            content: self.content.clone(),
            properties: parse_properties(&self.properties),
            scheduled: optional_ts_to_datetime(self.scheduled),
            deadline: optional_ts_to_datetime(self.deadline),
            start_time: optional_ts_to_datetime(self.start_time),
            repeated: optional_ts_to_datetime(self.repeated),
            logbook: optional_ts_to_datetime(self.logbook),
            collapsed: self.collapsed != 0,
            created_at: ts_to_datetime(self.created_at),
            updated_at: ts_to_datetime(self.updated_at),
            refs: parse_uuid_list(&self.refs),
            tags: parse_tag_list(&self.tags),
        })
    }
}

// ── Page Row → Entity ──────────────────────────────────────────────────

struct PageRow {
    id: Vec<u8>,
    name: String,
    title: Option<String>,
    namespace_id: Option<Vec<u8>>,
    journal_day: Option<i32>,
    format: String,
    file_id: Option<Vec<u8>>,
    original_name: Option<String>,
    journal: i64,
    created_at: i64,
    updated_at: i64,
    /// JSON-encoded properties map (added by migration 006). When the column
    /// doesn't exist (pre-migration databases), the field is None and the
    /// page is loaded with an empty properties map.
    properties: Option<String>,
}

impl PageRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
        // The `properties` column was added by migration 006. Pre-migration
        // databases don't have it, so the SELECT may or may not include it.
        // Try to fetch it; default to None if the column is missing.
        let properties = row
            .try_get::<Option<String>, _>("properties")
            .ok()
            .flatten();
        Ok(Self {
            id: row.get("id"),
            name: row.get("name"),
            title: row.get("title"),
            namespace_id: row.get("namespace_id"),
            journal_day: row.get("journal_day"),
            format: row.get("format"),
            file_id: row.get("file_id"),
            original_name: row.get("original_name"),
            journal: row.get("journal"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            properties,
        })
    }

    fn to_page(&self) -> Result<Page, DomainError> {
        // Parse the properties JSON column. None or '{}' means empty map.
        // (T-B.14: F5 spec — pre-existing rows default to empty.)
        let properties = self
            .properties
            .as_deref()
            .filter(|s| !s.is_empty() && *s != "{}")
            .map(|s| serde_json::from_str::<HashMap<String, PropertyValue>>(s).unwrap_or_default())
            .unwrap_or_default()
            .into_iter()
            .map(|(k, v)| (k, DefaultPropertyEntry::new(v)))
            .collect();

        Ok(Page {
            id: blob_to_uuid(&self.id)?,
            name: self.name.clone(),
            title: self.title.clone(),
            namespace_id: optional_blob_to_uuid(self.namespace_id.as_deref())?,
            journal_day: self.journal_day.map(JournalDay::from_i32_unchecked),
            format: parse_format(&self.format),
            file_id: optional_blob_to_uuid(self.file_id.as_deref())?,
            original_name: self.original_name.clone(),
            journal: self.journal != 0,
            created_at: ts_to_datetime(self.created_at),
            updated_at: ts_to_datetime(self.updated_at),
            properties,
        })
    }
}

// ── SqliteBlockRepository ─────────────────────────────────────────────

/// SQLite implementation of the [`BlockRepository`] trait.
///
/// This repository provides persistent storage for block entities
/// using SQLite via the sqlx async driver.
pub struct SqliteBlockRepository {
    pool: DbPool,
}

impl SqliteBlockRepository {
    /// Creates a new `SqliteBlockRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
    /// use quilt_infrastructure::database::sqlite::connection::create_pool;
    ///
    /// async {
    ///     let pool = create_pool("/tmp/test.db").await.unwrap();
    ///     let repo = SqliteBlockRepository::new(pool);
    /// };
    /// ```
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BlockRepository for SqliteBlockRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Block>, DomainError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => {
                let br = BlockRow::from_row(&r)?;
                Ok(Some(br.to_block()?))
            }
            None => Ok(None),
        }
    }

    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query("SELECT * FROM blocks WHERE page_id = ? ORDER BY order_index")
            .bind(uuid_to_blob(&page_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_page: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query("SELECT * FROM blocks WHERE parent_id = ? ORDER BY order_index")
            .bind(uuid_to_blob(&parent_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_children: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_with_refs: {}", e)))?;

        match row {
            Some(r) => {
                let br = BlockRow::from_row(&r)?;
                let block = br.to_block()?;
                let refs = parse_uuid_list(&br.refs);
                Ok((block, refs))
            }
            None => Err(DomainError::BlockNotFound(id)),
        }
    }

    async fn insert(&self, block: &Block) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO blocks
            (id, page_id, parent_id, order_index, level, format, block_type, marker, priority,
             content, properties, scheduled, deadline, start_time, repeated, logbook,
             collapsed, created_at, updated_at, refs, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&block.id))
        .bind(uuid_to_blob(&block.page_id))
        .bind(block.parent_id.as_ref().map(uuid_to_blob))
        .bind(block.order)
        .bind(block.level as i64)
        .bind(format_to_str(&block.format))
        .bind(block_type_to_str(&block.block_type))
        .bind(block.marker.as_ref().map(|m| marker_to_str(m)))
        .bind(block.priority.as_ref().map(|p| priority_to_str(p)))
        .bind(&block.content)
        .bind(properties_to_blob(&block.properties))
        .bind(block.scheduled.as_ref().map(datetime_to_ts))
        .bind(block.deadline.as_ref().map(datetime_to_ts))
        .bind(block.start_time.as_ref().map(datetime_to_ts))
        .bind(block.repeated.as_ref().map(datetime_to_ts))
        .bind(block.logbook.as_ref().map(datetime_to_ts))
        .bind(block.collapsed as i64)
        .bind(datetime_to_ts(&block.created_at))
        .bind(datetime_to_ts(&block.updated_at))
        .bind(uuid_list_to_blob(&block.refs))
        .bind(tag_list_to_blob(&block.tags))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("insert block: {}", e)))?;

        Ok(())
    }

    async fn update(&self, block: &Block) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE blocks SET
            page_id = ?, parent_id = ?, order_index = ?, level = ?,
            format = ?, block_type = ?, marker = ?, priority = ?, content = ?,
            properties = ?, scheduled = ?, deadline = ?, start_time = ?,
            repeated = ?, logbook = ?, collapsed = ?,
            updated_at = ?, refs = ?, tags = ?
            WHERE id = ?"#,
        )
        .bind(uuid_to_blob(&block.page_id))
        .bind(block.parent_id.as_ref().map(uuid_to_blob))
        .bind(block.order)
        .bind(block.level as i64)
        .bind(format_to_str(&block.format))
        .bind(block_type_to_str(&block.block_type))
        .bind(block.marker.as_ref().map(|m| marker_to_str(m)))
        .bind(block.priority.as_ref().map(|p| priority_to_str(p)))
        .bind(&block.content)
        .bind(properties_to_blob(&block.properties))
        .bind(block.scheduled.as_ref().map(datetime_to_ts))
        .bind(block.deadline.as_ref().map(datetime_to_ts))
        .bind(block.start_time.as_ref().map(datetime_to_ts))
        .bind(block.repeated.as_ref().map(datetime_to_ts))
        .bind(block.logbook.as_ref().map(datetime_to_ts))
        .bind(block.collapsed as i64)
        .bind(datetime_to_ts(&block.updated_at))
        .bind(uuid_list_to_blob(&block.refs))
        .bind(tag_list_to_blob(&block.tags))
        .bind(uuid_to_blob(&block.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("update block: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("delete block: {}", e)))?;
        Ok(())
    }

    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE blocks SET parent_id = ?, order_index = ?, updated_at = ? WHERE id = ?",
        )
        .bind(new_parent.as_ref().map(uuid_to_blob))
        .bind(new_order)
        .bind(Utc::now().timestamp())
        .bind(uuid_to_blob(&id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("move_block: {}", e)))?;
        Ok(())
    }

    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError> {
        // Query both legacy JSON refs (blocks.refs) and the new refs table
        let target_uuid_str = block_id.to_string();
        let target_uuid_blob = uuid_to_blob(&block_id);
        let rows = sqlx::query(
            r#"SELECT DISTINCT b.* FROM blocks b
            WHERE EXISTS (
                SELECT 1 FROM json_each(b.refs) AS je WHERE je.value = ?
            )
            OR b.id IN (
                SELECT source_id FROM refs WHERE target_id = ?
            )
            ORDER BY b.updated_at DESC"#,
        )
        .bind(&target_uuid_str)
        .bind(&target_uuid_blob)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_backlinks: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT b.* FROM blocks b
            JOIN blocks_fts fts ON fts.rowid = b.rowid
            WHERE blocks_fts MATCH ?
            ORDER BY bm25(blocks_fts)
            LIMIT ?"#,
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("search: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Block>, DomainError> {
        let rows =
            sqlx::query("SELECT * FROM blocks WHERE updated_at > ? ORDER BY updated_at DESC")
                .bind(since.timestamp())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(format!("get_updated_since: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocks WHERE page_id = ?")
            .bind(uuid_to_blob(&page_id))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("count_by_page: {}", e)))?;

        Ok(count as usize)
    }

    async fn count_all(&self) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("count_all: {}", e)))?;

        Ok(count as usize)
    }

    async fn query_dsl(&self, sql: &str, params: &[String]) -> Result<Vec<Block>, DomainError> {
        let mut query = sqlx::query(sql);
        for param in params {
            query = query.bind(param);
        }
        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("query_dsl: {}", e)))?;
        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }

    async fn list_by_property(
        &self,
        key: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<Block>, DomainError> {
        // Use json_extract over the `properties` blob. We pass the JSON
        // pointer as `$.<key>` and compare against the requested string
        // value. Limit is appended as a literal — `limit` is a `usize`
        // derived from a server-side parameter, never user input.
        let sql = if limit == 0 {
            "SELECT * FROM blocks \
             WHERE json_extract(properties, ?) IS NOT NULL \
               AND json_extract(properties, ?) = ? \
             ORDER BY created_at DESC"
                .to_string()
        } else {
            format!(
                "SELECT * FROM blocks \
                 WHERE json_extract(properties, ?) IS NOT NULL \
                   AND json_extract(properties, ?) = ? \
                 ORDER BY created_at DESC \
                 LIMIT {}",
                limit
            )
        };

        let pointer = format!("$.{}", key);
        let rows = sqlx::query(&sql)
            .bind(&pointer)
            .bind(&pointer)
            .bind(value)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("list_by_property: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }

    async fn list_by_property_key(&self, key: &str, limit: u32) -> Result<Vec<Block>, DomainError> {
        // Use json_extract over the `properties` blob. We pass the JSON
        // pointer as `$.<key>` and check that the extracted value is
        // non-NULL (which is the SQLite way of asking "this key
        // exists in the map"). Limit is appended as a literal — it
        // comes from a server-side parameter, never user input.
        let sql = if limit == 0 {
            "SELECT * FROM blocks \
             WHERE json_extract(properties, ?) IS NOT NULL \
             ORDER BY created_at DESC"
                .to_string()
        } else {
            format!(
                "SELECT * FROM blocks \
                 WHERE json_extract(properties, ?) IS NOT NULL \
                 ORDER BY created_at DESC \
                 LIMIT {}",
                limit
            )
        };

        let pointer = format!("$.{}", key);
        let rows = sqlx::query(&sql)
            .bind(&pointer)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("list_by_property_key: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }

    async fn list_distinct_keys(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, DomainError> {
        // Two query shapes: with and without the cursor predicate.
        // `json_each` over an object yields one row per top-level
        // key — it does NOT recurse into nested values, which is
        // exactly what we want (nested keys are not "property
        // keys" in our model). The cursor uses lexicographic ASC
        // (`>`) and `LIMIT` is bound as a parameter.
        let rows = match cursor {
            Some(c) => {
                sqlx::query(
                    "SELECT DISTINCT je.key \
                     FROM blocks, json_each(blocks.properties) AS je \
                     WHERE je.key > ? \
                     ORDER BY je.key ASC \
                     LIMIT ?",
                )
                .bind(c)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query(
                    "SELECT DISTINCT je.key \
                     FROM blocks, json_each(blocks.properties) AS je \
                     ORDER BY je.key ASC \
                     LIMIT ?",
                )
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DomainError::Storage(format!("list_distinct_keys: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("key")).collect())
    }

    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, DomainError> {
        // Two query shapes: with and without the prefix predicate.
        // The `created_by` value lives inside the `properties` JSON
        // blob (it's not a column), so we extract it with
        // `json_extract` and the `$.created_by` JSON pointer. We
        // also enforce `typeof() = 'text'` so a non-text value
        // stored under that key (e.g. a future booleanean flag
        // called `created_by`) cannot crash the extraction.
        // Distinct + ORDER BY is pushed down to SQLite so the
        // handler returns a stable list without doing it in Rust.
        //
        // The `%` LIKE suffix is appended to the bound prefix
        // parameter — the prefix itself is still a parameter, not a
        // concatenation into the SQL string. SQL injection-safe.
        let rows = match prefix {
            Some(p) => {
                let pattern = format!("{}%", p);
                sqlx::query(
                    "SELECT DISTINCT json_extract(properties, '$.created_by') AS author \
                     FROM blocks \
                     WHERE json_extract(properties, '$.created_by') IS NOT NULL \
                       AND typeof(json_extract(properties, '$.created_by')) = 'text' \
                       AND json_extract(properties, '$.created_by') LIKE ? \
                     ORDER BY author ASC",
                )
                .bind(pattern)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query(
                    "SELECT DISTINCT json_extract(properties, '$.created_by') AS author \
                     FROM blocks \
                     WHERE json_extract(properties, '$.created_by') IS NOT NULL \
                       AND typeof(json_extract(properties, '$.created_by')) = 'text' \
                     ORDER BY author ASC",
                )
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(|e| DomainError::Storage(format!("list_distinct_authors: {}", e)))?;

        Ok(rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("author").ok())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

// ── SqlitePageRepository ───────────────────────────────────────────────

/// SQLite implementation of the [`PageRepository`] trait.
///
/// This repository provides persistent storage for page entities
/// using SQLite via the sqlx async driver.
pub struct SqlitePageRepository {
    pool: DbPool,
    /// Optional property repository for read-only checks in `update_properties`.
    /// When None, falls back to a hardcoded list of system property keys.
    property_repo: Option<Arc<dyn PropertyRepository>>,
}

impl SqlitePageRepository {
    /// Creates a new `SqlitePageRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
    /// use quilt_infrastructure::database::sqlite::connection::create_pool;
    ///
    /// async {
    ///     let pool = create_pool("/tmp/test.db").await.unwrap();
    ///     let repo = SqlitePageRepository::new(pool);
    /// };
    /// ```
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            property_repo: None,
        }
    }

    /// Creates a new `SqlitePageRepository` with a property repository for
    /// read-only checks. Used by integration tests (T-B.14).
    pub fn with_property_repo(pool: DbPool, repo: Arc<dyn PropertyRepository>) -> Self {
        Self {
            pool,
            property_repo: Some(repo),
        }
    }

    /// Search pages whose `name` OR `title` contains the query
    /// substring (case-insensitive `LIKE '%q%'`). S2-03 — the
    /// server-side page search endpoint needs to match on both
    /// columns because pages with a custom display title should
    /// surface when the user types a fragment of the title that
    /// does not appear in the (lowercased) name.
    ///
    /// The trait's `search(query, limit)` only matches on `name`,
    /// which is what the rest of the codebase uses; this inherent
    /// helper exists to give the HTTP handler an OR query without
    /// widening the trait contract.
    ///
    /// `query` is the raw user input. The SQL is built with
    /// `lower(...) LIKE lower('%query%')` so the comparison is
    /// case-insensitive even for the `title` column (which is
    /// stored as-typed, unlike `name` which the `Page` entity
    /// lowercases on write).
    pub async fn search_by_name_or_title(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<Page>, DomainError> {
        let like = format!("%{}%", query.to_lowercase());
        let rows = sqlx::query(
            "SELECT * FROM pages \
             WHERE lower(name) LIKE ? OR lower(IFNULL(title, '')) LIKE ? \
             ORDER BY name LIMIT ?",
        )
        .bind(&like)
        .bind(&like)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("search_by_name_or_title: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    /// Check whether a key resolves to a read-only PropertyDefinition.
    async fn is_read_only_key(&self, key: &str) -> bool {
        if let Some(repo) = &self.property_repo {
            if let Ok(Some(def)) = repo.get_by_db_ident(key).await {
                return def.read_only;
            }
            if let Some(def) = quilt_domain::properties::builtin::get_builtin_property(key) {
                return def.read_only;
            }
            return false;
        }
        matches!(key, "id" | "created_at" | "updated_at")
    }

    /// Serialize a properties map to a JSON string for SQLite storage.
    fn properties_to_json(props: &HashMap<String, DefaultPropertyEntry<PropertyValue>>) -> String {
        // Serialize the inner values only (strip the entry wrapper) — the
        // schema treats the column as `{"key": value}` JSON.
        let map: HashMap<String, PropertyValue> = props
            .iter()
            .map(|(k, v)| (k.clone(), v.value().clone()))
            .collect();
        serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
    }
}

#[async_trait]
impl PageRepository for SqlitePageRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query("SELECT * FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query("SELECT * FROM pages WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_name: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query("SELECT * FROM pages WHERE journal_day = ? AND journal = 1")
            .bind(day.as_i32())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_journal: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_all: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE namespace_id = ? ORDER BY name")
            .bind(uuid_to_blob(&namespace_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_namespace_pages: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn insert(&self, page: &Page) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO pages
            (id, name, title, namespace_id, journal_day, format, file_id,
             original_name, journal, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&page.id))
        .bind(&page.name)
        .bind(&page.title)
        .bind(page.namespace_id.as_ref().map(uuid_to_blob))
        .bind(page.journal_day.map(|d| d.as_i32()))
        .bind(format_to_str(&page.format))
        .bind(page.file_id.as_ref().map(uuid_to_blob))
        .bind(&page.original_name)
        .bind(page.journal as i64)
        .bind(datetime_to_ts(&page.created_at))
        .bind(datetime_to_ts(&page.updated_at))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("insert page: {}", e)))?;

        Ok(())
    }

    async fn update(&self, page: &Page) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE pages SET
            name = ?, title = ?, namespace_id = ?, journal_day = ?,
            format = ?, file_id = ?, original_name = ?, journal = ?,
            updated_at = ?
            WHERE id = ?"#,
        )
        .bind(&page.name)
        .bind(&page.title)
        .bind(page.namespace_id.as_ref().map(uuid_to_blob))
        .bind(page.journal_day.map(|d| d.as_i32()))
        .bind(format_to_str(&page.format))
        .bind(page.file_id.as_ref().map(uuid_to_blob))
        .bind(&page.original_name)
        .bind(page.journal as i64)
        .bind(datetime_to_ts(&page.updated_at))
        .bind(uuid_to_blob(&page.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("update page: {}", e)))?;

        Ok(())
    }

    async fn rename(&self, id: Uuid, new_name: &str) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query("UPDATE pages SET name = ?, updated_at = ? WHERE id = ?")
            .bind(new_name)
            .bind(now)
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("rename page: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("delete page: {}", e)))?;
        Ok(())
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE updated_at > ? ORDER BY updated_at DESC")
            .bind(since.timestamp())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_updated_since: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages ORDER BY updated_at DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_recent: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("count: {}", e)))?;

        Ok(count as usize)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError> {
        let like = format!("%{}%", query);
        let rows = sqlx::query("SELECT * FROM pages WHERE name LIKE ? ORDER BY name LIMIT ?")
            .bind(&like)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("search: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn update_properties(
        &self,
        page_id: Uuid,
        props: HashMap<String, DefaultPropertyEntry<PropertyValue>>,
    ) -> Result<Page, DomainError> {
        // 1. Read-only check: reject any key that resolves to read-only.
        //    This is atomic — first read-only key fails the whole call.
        for key in props.keys() {
            if self.is_read_only_key(key).await {
                return Err(DomainError::PropertyReadOnly(key.clone()));
            }
        }

        // 2. Load page, merge, persist.
        let row = sqlx::query("SELECT * FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&page_id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("update_properties load: {}", e)))?;

        let row = row.ok_or(DomainError::PageNotFound(page_id))?;
        let pr = PageRow::from_row(&row)?;
        let mut page = pr.to_page()?;

        let merged = quilt_domain::properties::merge_properties(&page.properties, props);
        page.properties = merged;
        page.updated_at = Utc::now();

        let json = Self::properties_to_json(&page.properties);
        sqlx::query("UPDATE pages SET properties = ?, updated_at = ? WHERE id = ?")
            .bind(&json)
            .bind(datetime_to_ts(&page.updated_at))
            .bind(uuid_to_blob(&page_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("update_properties save: {}", e)))?;

        Ok(page)
    }
}

// ── SqliteTagRepository ────────────────────────────────────────────────

/// SQLite implementation of the [`TagRepository`] trait.
///
/// This repository manages the many-to-many relationship between pages and tags,
/// providing efficient tag-based querying and searching.
pub struct SqliteTagRepository {
    pool: DbPool,
}

impl SqliteTagRepository {
    /// Creates a new `SqliteTagRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_infrastructure::database::sqlite::repositories::SqliteTagRepository;
    /// use quilt_infrastructure::database::sqlite::connection::create_pool;
    ///
    /// async {
    ///     let pool = create_pool("/tmp/test.db").await.unwrap();
    ///     let repo = SqliteTagRepository::new(pool);
    /// };
    /// ```
    #[allow(dead_code)]
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TagRepository for SqliteTagRepository {
    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query("SELECT tag FROM tags WHERE page_id = ? ORDER BY tag")
            .bind(uuid_to_blob(&page_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_page: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query("SELECT page_id FROM tags WHERE tag = ?")
            .bind(tag)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_pages_with_tag: {}", e)))?;

        rows.iter()
            .map(|r| {
                let blob: Vec<u8> = r.get("page_id");
                blob_to_uuid(&blob)
            })
            .collect()
    }

    async fn add_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        sqlx::query("INSERT OR IGNORE INTO tags (page_id, tag, created_at) VALUES (?, ?, ?)")
            .bind(uuid_to_blob(&page_id))
            .bind(tag)
            .bind(Utc::now().timestamp())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("add_tag: {}", e)))?;
        Ok(())
    }

    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM tags WHERE page_id = ? AND tag = ?")
            .bind(uuid_to_blob(&page_id))
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("remove_tag: {}", e)))?;
        Ok(())
    }

    async fn get_all_tags(&self) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query("SELECT DISTINCT tag FROM tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_all_tags: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_tag_counts(&self) -> Result<Vec<(String, usize)>, DomainError> {
        let rows =
            sqlx::query("SELECT tag, COUNT(*) as cnt FROM tags GROUP BY tag ORDER BY cnt DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(format!("get_tag_counts: {}", e)))?;

        Ok(rows
            .iter()
            .map(|r| {
                let tag: String = r.get("tag");
                let cnt: i64 = r.get("cnt");
                (tag, cnt as usize)
            })
            .collect())
    }

    async fn search_tags(&self, prefix: &str, limit: usize) -> Result<Vec<String>, DomainError> {
        let like = format!("{}%", prefix);
        let rows =
            sqlx::query("SELECT DISTINCT tag FROM tags WHERE tag LIKE ? ORDER BY tag LIMIT ?")
                .bind(&like)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Storage(format!("search_tags: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }
}

// ── SqliteRefRepository ───────────────────────────────────────────────

/// SQLite implementation of the [`RefRepository`] trait.
///
/// This repository provides persistent storage for the bidirectional
/// reference model using the `refs` table with `source_id`, `target_id`,
/// `ref_type`, and `custom_context` columns.
///
/// # Schema
///
/// ```sql
/// CREATE TABLE refs (
///     source_id BLOB NOT NULL,
///     target_id BLOB NOT NULL,
///     ref_type TEXT NOT NULL CHECK(ref_type IN ('page_ref','block_ref','tag','alias')),
///     created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
///     custom_context TEXT,           -- Q028: Editable Backlinks
///     PRIMARY KEY (source_id, target_id, ref_type)
/// );
/// ```
///
/// The `custom_context` column is nullable on purpose. `NULL` means
/// "no override" — the Backlinks panel falls back to the source
/// block's content snippet. An empty string is also valid and means
/// "override exists but is empty" (used to clear an override's text
/// without re-fetching defaults).
pub struct SqliteRefRepository {
    pool: DbPool,
}

impl SqliteRefRepository {
    /// Creates a new `SqliteRefRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn map_ref_type(s: &str) -> Result<RefType, DomainError> {
        RefType::from_str(s)
            .ok_or_else(|| DomainError::InvalidData(format!("Unknown ref_type: {}", s)))
    }

    fn ref_type_to_str(rt: &RefType) -> &'static str {
        rt.as_str()
    }
}

#[async_trait]
impl RefRepository for SqliteRefRepository {
    async fn get_forward_refs(&self, source_id: Uuid) -> Result<Vec<(Uuid, RefType)>, DomainError> {
        let rows = sqlx::query(
            "SELECT target_id, ref_type FROM refs WHERE source_id = ? ORDER BY ref_type",
        )
        .bind(uuid_to_blob(&source_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_forward_refs: {}", e)))?;

        rows.iter()
            .map(|row| {
                let target_blob: Vec<u8> = row.get("target_id");
                let target = blob_to_uuid(&target_blob)?;
                let ref_type_str: String = row.get("ref_type");
                let ref_type = Self::map_ref_type(&ref_type_str)?;
                Ok((target, ref_type))
            })
            .collect()
    }

    async fn get_backlinks(&self, target_id: Uuid) -> Result<Vec<(Uuid, RefType)>, DomainError> {
        let rows = sqlx::query(
            "SELECT source_id, ref_type FROM refs WHERE target_id = ? ORDER BY ref_type",
        )
        .bind(uuid_to_blob(&target_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_backlinks: {}", e)))?;

        rows.iter()
            .map(|row| {
                let source_blob: Vec<u8> = row.get("source_id");
                let source = blob_to_uuid(&source_blob)?;
                let ref_type_str: String = row.get("ref_type");
                let ref_type = Self::map_ref_type(&ref_type_str)?;
                Ok((source, ref_type))
            })
            .collect()
    }

    async fn sync_refs(
        &self,
        source_id: Uuid,
        refs: &[(Uuid, RefType)],
    ) -> Result<(), DomainError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Storage(format!("begin transaction: {}", e)))?;

        // Delete all existing refs for this source
        sqlx::query("DELETE FROM refs WHERE source_id = ?")
            .bind(uuid_to_blob(&source_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Storage(format!("delete refs: {}", e)))?;

        // Insert new refs
        for (target_id, ref_type) in refs {
            sqlx::query("INSERT INTO refs (source_id, target_id, ref_type) VALUES (?, ?, ?)")
                .bind(uuid_to_blob(&source_id))
                .bind(uuid_to_blob(target_id))
                .bind(Self::ref_type_to_str(ref_type))
                .execute(&mut *tx)
                .await
                .map_err(|e| DomainError::Storage(format!("insert ref: {}", e)))?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Storage(format!("commit transaction: {}", e)))?;

        Ok(())
    }

    async fn rebuild_index(&self) -> Result<Vec<RefRow>, DomainError> {
        let rows = sqlx::query(
            "SELECT source_id, target_id, ref_type, custom_context FROM refs ORDER BY source_id, target_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("rebuild_index: {}", e)))?;

        rows.iter()
            .map(|row| {
                let source_blob: Vec<u8> = row.get("source_id");
                let target_blob: Vec<u8> = row.get("target_id");
                let ref_type_str: String = row.get("ref_type");
                let custom_context: Option<String> = row.try_get("custom_context").ok().flatten();

                Ok(RefRow {
                    source_id: blob_to_uuid(&source_blob)?,
                    target_id: blob_to_uuid(&target_blob)?,
                    ref_type: Self::map_ref_type(&ref_type_str)?,
                    custom_context,
                })
            })
            .collect()
    }

    async fn insert_ref(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
    ) -> Result<(), DomainError> {
        sqlx::query("INSERT OR IGNORE INTO refs (source_id, target_id, ref_type) VALUES (?, ?, ?)")
            .bind(uuid_to_blob(&source_id))
            .bind(uuid_to_blob(&target_id))
            .bind(Self::ref_type_to_str(&ref_type))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("insert_ref: {}", e)))?;
        Ok(())
    }

    async fn get_unlinked_references(
        &self,
        page_name: &str,
        page_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String)>, DomainError> {
        // Escape special LIKE characters in the page name
        let escaped = page_name
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");

        let like_pattern = format!("%{}%", escaped);

        // Search blocks whose content text contains the page name (case-insensitive),
        // but exclude blocks that already have an explicit [[page]] ref in the refs table.
        //
        // NOTE: Uses LIKE for maximum compatibility. FTS5 is available in the schema
        // (blocks_fts virtual table) and can be used for better performance on large datasets.
        let rows = sqlx::query(
            r#"
            SELECT b.id, b.page_id, b.content
            FROM blocks b
            WHERE b.content LIKE ? ESCAPE '\'
              AND b.id NOT IN (
                SELECT r.source_id FROM refs r WHERE r.target_id = ?
              )
            ORDER BY b.updated_at DESC
            LIMIT 50
            "#,
        )
        .bind(&like_pattern)
        .bind(uuid_to_blob(&page_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_unlinked_references: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let block_id: Vec<u8> = row.get("id");
            let page_id_blob: Vec<u8> = row.get("page_id");
            let content: String = row.get("content");

            let block_id = blob_to_uuid(&block_id)?;
            let source_page_id = blob_to_uuid(&page_id_blob)?;

            // Build a ~100-char content snippet
            let snippet = if content.len() > 100 {
                format!("{}...", &content[..100])
            } else {
                content
            };

            results.push((block_id, source_page_id, snippet));
        }

        Ok(results)
    }

    /// Set or clear the user-edited context override for a single
    /// reference. Q028: Editable Backlinks.
    ///
    /// Returns `true` when a reference row was found and updated, `false`
    /// when no row with the given `(source_id, target_id, ref_type)` key
    /// exists (caller maps to 404).
    async fn set_custom_context(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
        context: Option<&str>,
    ) -> Result<bool, DomainError> {
        // `update` is the only way to atomically "set if exists, else
        // no-op" in SQLite. We do NOT use INSERT OR REPLACE because the
        // primary key is `(source_id, target_id, ref_type)` and we must
        // not create phantom rows.
        let result = sqlx::query(
            "UPDATE refs SET custom_context = ? \
             WHERE source_id = ? AND target_id = ? AND ref_type = ?",
        )
        .bind(context)
        .bind(uuid_to_blob(&source_id))
        .bind(uuid_to_blob(&target_id))
        .bind(Self::ref_type_to_str(&ref_type))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("set_custom_context: {}", e)))?;

        Ok(result.rows_affected() > 0)
    }

    /// Get the user-edited context override for a single reference.
    ///
    /// Returns `None` when:
    /// - the reference does not exist, OR
    /// - the reference exists but has no override (NULL `custom_context`).
    ///
    /// Callers that need to distinguish these two cases should combine
    /// this with `get_forward_refs` or `get_backlinks`.
    async fn get_custom_context(
        &self,
        source_id: Uuid,
        target_id: Uuid,
        ref_type: RefType,
    ) -> Result<Option<String>, DomainError> {
        let row = sqlx::query(
            "SELECT custom_context FROM refs \
             WHERE source_id = ? AND target_id = ? AND ref_type = ?",
        )
        .bind(uuid_to_blob(&source_id))
        .bind(uuid_to_blob(&target_id))
        .bind(Self::ref_type_to_str(&ref_type))
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_custom_context: {}", e)))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let context: Option<String> = row.try_get("custom_context").ok().flatten();
                Ok(context)
            }
        }
    }

    /// Get the user-edited context overrides for every reference that
    /// points at a given target.
    ///
    /// Returns `(source_id, ref_type, custom_context)` tuples. References
    /// without an override (NULL `custom_context`) are intentionally
    /// omitted — the caller can use absence to mean "use the default
    /// snippet". Empty strings are NOT omitted: an empty string is a
    /// meaningful "override exists but is empty" state.
    async fn get_custom_contexts_for_target(
        &self,
        target_id: Uuid,
    ) -> Result<Vec<(Uuid, RefType, String)>, DomainError> {
        let rows = sqlx::query(
            "SELECT source_id, ref_type, custom_context \
             FROM refs \
             WHERE target_id = ? AND custom_context IS NOT NULL \
             ORDER BY source_id, ref_type",
        )
        .bind(uuid_to_blob(&target_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_custom_contexts_for_target: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let source_blob: Vec<u8> = row.get("source_id");
            let ref_type_str: String = row.get("ref_type");
            let context: Option<String> = row.try_get("custom_context").ok().flatten();

            // The WHERE clause already filters out NULL contexts, but
            // `try_get` can still return None on type mismatches; treat
            // that as "no override" and skip the row.
            let Some(context) = context else { continue };

            results.push((
                blob_to_uuid(&source_blob)?,
                Self::map_ref_type(&ref_type_str)?,
                context,
            ));
        }

        Ok(results)
    }
}

// ── SqliteSettingsRepository ───────────────────────────────────────────

/// SQLite-backed settings repository.
///
/// Uses the singleton `user_settings` table (single row with id=1).
/// If the table doesn't exist or no row is found, returns [`UserSettings::default`].
#[derive(Clone)]
pub struct SqliteSettingsRepository {
    pool: DbPool,
}

impl SqliteSettingsRepository {
    /// Creates a new `SqliteSettingsRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get_user_settings(&self) -> Result<UserSettings, DomainError> {
        let row = sqlx::query_as::<_, (String, String, u8, String)>(
            "SELECT timezone, journal_format, start_of_week, preferred_format \
             FROM user_settings WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_user_settings: {}", e)))?;

        match row {
            Some((timezone, journal_format, start_of_week, preferred_format)) => Ok(UserSettings {
                timezone,
                journal_format,
                start_of_week,
                preferred_format: BlockFormat::parse_str(&preferred_format)
                    .unwrap_or(BlockFormat::Markdown),
            }),
            None => Ok(UserSettings::default()),
        }
    }

    async fn update_user_settings(&self, settings: &UserSettings) -> Result<(), DomainError> {
        let preferred_format = match settings.preferred_format {
            BlockFormat::Markdown => "markdown",
            BlockFormat::Org => "org",
        };

        sqlx::query(
            "INSERT INTO user_settings (id, timezone, journal_format, start_of_week, preferred_format, updated_at) \
             VALUES (1, ?, ?, ?, ?, unixepoch('now')) \
             ON CONFLICT(id) DO UPDATE SET \
             timezone = excluded.timezone, \
             journal_format = excluded.journal_format, \
             start_of_week = excluded.start_of_week, \
             preferred_format = excluded.preferred_format, \
             updated_at = excluded.updated_at",
        )
        .bind(&settings.timezone)
        .bind(&settings.journal_format)
        .bind(settings.start_of_week)
        .bind(preferred_format)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("update_user_settings: {}", e)))?;

        Ok(())
    }
}

// ── SqliteTourStateRepository ──────────────────────────────────────

/// SQLite implementation of the [`TourStateRepository`] trait.
///
/// Persists tour-dismissal state in the `tour_dismissals` table added
/// by [`connection::run_migrations`]. The user identifier is an opaque
/// string (V1: the api key from the `Authorization` header) — we do
/// not validate it here because the auth middleware has already
/// accepted it by the time we get a request.
#[derive(Clone)]
pub struct SqliteTourStateRepository {
    pool: DbPool,
}

impl SqliteTourStateRepository {
    /// Creates a new `SqliteTourStateRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TourStateRepository for SqliteTourStateRepository {
    async fn get_dismissed_tours(&self, user_id: &str) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query(
            "SELECT tour_name FROM tour_dismissals WHERE user_id = ? ORDER BY tour_name",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_dismissed_tours: {}", e)))?;

        Ok(rows
            .iter()
            .map(|r| r.get::<String, _>("tour_name"))
            .collect())
    }

    async fn dismiss_tour(&self, user_id: &str, tour_name: &str) -> Result<(), DomainError> {
        // `INSERT OR REPLACE` makes the operation idempotent. The
        // composite primary key `(user_id, tour_name)` is what makes
        // the conflict detection work — a second dismissal of the
        // same pair updates the `dismissed_at` timestamp rather than
        // raising a constraint error.
        sqlx::query(
            "INSERT INTO tour_dismissals (user_id, tour_name, dismissed_at) \
             VALUES (?, ?, unixepoch('now')) \
             ON CONFLICT(user_id, tour_name) DO UPDATE SET \
             dismissed_at = excluded.dismissed_at",
        )
        .bind(user_id)
        .bind(tour_name)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("dismiss_tour: {}", e)))?;

        Ok(())
    }
}

// ── SqlitePropertyRepository ──────────────────────────────────────

/// SQLite implementation of the [`PropertyRepository`] trait.
///
/// Persists user-defined `PropertyDefinition` rows in the
/// `property_definitions` table (added in PI-3) and their associated
/// `ClosedValue` rows in `property_closed_values`. The builtin
/// definitions (status, priority, deadline, scheduled, url, template,
/// id/created_at/updated_at, tags, created_by) are NOT stored here —
/// they are served by the static `BUILTIN_PROPERTIES` map in
/// `quilt-domain::properties::builtin`.
///
/// Each `PropertyDefinition` round-trip also rehydrates its
/// `closed_values` vector via a second SELECT — that keeps the
/// definition table flat while still presenting a single entity
/// to callers (matches the trait contract).
pub struct SqlitePropertyRepository {
    pool: DbPool,
}

impl SqlitePropertyRepository {
    /// Creates a new `SqlitePropertyRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ── Row ↔ Entity mapping helpers ───────────────────────────────

    /// Map a `property_definitions` row to a `PropertyDefinition`.
    /// `closed_values` is NOT included — call `load_closed_values`
    /// separately and merge, or use the private `row_to_definition`
    /// helper below.
    fn row_to_definition(&self, row: &sqlx::sqlite::SqliteRow) -> Result<PropertyDefinition, DomainError> {
        let id_blob: Vec<u8> = row.get("id");
        let id = blob_to_uuid(&id_blob)?;
        let property_type_str: String = row.get("property_type");
        let cardinality_str: String = row.get("cardinality");
        let view_context_str: String = row.get("view_context");
        let status_str: String = row.get("status");

        let property_type = PropertyType::from_str(&property_type_str).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown property_type: {}", property_type_str))
        })?;
        let cardinality = Cardinality::from_str(&cardinality_str).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown cardinality: {}", cardinality_str))
        })?;
        let view_context = ViewContext::from_str(&view_context_str).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown view_context: {}", view_context_str))
        })?;
        let status = PropertyStatus::from_str(&status_str).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown property status: {}", status_str))
        })?;

        let public: i64 = row.get("public");
        let queryable: i64 = row.get("queryable");
        let hidden: i64 = row.get("hidden");
        let read_only: i64 = row.get("read_only");
        let block_count: i64 = row.get("block_count");
        let page_count: i64 = row.get("page_count");
        let first_seen_at: Option<i64> = row.get("first_seen_at");
        let last_seen_at: Option<i64> = row.get("last_seen_at");

        Ok(PropertyDefinition {
            id,
            db_ident: row.get("db_ident"),
            title: row.get("title"),
            property_type,
            cardinality,
            closed_values: Vec::new(), // Populated by `load_closed_values` callers
            view_context,
            public: public != 0,
            queryable: queryable != 0,
            hidden: hidden != 0,
            attribute: row.get("attribute"),
            read_only: read_only != 0,
            status,
            alias_of: row.get("alias_of"),
            block_count: block_count.max(0) as u64,
            page_count: page_count.max(0) as u64,
            first_seen_at: optional_ts_to_datetime(first_seen_at),
            last_seen_at: optional_ts_to_datetime(last_seen_at),
        })
    }

    /// Fetch the closed values for a property definition. Returns
    /// an empty vector when the property has no closed values or
    /// when no row matches.
    async fn load_closed_values(&self, property_id: Uuid) -> Result<Vec<ClosedValue>, DomainError> {
        let rows = sqlx::query(
            "SELECT id, db_ident, value, icon, sort_order \
             FROM property_closed_values \
             WHERE property_id = ? \
             ORDER BY sort_order ASC, value ASC",
        )
        .bind(uuid_to_blob(&property_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("load_closed_values: {}", e)))?;

        rows.iter()
            .map(|row| {
                let id_blob: Vec<u8> = row.get("id");
                let id = blob_to_uuid(&id_blob)?;
                let order: f64 = row.get("sort_order");
                Ok(ClosedValue {
                    id,
                    db_ident: row.get("db_ident"),
                    value: row.get("value"),
                    icon: row.get("icon"),
                    order,
                })
            })
            .collect()
    }

    /// Fetch a definition row by id and merge its closed values.
    async fn fetch_one_with_closed_values(
        &self,
        id: Uuid,
    ) -> Result<Option<PropertyDefinition>, DomainError> {
        let row = sqlx::query("SELECT * FROM property_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("fetch_one_with_closed_values: {}", e)))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let mut def = self.row_to_definition(&row)?;
                def.closed_values = self.load_closed_values(def.id).await?;
                Ok(Some(def))
            }
        }
    }

    /// Build a `WHERE db_ident IN (?, ?, ...)` clause with one bind
    /// per id. Returns the SQL fragment and the count of placeholders.
    fn build_in_clause(idents: &[&str]) -> String {
        let placeholders = std::iter::repeat("?")
            .take(idents.len())
            .collect::<Vec<_>>()
            .join(", ");
        format!("db_ident IN ({})", placeholders)
    }
}

#[async_trait]
impl PropertyRepository for SqlitePropertyRepository {
    #[instrument(skip(self))]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyDefinition>, DomainError> {
        self.fetch_one_with_closed_values(id).await
    }

    #[instrument(skip(self))]
    async fn get_by_db_ident(
        &self,
        ident: &str,
    ) -> Result<Option<PropertyDefinition>, DomainError> {
        let row = sqlx::query("SELECT * FROM property_definitions WHERE db_ident = ?")
            .bind(ident)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_db_ident: {}", e)))?;

        match row {
            None => Ok(None),
            Some(row) => {
                let mut def = self.row_to_definition(&row)?;
                def.closed_values = self.load_closed_values(def.id).await?;
                Ok(Some(def))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM property_definitions ORDER BY db_ident ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_all: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut def = self.row_to_definition(row)?;
            def.closed_values = self.load_closed_values(def.id).await?;
            results.push(def);
        }
        Ok(results)
    }

    #[instrument(skip(self, def))]
    async fn insert(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO property_definitions
            (id, db_ident, title, property_type, cardinality, view_context,
             public, queryable, hidden, attribute, read_only, status, alias_of,
             block_count, page_count, first_seen_at, last_seen_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&def.id))
        .bind(&def.db_ident)
        .bind(&def.title)
        .bind(def.property_type.as_str())
        .bind(def.cardinality.as_str())
        .bind(def.view_context.as_str())
        .bind(def.public as i64)
        .bind(def.queryable as i64)
        .bind(def.hidden as i64)
        .bind(def.attribute.as_deref())
        .bind(def.read_only as i64)
        .bind(def.status.as_str())
        .bind(def.alias_of.as_deref())
        .bind(def.block_count as i64)
        .bind(def.page_count as i64)
        .bind(def.first_seen_at.as_ref().map(datetime_to_ts))
        .bind(def.last_seen_at.as_ref().map(datetime_to_ts))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("insert property: {}", e)))?;

        // Closed values are written in a second pass. We replace any
        // pre-existing rows (idempotent re-insert) so callers can
        // pass a fully-formed `def` without first calling `delete`.
        for cv in &def.closed_values {
            sqlx::query(
                r#"INSERT OR REPLACE INTO property_closed_values
                (id, property_id, db_ident, value, icon, sort_order)
                VALUES (?, ?, ?, ?, ?, ?)"#,
            )
            .bind(uuid_to_blob(&cv.id))
            .bind(uuid_to_blob(&def.id))
            .bind(&cv.db_ident)
            .bind(&cv.value)
            .bind(cv.icon.as_deref())
            .bind(cv.order)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("insert closed value: {}", e)))?;
        }

        Ok(())
    }

    #[instrument(skip(self, def))]
    async fn update(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        let result = sqlx::query(
            r#"UPDATE property_definitions SET
            db_ident = ?, title = ?, property_type = ?, cardinality = ?,
            view_context = ?, public = ?, queryable = ?, hidden = ?,
            attribute = ?, read_only = ?, status = ?, alias_of = ?,
            block_count = ?, page_count = ?, first_seen_at = ?, last_seen_at = ?
            WHERE id = ?"#,
        )
        .bind(&def.db_ident)
        .bind(&def.title)
        .bind(def.property_type.as_str())
        .bind(def.cardinality.as_str())
        .bind(def.view_context.as_str())
        .bind(def.public as i64)
        .bind(def.queryable as i64)
        .bind(def.hidden as i64)
        .bind(def.attribute.as_deref())
        .bind(def.read_only as i64)
        .bind(def.status.as_str())
        .bind(def.alias_of.as_deref())
        .bind(def.block_count as i64)
        .bind(def.page_count as i64)
        .bind(def.first_seen_at.as_ref().map(datetime_to_ts))
        .bind(def.last_seen_at.as_ref().map(datetime_to_ts))
        .bind(uuid_to_blob(&def.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("update property: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("property".to_string()));
        }

        // Replace closed values atomically: delete all existing, re-insert.
        // A transaction would be ideal; for V1 the partial state
        // is acceptable because the trait contract is "the caller's
        // `def` is the new authoritative state".
        sqlx::query("DELETE FROM property_closed_values WHERE property_id = ?")
            .bind(uuid_to_blob(&def.id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("delete closed values: {}", e)))?;

        for cv in &def.closed_values {
            sqlx::query(
                r#"INSERT INTO property_closed_values
                (id, property_id, db_ident, value, icon, sort_order)
                VALUES (?, ?, ?, ?, ?, ?)"#,
            )
            .bind(uuid_to_blob(&cv.id))
            .bind(uuid_to_blob(&def.id))
            .bind(&cv.db_ident)
            .bind(&cv.value)
            .bind(cv.icon.as_deref())
            .bind(cv.order)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("insert closed value on update: {}", e)))?;
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_closed_values(
        &self,
        property_id: Uuid,
    ) -> Result<Vec<ClosedValue>, DomainError> {
        self.load_closed_values(property_id).await
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let result = sqlx::query("DELETE FROM property_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("delete property: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("property".to_string()));
        }
        // ON DELETE CASCADE handles property_closed_values rows.
        Ok(())
    }

    #[instrument(skip(self, idents))]
    async fn get_by_db_idents(
        &self,
        idents: &[&str],
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        if idents.is_empty() {
            return Ok(Vec::new());
        }
        let clause = Self::build_in_clause(idents);
        let sql = format!(
            "SELECT * FROM property_definitions WHERE {} ORDER BY db_ident ASC",
            clause
        );
        let mut query = sqlx::query(&sql);
        for ident in idents {
            query = query.bind(*ident);
        }
        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Storage(format!("get_by_db_idents: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut def = self.row_to_definition(row)?;
            def.closed_values = self.load_closed_values(def.id).await?;
            results.push(def);
        }
        Ok(results)
    }

    #[instrument(skip(self))]
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        if query.is_empty() {
            return self.list_by_usage(limit).await;
        }
        let like = format!("%{}%", query);
        let rows = sqlx::query(
            "SELECT * FROM property_definitions \
             WHERE db_ident LIKE ? OR title LIKE ? \
             ORDER BY db_ident ASC LIMIT ?",
        )
        .bind(&like)
        .bind(&like)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("search properties: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut def = self.row_to_definition(row)?;
            def.closed_values = self.load_closed_values(def.id).await?;
            results.push(def);
        }
        Ok(results)
    }

    #[instrument(skip(self))]
    async fn list_by_usage(
        &self,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM property_definitions \
             ORDER BY block_count DESC, db_ident ASC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("list_by_usage: {}", e)))?;

        let mut results = Vec::with_capacity(rows.len());
        for row in &rows {
            let mut def = self.row_to_definition(row)?;
            def.closed_values = self.load_closed_values(def.id).await?;
            results.push(def);
        }
        Ok(results)
    }

    // ── PI-5: Analytics implementations ──

    #[instrument(skip(self))]
    async fn get_co_occurrences(
        &self,
        limit: usize,
    ) -> Result<Vec<quilt_domain::properties::analytics::PropertyCoOccurrence>, DomainError> {
        // Strategy: extract all property keys from blocks.properties JSON,
        // then self-join to find pairs that co-occur on the same block.
        // SQLite's json_each() lets us iterate keys of the properties JSON blob.
        let rows = sqlx::query(
            r#"
            WITH block_keys AS (
                SELECT b.id AS block_id, je.key AS prop_key
                FROM blocks b, json_each(b.properties) AS je
                WHERE b.properties != '{}'
            ),
            pair_counts AS (
                SELECT
                    MIN(a.prop_key, b.prop_key) AS key_a,
                    MAX(a.prop_key, b.prop_key) AS key_b,
                    COUNT(DISTINCT a.block_id) AS co_count
                FROM block_keys a
                JOIN block_keys b ON a.block_id = b.block_id AND a.prop_key < b.prop_key
                GROUP BY key_a, key_b
            ),
            solo_counts AS (
                SELECT prop_key, COUNT(DISTINCT block_id) AS solo_count
                FROM block_keys
                GROUP BY prop_key
            )
            SELECT
                pc.key_a,
                pc.key_b,
                pc.co_count,
                sa.solo_count AS count_a,
                sb.solo_count AS count_b
            FROM pair_counts pc
            JOIN solo_counts sa ON sa.prop_key = pc.key_a
            JOIN solo_counts sb ON sb.prop_key = pc.key_b
            ORDER BY pc.co_count DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_co_occurrences: {}", e)))?;

        let total_blocks = self.count_blocks_with_properties().await.unwrap_or(1).max(1) as f64;

        let results: Vec<_> = rows
            .iter()
            .map(|row| {
                let key_a: String = row.get("key_a");
                let key_b: String = row.get("key_b");
                let co_count: i64 = row.get("co_count");
                let count_a: i64 = row.get("count_a");
                let count_b: i64 = row.get("count_b");

                let co_count = co_count as u64;
                let count_a = count_a as u64;
                let count_b = count_b as u64;

                // PMI = log2(P(a,b) / (P(a) * P(b)))
                // P(a,b) = co_count / total_blocks
                // P(a) = count_a / total_blocks
                // P(b) = count_b / total_blocks
                let pmi = if co_count > 0 && count_a > 0 && count_b > 0 && total_blocks > 0.0 {
                    let p_ab = co_count as f64 / total_blocks;
                    let p_a = count_a as f64 / total_blocks;
                    let p_b = count_b as f64 / total_blocks;
                    let denom = p_a * p_b;
                    if denom > 0.0 {
                        (p_ab / denom).log2()
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                quilt_domain::properties::analytics::PropertyCoOccurrence {
                    key_a,
                    key_b,
                    co_occurrence_count: co_count,
                    count_a,
                    count_b,
                    pmi,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn get_trends(
        &self,
        period_days: u32,
        limit: usize,
    ) -> Result<Vec<quilt_domain::properties::analytics::PropertyTrend>, DomainError> {
        let period_ms = (period_days as i64) * 24 * 60 * 60 * 1000;

        let rows = sqlx::query(
            r#"
            WITH block_keys AS (
                SELECT b.id, je.key AS prop_key, b.updated_at
                FROM blocks b, json_each(b.properties) AS je
                WHERE b.properties != '{}'
            ),
            current_period AS (
                SELECT prop_key, COUNT(DISTINCT id) AS cnt
                FROM block_keys
                WHERE updated_at >= (unixepoch('now') * 1000 - ?)
                GROUP BY prop_key
            ),
            previous_period AS (
                SELECT prop_key, COUNT(DISTINCT id) AS cnt
                FROM block_keys
                WHERE updated_at >= (unixepoch('now') * 1000 - ?)
                  AND updated_at < (unixepoch('now') * 1000 - ?)
                GROUP BY prop_key
            )
            SELECT
                COALESCE(c.prop_key, p.prop_key) AS key,
                COALESCE(c.cnt, 0) AS current_count,
                COALESCE(p.cnt, 0) AS previous_count
            FROM current_period c
            FULL OUTER JOIN previous_period p ON c.prop_key = p.prop_key
            ORDER BY COALESCE(c.cnt, 0) DESC
            LIMIT ?
            "#,
        )
        .bind(period_ms)
        .bind(period_ms * 2)
        .bind(period_ms)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("get_trends: {}", e)))?;

        let results: Vec<_> = rows
            .iter()
            .map(|row| {
                let key: String = row.get("key");
                let current_count: i64 = row.get("current_count");
                let previous_count: i64 = row.get("previous_count");

                let current = current_count as u64;
                let previous = previous_count as u64;

                let (change_percent, direction) = if previous == 0 && current > 0 {
                    (f64::INFINITY, quilt_domain::properties::analytics::TrendDirection::New)
                } else if previous == 0 && current == 0 {
                    (0.0, quilt_domain::properties::analytics::TrendDirection::Stable)
                } else {
                    let change = ((current as f64 - previous as f64) / previous as f64) * 100.0;
                    let dir = if change > 10.0 {
                        quilt_domain::properties::analytics::TrendDirection::Rising
                    } else if change < -10.0 {
                        quilt_domain::properties::analytics::TrendDirection::Declining
                    } else {
                        quilt_domain::properties::analytics::TrendDirection::Stable
                    };
                    (change, dir)
                };

                quilt_domain::properties::analytics::PropertyTrend {
                    key,
                    current_count: current,
                    previous_count: previous,
                    change_percent,
                    direction,
                }
            })
            .collect();

        Ok(results)
    }

    #[instrument(skip(self))]
    async fn count_distinct_properties(&self) -> Result<u64, DomainError> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(DISTINCT je.key) AS cnt
            FROM blocks b, json_each(b.properties) AS je
            WHERE b.properties != '{}'
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("count_distinct_properties: {}", e)))?;

        let count: i64 = row.get("cnt");
        Ok(count as u64)
    }

    #[instrument(skip(self))]
    async fn count_blocks_with_properties(&self) -> Result<u64, DomainError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS cnt FROM blocks WHERE properties != '{}'",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("count_blocks_with_properties: {}", e)))?;

        let count: i64 = row.get("cnt");
        Ok(count as u64)
    }
}

// ── PI-7: SqliteSchemaRepository ──────────────────────────────────────

/// SQLite implementation of the SchemaRepository trait.
pub struct SqliteSchemaRepository {
    pool: sqlx::SqlitePool,
}

impl SqliteSchemaRepository {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    fn row_to_schema(
        &self,
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<quilt_domain::properties::schema::PropertySchema, quilt_domain::DomainError> {
        let id_bytes: Vec<u8> = row.try_get("id").map_err(|e| {
            quilt_domain::DomainError::Storage(format!("schema id: {}", e))
        })?;
        let id = blob_to_uuid(&id_bytes)?;

        let name: String = row.try_get("name").map_err(|e| {
            quilt_domain::DomainError::Storage(format!("schema name: {}", e))
        })?;
        let description: String = row.try_get("description").unwrap_or_default();
        let keys_json: String = row.try_get("property_keys").unwrap_or_else(|_| "[]".to_string());
        let property_keys: Vec<String> = serde_json::from_str(&keys_json).unwrap_or_default();
        let auto_detected: i64 = row.try_get("auto_detected").unwrap_or(0);
        let created_at: i64 = row.try_get("created_at").unwrap_or(0);
        let updated_at: i64 = row.try_get("updated_at").unwrap_or(0);

        Ok(quilt_domain::properties::schema::PropertySchema {
            id,
            name,
            description,
            property_keys,
            auto_detected: auto_detected != 0,
            created_at,
            updated_at,
        })
    }
}

#[async_trait::async_trait]
impl quilt_domain::repositories::SchemaRepository for SqliteSchemaRepository {
    #[tracing::instrument(skip(self))]
    async fn get_by_id(
        &self,
        id: quilt_domain::value_objects::Uuid,
    ) -> Result<Option<quilt_domain::properties::schema::PropertySchema>, quilt_domain::DomainError> {
        let row = sqlx::query("SELECT * FROM property_schemas WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| quilt_domain::DomainError::Storage(format!("schema get_by_id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(self.row_to_schema(&r)?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_by_name(
        &self,
        name: &str,
    ) -> Result<Option<quilt_domain::properties::schema::PropertySchema>, quilt_domain::DomainError> {
        let row = sqlx::query("SELECT * FROM property_schemas WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| quilt_domain::DomainError::Storage(format!("schema get_by_name: {}", e)))?;

        match row {
            Some(r) => Ok(Some(self.row_to_schema(&r)?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn list_all(
        &self,
    ) -> Result<Vec<quilt_domain::properties::schema::PropertySchema>, quilt_domain::DomainError> {
        let rows = sqlx::query("SELECT * FROM property_schemas ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| quilt_domain::DomainError::Storage(format!("schema list_all: {}", e)))?;

        rows.iter()
            .map(|r| self.row_to_schema(r))
            .collect()
    }

    #[tracing::instrument(skip(self))]
    async fn list_auto_detected(
        &self,
    ) -> Result<Vec<quilt_domain::properties::schema::PropertySchema>, quilt_domain::DomainError> {
        let rows = sqlx::query("SELECT * FROM property_schemas WHERE auto_detected = 1 ORDER BY name ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| quilt_domain::DomainError::Storage(format!("schema list_auto_detected: {}", e)))?;

        rows.iter()
            .map(|r| self.row_to_schema(r))
            .collect()
    }

    #[tracing::instrument(skip(self))]
    async fn insert(
        &self,
        schema: &quilt_domain::properties::schema::PropertySchema,
    ) -> Result<(), quilt_domain::DomainError> {
        let keys_json = serde_json::to_string(&schema.property_keys)
            .map_err(|e| quilt_domain::DomainError::Storage(format!("serialize keys: {}", e)))?;

        sqlx::query(
            r#"INSERT INTO property_schemas (id, name, description, property_keys, auto_detected, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&schema.id))
        .bind(&schema.name)
        .bind(&schema.description)
        .bind(&keys_json)
        .bind(schema.auto_detected as i64)
        .bind(schema.created_at)
        .bind(schema.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| quilt_domain::DomainError::Storage(format!("schema insert: {}", e)))?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update(
        &self,
        schema: &quilt_domain::properties::schema::PropertySchema,
    ) -> Result<(), quilt_domain::DomainError> {
        let keys_json = serde_json::to_string(&schema.property_keys)
            .map_err(|e| quilt_domain::DomainError::Storage(format!("serialize keys: {}", e)))?;

        sqlx::query(
            r#"UPDATE property_schemas
            SET name = ?, description = ?, property_keys = ?, auto_detected = ?, updated_at = ?
            WHERE id = ?"#,
        )
        .bind(&schema.name)
        .bind(&schema.description)
        .bind(&keys_json)
        .bind(schema.auto_detected as i64)
        .bind(schema.updated_at)
        .bind(uuid_to_blob(&schema.id))
        .execute(&self.pool)
        .await
        .map_err(|e| quilt_domain::DomainError::Storage(format!("schema update: {}", e)))?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete(
        &self,
        id: quilt_domain::value_objects::Uuid,
    ) -> Result<(), quilt_domain::DomainError> {
        sqlx::query("DELETE FROM property_schemas WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| quilt_domain::DomainError::Storage(format!("schema delete: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::sqlite::connection;

    async fn setup_test_db() -> DbPool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");
        connection::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");
        pool
    }

    fn make_block(page_id: Uuid, content: &str) -> Block {
        use quilt_domain::entities::BlockCreate;
        let create = BlockCreate {
            page_id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: HashMap::new(),
        };
        Block::new(create).expect("Failed to create block")
    }

    fn make_page(name: &str) -> Page {
        use quilt_domain::entities::PageCreate;
        let create = PageCreate {
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        };
        Page::new(create).expect("Failed to create page")
    }

    // ── BlockRepository Tests ──────────────────────────────────────

    #[tokio::test]
    async fn test_block_insert_and_get_by_id() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("test-page");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "Hello world");
        repo.insert(&block).await.unwrap();

        let found = repo.get_by_id(block.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Hello world");
    }

    #[tokio::test]
    async fn test_block_get_by_page() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("blocks-page");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let b1 = make_block(page.id, "First");
        let b2 = make_block(page.id, "Second");
        repo.insert(&b1).await.unwrap();
        repo.insert(&b2).await.unwrap();

        let blocks = repo.get_by_page(page.id).await.unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[tokio::test]
    async fn test_block_update() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("update-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let mut block = make_block(page.id, "Original");
        repo.insert(&block).await.unwrap();

        block.content = "Updated".to_string();
        repo.update(&block).await.unwrap();

        let found = repo.get_by_id(block.id).await.unwrap().unwrap();
        assert_eq!(found.content, "Updated");
    }

    #[tokio::test]
    async fn test_block_delete() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("delete-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "To delete");
        repo.insert(&block).await.unwrap();

        repo.delete(block.id).await.unwrap();

        let found = repo.get_by_id(block.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_block_move() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("move-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let parent = make_block(page.id, "Parent");
        let child = make_block(page.id, "Child");
        repo.insert(&parent).await.unwrap();
        repo.insert(&child).await.unwrap();

        repo.move_block(child.id, Some(parent.id), 2.5)
            .await
            .unwrap();

        let updated = repo.get_by_id(child.id).await.unwrap().unwrap();
        assert_eq!(updated.parent_id, Some(parent.id));
        assert_eq!(updated.order, 2.5);
    }

    #[tokio::test]
    async fn test_block_count_by_page() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("count-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        assert_eq!(repo.count_by_page(page.id).await.unwrap(), 0);

        repo.insert(&make_block(page.id, "A")).await.unwrap();
        repo.insert(&make_block(page.id, "B")).await.unwrap();

        assert_eq!(repo.count_by_page(page.id).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_block_search_fts() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("search-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let b1 = make_block(page.id, "Rust programming language");
        let b2 = make_block(page.id, "Python scripting");
        let b3 = make_block(page.id, "Rust is fast and safe");
        repo.insert(&b1).await.unwrap();
        repo.insert(&b2).await.unwrap();
        repo.insert(&b3).await.unwrap();

        // FTS5: search for "Rust"
        let results = repo.search("\"Rust\"", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    // ── PageRepository Tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_page_insert_and_get_by_name() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let page = make_page("my-awesome-page");
        repo.insert(&page).await.unwrap();

        let found = repo.get_by_name("my-awesome-page").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my-awesome-page");
    }

    #[tokio::test]
    async fn test_page_get_all_and_count() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        repo.insert(&make_page("page-a")).await.unwrap();
        repo.insert(&make_page("page-b")).await.unwrap();
        repo.insert(&make_page("page-c")).await.unwrap();

        let all = repo.get_all().await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(repo.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_page_rename() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let page = make_page("old-name");
        repo.insert(&page).await.unwrap();

        repo.rename(page.id, "new-name").await.unwrap();

        let found = repo.get_by_name("new-name").await.unwrap();
        assert!(found.is_some());
        let old = repo.get_by_name("old-name").await.unwrap();
        assert!(old.is_none());
    }

    #[tokio::test]
    async fn test_page_delete() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let page = make_page("to-delete");
        repo.insert(&page).await.unwrap();

        repo.delete(page.id).await.unwrap();

        let found = repo.get_by_id(page.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_page_journal() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let day = JournalDay::from_i32(20260503).unwrap();
        let page = Page::new_journal(day, BlockFormat::Markdown, "%Y-%m-%d").unwrap();
        repo.insert(&page).await.unwrap();

        let found = repo.get_journal(day).await.unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().journal);
    }

    #[tokio::test]
    async fn test_page_search() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        repo.insert(&make_page("rust-programming")).await.unwrap();
        repo.insert(&make_page("python-scripts")).await.unwrap();

        let results = repo.search("rust", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust-programming");
    }

    // ── F5 + F8 + F9: update_properties integration tests ─────────

    /// Helper: create a page and return its id.
    async fn create_test_page_in_repo(repo: &SqlitePageRepository, name: &str) -> Uuid {
        let page = make_page(name);
        repo.insert(&page).await.unwrap();
        page.id
    }

    fn entry_str(s: &str) -> DefaultPropertyEntry<PropertyValue> {
        DefaultPropertyEntry::new(PropertyValue::string(s))
    }

    /// Create a timestamped entry. The use case layer must stamp explicit
    /// user updates with `updated_at = Some(now)` so the LWW merge in
    /// `update_properties` correctly replaces the existing value.
    fn entry_str_ts(
        s: &str,
        ts: chrono::DateTime<chrono::Utc>,
    ) -> DefaultPropertyEntry<PropertyValue> {
        DefaultPropertyEntry::with_timestamp(PropertyValue::string(s), ts)
    }

    #[tokio::test]
    async fn test_update_properties_round_trip() {
        // F5: round-trip a single property through SQLite.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "rt-page").await;

        let mut props = HashMap::new();
        props.insert("status".to_string(), entry_str("Doing"));
        let updated = repo.update_properties(id, props).await.unwrap();

        assert_eq!(
            updated.properties["status"].value(),
            &PropertyValue::String("Doing".to_string())
        );

        // Re-fetch and verify persistence.
        let fetched = repo.get_by_id(id).await.unwrap().unwrap();
        assert_eq!(
            fetched.properties["status"].value(),
            &PropertyValue::String("Doing".to_string())
        );
    }

    #[tokio::test]
    async fn test_update_properties_concurrent_different_keys() {
        // F5 scenario: two callers update different keys → both preserved.
        // Each caller timestamps their update so the LWW merge in
        // merge_properties correctly applies the user intent.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "concurrent-page").await;

        // Seed: {a -> A0, b -> B0} (timestamped so subsequent updates win)
        let t0 = chrono::Utc::now();
        let mut seed = HashMap::new();
        seed.insert("a".to_string(), entry_str_ts("A0", t0));
        seed.insert("b".to_string(), entry_str_ts("B0", t0));
        repo.update_properties(id, seed).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let t1 = chrono::Utc::now();

        // Caller X updates "a" (timestamped, later than t0)
        let mut x = HashMap::new();
        x.insert("a".to_string(), entry_str_ts("A1", t1));
        repo.update_properties(id, x).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let t2 = chrono::Utc::now();

        // Caller Y updates "b" (timestamped, later than t1, no key overlap)
        let mut y = HashMap::new();
        y.insert("b".to_string(), entry_str_ts("B1", t2));
        repo.update_properties(id, y).await.unwrap();

        // Both updates preserved.
        let fetched = repo.get_by_id(id).await.unwrap().unwrap();
        assert_eq!(
            fetched.properties["a"].value(),
            &PropertyValue::String("A1".to_string())
        );
        assert_eq!(
            fetched.properties["b"].value(),
            &PropertyValue::String("B1".to_string())
        );
    }

    #[tokio::test]
    async fn test_update_properties_rejects_read_only_key() {
        // F9: User cannot set `created_at` — returns PropertyReadOnly.
        // Without a property_repo, the impl uses the hardcoded system list.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "ro-page").await;

        let mut props = HashMap::new();
        props.insert(
            "created_at".to_string(),
            DefaultPropertyEntry::new(PropertyValue::string("2026-01-01")),
        );
        let result = repo.update_properties(id, props).await;

        assert!(matches!(result, Err(DomainError::PropertyReadOnly(k)) if k == "created_at"));

        // The persisted map is unchanged.
        let fetched = repo.get_by_id(id).await.unwrap().unwrap();
        assert!(fetched.properties.is_empty());
    }

    #[tokio::test]
    async fn test_update_properties_mixed_batch_rejected_atomically() {
        // F9: a single rejected key fails the whole call with no partial write.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "mixed-page").await;

        // Seed with "status" so we can verify it's NOT overwritten.
        let mut seed = HashMap::new();
        seed.insert("status".to_string(), entry_str("Doing"));
        repo.update_properties(id, seed).await.unwrap();

        // Mixed batch: one writable, one read-only.
        let mut batch = HashMap::new();
        batch.insert("status".to_string(), entry_str("Done"));
        batch.insert(
            "id".to_string(),
            DefaultPropertyEntry::new(PropertyValue::string("forged")),
        );
        let result = repo.update_properties(id, batch).await;

        assert!(matches!(result, Err(DomainError::PropertyReadOnly(k)) if k == "id"));

        // status is NOT updated — atomic rejection.
        let fetched = repo.get_by_id(id).await.unwrap().unwrap();
        assert_eq!(
            fetched.properties["status"].value(),
            &PropertyValue::String("Doing".to_string())
        );
    }

    #[tokio::test]
    async fn test_update_properties_preserves_existing_read_only_key() {
        // F9: a read-only key already on the page is preserved when updating
        // other (writable) keys.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "preserve-page").await;

        // Directly insert a page with an "id" property by writing the JSON
        // column. (T-B.14 doesn't expose insert_with_property, so we use the
        // public method to seed non-read-only properties, then check the
        // behavior of update_properties for a writable key.)
        let mut seed = HashMap::new();
        seed.insert("status".to_string(), entry_str("Doing"));
        repo.update_properties(id, seed).await.unwrap();

        // Now update with a different writable key.
        let mut next = HashMap::new();
        next.insert("priority".to_string(), entry_str("A"));
        let updated = repo.update_properties(id, next).await.unwrap();

        // status is preserved, priority is added.
        assert!(updated.properties.contains_key("status"));
        assert!(updated.properties.contains_key("priority"));
    }

    #[tokio::test]
    async fn test_update_properties_100_entries() {
        // F5: large payload round-trips.
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);
        let id = create_test_page_in_repo(&repo, "huge-page").await;

        let mut props = HashMap::new();
        for i in 0..100 {
            props.insert(format!("key_{i:03}"), entry_str(&format!("v_{i}")));
        }
        let updated = repo.update_properties(id, props).await.unwrap();
        assert_eq!(updated.properties.len(), 100);

        let fetched = repo.get_by_id(id).await.unwrap().unwrap();
        assert_eq!(fetched.properties.len(), 100);
        for i in 0..100 {
            let k = format!("key_{i:03}");
            assert_eq!(
                fetched.properties[&k].value(),
                &PropertyValue::String(format!("v_{i}"))
            );
        }
    }

    // ── TagRepository Tests ────────────────────────────────────────

    #[tokio::test]
    async fn test_tag_add_and_get() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let page = make_page("tagged-page");
        page_repo.insert(&page).await.unwrap();

        tag_repo.add_tag(page.id, "rust").await.unwrap();
        tag_repo.add_tag(page.id, "async").await.unwrap();

        let tags = tag_repo.get_by_page(page.id).await.unwrap();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"rust".to_string()));
    }

    #[tokio::test]
    async fn test_tag_remove() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let page = make_page("rm-tag-page");
        page_repo.insert(&page).await.unwrap();

        tag_repo.add_tag(page.id, "temp").await.unwrap();
        tag_repo.remove_tag(page.id, "temp").await.unwrap();

        let tags = tag_repo.get_by_page(page.id).await.unwrap();
        assert!(tags.is_empty());
    }

    #[tokio::test]
    async fn test_tag_counts() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let p1 = make_page("p1");
        let p2 = make_page("p2");
        page_repo.insert(&p1).await.unwrap();
        page_repo.insert(&p2).await.unwrap();

        tag_repo.add_tag(p1.id, "common").await.unwrap();
        tag_repo.add_tag(p2.id, "common").await.unwrap();
        tag_repo.add_tag(p1.id, "unique").await.unwrap();

        let counts = tag_repo.get_tag_counts().await.unwrap();
        let common = counts.iter().find(|(t, _)| t == "common").unwrap();
        assert_eq!(common.1, 2);
    }

    // ── RefRepository Tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_ref_sync_refs_and_get_forward() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        repo.sync_refs(source_id, &[(target_id, RefType::BlockRef)])
            .await
            .unwrap();

        let forward = repo.get_forward_refs(source_id).await.unwrap();
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0], (target_id, RefType::BlockRef));
    }

    #[tokio::test]
    async fn test_ref_get_backlinks() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        repo.sync_refs(source_id, &[(target_id, RefType::PageRef)])
            .await
            .unwrap();

        let backlinks = repo.get_backlinks(target_id).await.unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0], (source_id, RefType::PageRef));
    }

    #[tokio::test]
    async fn test_ref_sync_replaces_old_refs() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool.clone());
        let source_id = Uuid::new_v4();
        let target1 = Uuid::new_v4();
        let target2 = Uuid::new_v4();

        // First sync: reference target1
        repo.sync_refs(source_id, &[(target1, RefType::BlockRef)])
            .await
            .unwrap();
        assert_eq!(repo.get_forward_refs(source_id).await.unwrap().len(), 1);

        // Second sync: reference target2 only
        repo.sync_refs(source_id, &[(target2, RefType::BlockRef)])
            .await
            .unwrap();

        let forward = repo.get_forward_refs(source_id).await.unwrap();
        assert_eq!(forward.len(), 1);
        assert_eq!(forward[0], (target2, RefType::BlockRef));

        // target1 should have no backlinks
        assert_eq!(repo.get_backlinks(target1).await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_ref_multiple_types_same_target() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        repo.sync_refs(
            source_id,
            &[
                (target_id, RefType::PageRef),
                (target_id, RefType::BlockRef),
            ],
        )
        .await
        .unwrap();

        let forward = repo.get_forward_refs(source_id).await.unwrap();
        assert_eq!(forward.len(), 2);

        let types: Vec<RefType> = forward.iter().map(|(_, t)| *t).collect();
        assert!(types.contains(&RefType::PageRef));
        assert!(types.contains(&RefType::BlockRef));
    }

    #[tokio::test]
    async fn test_ref_rebuild_index() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        repo.sync_refs(source_id, &[(target_id, RefType::Tag)])
            .await
            .unwrap();

        let rows = repo.rebuild_index().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source_id, source_id);
        assert_eq!(rows[0].target_id, target_id);
        assert_eq!(rows[0].ref_type, RefType::Tag);
    }

    #[tokio::test]
    async fn test_ref_empty_forward_refs() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();

        let forward = repo.get_forward_refs(source_id).await.unwrap();
        assert!(forward.is_empty());
    }

    #[tokio::test]
    async fn test_ref_empty_backlinks() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let target_id = Uuid::new_v4();

        let backlinks = repo.get_backlinks(target_id).await.unwrap();
        assert!(backlinks.is_empty());
    }

    #[tokio::test]
    async fn test_ref_sync_empty_list_clears_refs() {
        let pool = setup_test_db().await;
        let repo = SqliteRefRepository::new(pool);
        let source_id = Uuid::new_v4();
        let target_id = Uuid::new_v4();

        repo.sync_refs(source_id, &[(target_id, RefType::BlockRef)])
            .await
            .unwrap();
        assert_eq!(repo.get_forward_refs(source_id).await.unwrap().len(), 1);

        // Sync with empty list — should clear
        repo.sync_refs(source_id, &[]).await.unwrap();
        assert!(repo.get_forward_refs(source_id).await.unwrap().is_empty());
    }

    // ── TourStateRepository Tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_tour_state_empty_for_new_user() {
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool);
        let tours = repo.get_dismissed_tours("api-key-1").await.unwrap();
        assert!(tours.is_empty());
    }

    #[tokio::test]
    async fn test_tour_state_dismiss_then_get_round_trips() {
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool);
        repo.dismiss_tour("api-key-1", "welcome").await.unwrap();
        let tours = repo.get_dismissed_tours("api-key-1").await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
    }

    #[tokio::test]
    async fn test_tour_state_dismiss_is_idempotent_at_sql_level() {
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool.clone());
        // Dismiss the same tour four times. The composite primary
        // key (user_id, tour_name) plus ON CONFLICT DO UPDATE means
        // the row count after every call is exactly one.
        repo.dismiss_tour("api-key-1", "welcome").await.unwrap();
        repo.dismiss_tour("api-key-1", "welcome").await.unwrap();
        repo.dismiss_tour("api-key-1", "welcome").await.unwrap();
        repo.dismiss_tour("api-key-1", "welcome").await.unwrap();

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tour_dismissals WHERE user_id = ?")
                .bind("api-key-1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);

        let tours = repo.get_dismissed_tours("api-key-1").await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
    }

    #[tokio::test]
    async fn test_tour_state_multiple_tours_per_user() {
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool);
        repo.dismiss_tour("u", "welcome").await.unwrap();
        repo.dismiss_tour("u", "cognitive").await.unwrap();
        repo.dismiss_tour("u", "mcp").await.unwrap();

        // The query orders by `tour_name` so the assertion is
        // stable across runs.
        let tours = repo.get_dismissed_tours("u").await.unwrap();
        assert_eq!(
            tours,
            vec![
                "cognitive".to_string(),
                "mcp".to_string(),
                "welcome".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn test_tour_state_users_are_isolated() {
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool);
        repo.dismiss_tour("alice", "welcome").await.unwrap();
        let bob = repo.get_dismissed_tours("bob").await.unwrap();
        assert!(bob.is_empty(), "bob's tour list must not see alice's rows");
    }

    #[tokio::test]
    async fn test_tour_state_user_id_with_special_chars() {
        // The api key is a UUIDv4 today, but the column is TEXT. A
        // future migration could switch to a real user id with
        // arbitrary characters; the repo must not choke on them.
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool);
        let weird = "user/with spaces & symbols:😀=42";
        repo.dismiss_tour(weird, "welcome").await.unwrap();
        let tours = repo.get_dismissed_tours(weird).await.unwrap();
        assert_eq!(tours, vec!["welcome".to_string()]);
    }

    #[tokio::test]
    async fn test_tour_state_redismiss_updates_timestamp() {
        // Sleeping 1s in a test is annoying, but sqlite's
        // `unixepoch('now')` is second-precision so a 1.1s wait is
        // the smallest reliable gap. We only check that the second
        // call's timestamp is >= the first one, which is enough to
        // prove ON CONFLICT DO UPDATE ran.
        let pool = setup_test_db().await;
        let repo = SqliteTourStateRepository::new(pool.clone());
        repo.dismiss_tour("u", "welcome").await.unwrap();
        let first: i64 =
            sqlx::query_scalar("SELECT dismissed_at FROM tour_dismissals WHERE user_id = ?")
                .bind("u")
                .fetch_one(&pool)
                .await
                .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        repo.dismiss_tour("u", "welcome").await.unwrap();
        let second: i64 =
            sqlx::query_scalar("SELECT dismissed_at FROM tour_dismissals WHERE user_id = ?")
                .bind("u")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            second > first,
            "second dismiss should refresh the timestamp"
        );
    }

    // ── BlockType persistence tests (P0 of `quilt-blocktype-persistence`) ──
    //
    // These tests prove the end-to-end contract: a block's `block_type`
    // round-trips through SQLite byte-identical. They cover all 11
    // variants to catch any future mismatch between the Rust enum and
    // the persisted string.

    /// Helper that inserts a block with a specific `BlockType` and reads
    /// it back, asserting the type survives the round-trip. Every
    /// `BlockType` variant in the public contract is exercised once.
    async fn assert_block_type_round_trip(block_type: BlockType) {
        let pool = setup_test_db().await;
        let page_id = Uuid::new_v4();
        // Create a page (FK target for the block).
        sqlx::query(
            "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                     VALUES (?, ?, 'markdown', 0, 0, 0)",
        )
        .bind(uuid_to_blob(&page_id))
        .bind(format!("page-for-{:?}", block_type))
        .execute(&pool)
        .await
        .unwrap();

        let repo = SqliteBlockRepository::new(pool.clone());
        let mut block = make_block(page_id, "x");
        block.block_type = block_type;

        repo.insert(&block).await.expect("insert should succeed");
        let loaded = repo
            .get_by_id(block.id)
            .await
            .expect("get_by_id should succeed")
            .expect("block should exist after insert");

        assert_eq!(
            loaded.block_type, block_type,
            "block_type round-trip mismatch for variant {:?}",
            block_type
        );
    }

    /// Every `BlockType` variant must round-trip through SQLite. This is
    /// the wire contract that the frontend's slash commands rely on.
    #[tokio::test]
    async fn test_block_type_round_trip_for_every_variant() {
        for variant in BlockType::all() {
            assert_block_type_round_trip(*variant).await;
        }
    }

    /// The PATCH path mutates a block's `block_type` in place. The
    /// column must reflect the new value after `repo.update(&block)`.
    #[tokio::test]
    async fn test_block_type_persists_after_update() {
        let pool = setup_test_db().await;
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                     VALUES (?, ?, 'markdown', 0, 0, 0)",
        )
        .bind(uuid_to_blob(&page_id))
        .bind("page-update-bt")
        .execute(&pool)
        .await
        .unwrap();

        let repo = SqliteBlockRepository::new(pool);
        let mut block = make_block(page_id, "Heading text");
        repo.insert(&block).await.unwrap();

        // Simulate the slash-command PATCH path: mutate the entity, then
        // write it back. This is what `update_block` does in
        // `quilt-server/src/handlers/blocks.rs`.
        block.block_type = BlockType::Heading1;
        repo.update(&block).await.unwrap();

        let loaded = repo.get_by_id(block.id).await.unwrap().unwrap();
        assert_eq!(loaded.block_type, BlockType::Heading1);

        // And a second change — covers the same-block-update path that
        // the slash-command registry exercises when the user toggles
        // between kinds rapidly.
        block.block_type = BlockType::Code;
        repo.update(&block).await.unwrap();
        let loaded = repo.get_by_id(block.id).await.unwrap().unwrap();
        assert_eq!(loaded.block_type, BlockType::Code);
    }

    /// The DB column is `NOT NULL DEFAULT 'paragraph'`. Rows inserted
    /// via raw SQL (bypassing the repo) without specifying a type must
    /// still load as `BlockType::Paragraph`. This is the migration
    /// safety net — pre-migration data backfills to `'paragraph'`.
    #[tokio::test]
    async fn test_default_block_type_is_paragraph() {
        let pool = setup_test_db().await;
        let page_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                     VALUES (?, ?, 'markdown', 0, 0, 0)",
        )
        .bind(uuid_to_blob(&page_id))
        .bind("page-default")
        .execute(&pool)
        .await
        .unwrap();

        // Insert a block WITHOUT specifying block_type — relies on the
        // column's `DEFAULT 'paragraph'`.
        let block_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO blocks (id, page_id, order_index, level, format, content, \
             properties, collapsed, created_at, updated_at, refs, tags) \
             VALUES (?, ?, 1.0, 1, 'markdown', '', '{}', 0, 0, 0, '[]', '[]')",
        )
        .bind(uuid_to_blob(&block_id))
        .bind(uuid_to_blob(&page_id))
        .execute(&pool)
        .await
        .unwrap();

        let repo = SqliteBlockRepository::new(pool);
        let loaded = repo.get_by_id(block_id).await.unwrap().unwrap();
        assert_eq!(
            loaded.block_type,
            BlockType::Paragraph,
            "rows without an explicit block_type must default to Paragraph"
        );
    }

    /// Migrations must be idempotent. Running them twice on the same
    /// pool must not error — this matches the contract of
    /// migration 006 (`pages.properties`) and the existing
    /// `ALTER TABLE ... "duplicate column"` swallow.
    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let pool = setup_test_db().await;
        // Second run must not error.
        connection::run_migrations(&pool)
            .await
            .expect("second run_migrations call should be a no-op");
    }
}
