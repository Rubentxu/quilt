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

use crate::database::sqlite::connection::DbPool;
use quilt_domain::entities::{Block, Page, UserSettings};
use quilt_domain::errors::DomainError;
use quilt_domain::properties::entry::{DefaultPropertyEntry, HasValue};
use quilt_domain::references::RefType;
use quilt_domain::repositories::{
    BlockRepository, PageRepository, PropertyRepository, RefRepository, RefRow,
    SettingsRepository, TagRepository,
};
use quilt_domain::value_objects::{
    BlockFormat, JournalDay, Priority, PropertyValue, TaskMarker, Uuid,
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
            .map(|s| {
                serde_json::from_str::<HashMap<String, PropertyValue>>(s)
                    .unwrap_or_default()
            })
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
            (id, page_id, parent_id, order_index, level, format, marker, priority,
             content, properties, scheduled, deadline, start_time, repeated, logbook,
             collapsed, created_at, updated_at, refs, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&block.id))
        .bind(uuid_to_blob(&block.page_id))
        .bind(block.parent_id.as_ref().map(uuid_to_blob))
        .bind(block.order)
        .bind(block.level as i64)
        .bind(format_to_str(&block.format))
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
            format = ?, marker = ?, priority = ?, content = ?,
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
    pub fn with_property_repo(
        pool: DbPool,
        repo: Arc<dyn PropertyRepository>,
    ) -> Self {
        Self {
            pool,
            property_repo: Some(repo),
        }
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
    fn properties_to_json(
        props: &HashMap<String, DefaultPropertyEntry<PropertyValue>>,
    ) -> String {
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
/// and `ref_type` columns.
///
/// # Schema
///
/// ```sql
/// CREATE TABLE refs (
///     source_id BLOB NOT NULL,
///     target_id BLOB NOT NULL,
///     ref_type TEXT NOT NULL CHECK(ref_type IN ('page_ref','block_ref','tag','alias')),
///     created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
///     PRIMARY KEY (source_id, target_id, ref_type)
/// );
/// ```
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
            "SELECT source_id, target_id, ref_type FROM refs ORDER BY source_id, target_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Storage(format!("rebuild_index: {}", e)))?;

        rows.iter()
            .map(|row| {
                let source_blob: Vec<u8> = row.get("source_id");
                let target_blob: Vec<u8> = row.get("target_id");
                let ref_type_str: String = row.get("ref_type");

                Ok(RefRow {
                    source_id: blob_to_uuid(&source_blob)?,
                    target_id: blob_to_uuid(&target_blob)?,
                    ref_type: Self::map_ref_type(&ref_type_str)?,
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
        sqlx::query(
            "INSERT OR IGNORE INTO refs (source_id, target_id, ref_type) VALUES (?, ?, ?)",
        )
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
        let escaped = page_name.replace('\\', "\\\\")
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
            Some((timezone, journal_format, start_of_week, preferred_format)) => {
                Ok(UserSettings {
                    timezone,
                    journal_format,
                    start_of_week,
                    preferred_format: BlockFormat::parse_str(&preferred_format)
                        .unwrap_or(BlockFormat::Markdown),
                })
            }
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

// ── Integration Tests ────────────────────────────────────────────────

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
    async fn create_test_page_in_repo(
        repo: &SqlitePageRepository,
        name: &str,
    ) -> Uuid {
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
}
