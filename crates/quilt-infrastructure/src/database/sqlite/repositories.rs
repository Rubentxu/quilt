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
//! - [`SqliteFileRepository`]: Persists [`File`] entities (stubbed)
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

use crate::database::sqlite::connection::DbPool;
use quilt_domain::classes::types::Class;
use quilt_domain::entities::{Block, BlockSummary, DeepLink, File, Page, ScheduledTask, TaskType, LinkSourceType, LinkType};
use quilt_domain::errors::DomainError;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::properties::types::{Cardinality, ClosedValue, PropertyType, ViewContext};
use quilt_domain::repositories::{
    BlockRepository, BlockSummaryRepository, ClassRepository, DeepLinkRepository, FileRepository, PageRepository,
    PropertyRepository, ScheduledTaskRepository, TagRepository,
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
    #[allow(dead_code)]
    deleted_at: Option<i64>,
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
            deleted_at: row.get("deleted_at"),
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
    #[allow(dead_code)]
    deleted_at: Option<i64>,
}

impl PageRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
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
            deleted_at: row.get("deleted_at"),
        })
    }

    fn to_page(&self) -> Result<Page, DomainError> {
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
        })
    }
}

// ── File Row → Entity ──────────────────────────────────────────────────

struct FileRow {
    id: Vec<u8>,
    path: String,
    content: Option<String>,
    hash: Vec<u8>,
    size_bytes: i64,
    mime_type: Option<String>,
    created_at: i64,
    last_modified_at: i64,
}

impl FileRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
        Ok(Self {
            id: row.get("id"),
            path: row.get("path"),
            content: row.get("content"),
            hash: row.get("hash"),
            size_bytes: row.get("size_bytes"),
            mime_type: row.get("mime_type"),
            created_at: row.get("created_at"),
            last_modified_at: row.get("last_modified_at"),
        })
    }

    fn to_file(&self) -> Result<File, DomainError> {
        Ok(File::new_full(
            blob_to_uuid(&self.id)?,
            self.path.clone(),
            self.content.clone(),
            self.hash.clone(),
            self.size_bytes,
            self.mime_type.clone(),
            ts_to_datetime(self.created_at),
            ts_to_datetime(self.last_modified_at),
        ))
    }
}

// ── SqliteBlockRepository ─────────────────────────────────────────────

/// SQLite implementation of the [`BlockRepository`] trait.
///
/// This repository provides persistent storage for block entities
/// using SQLite via the sqlx async driver.
#[derive(Clone)]
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
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ? AND deleted_at IS NULL")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => {
                let br = BlockRow::from_row(&r)?;
                Ok(Some(br.to_block()?))
            }
            None => Ok(None),
        }
    }

    async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE page_id = ? AND deleted_at IS NULL ORDER BY order_index",
        )
        .bind(uuid_to_blob(&page_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_by_page: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE parent_id = ? AND deleted_at IS NULL ORDER BY order_index",
        )
        .bind(uuid_to_blob(&parent_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_children: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ? AND deleted_at IS NULL")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_with_refs: {}", e)))?;

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
        .map_err(|e| DomainError::Database(format!("insert block: {}", e)))?;

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
        .map_err(|e| DomainError::Database(format!("update block: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query("UPDATE blocks SET deleted_at = ? WHERE id = ?")
            .bind(now)
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("soft_delete block: {}", e)))?;
        Ok(())
    }

    async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("hard_delete block: {}", e)))?;
        Ok(())
    }

    async fn restore(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE blocks SET deleted_at = NULL WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("restore block: {}", e)))?;
        Ok(())
    }

    async fn get_deleted_since(&self, since: DateTime<Utc>) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE deleted_at IS NOT NULL AND deleted_at >= ? ORDER BY deleted_at DESC",
        )
        .bind(since.timestamp())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_deleted_since: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM blocks WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("recycle_bin: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn move_block(
        &self,
        id: Uuid,
        new_parent: Option<Uuid>,
        new_order: f64,
    ) -> Result<(), DomainError> {
        let result = sqlx::query(
            "UPDATE blocks SET parent_id = ?, order_index = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(new_parent.as_ref().map(uuid_to_blob))
        .bind(new_order)
        .bind(Utc::now().timestamp())
        .bind(uuid_to_blob(&id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("move_block: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::BlockNotFound(id));
        }

        Ok(())
    }

    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT b.* FROM blocks b
            JOIN refs r ON b.id = r.source_id
            WHERE r.target_id = ? AND b.deleted_at IS NULL
            ORDER BY b.updated_at DESC"#,
        )
        .bind(uuid_to_blob(&block_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_backlinks: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query(
            r#"SELECT b.* FROM blocks b
            JOIN blocks_fts fts ON fts.rowid = b.rowid
            WHERE blocks_fts MATCH ? AND b.deleted_at IS NULL
            ORDER BY bm25(blocks_fts)
            LIMIT ?"#,
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("search: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Block>, DomainError> {
        let rows =
            sqlx::query("SELECT * FROM blocks WHERE updated_at > ? AND deleted_at IS NULL ORDER BY updated_at DESC")
                .bind(since.timestamp())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("get_updated_since: {}", e)))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blocks WHERE page_id = ? AND deleted_at IS NULL",
        )
        .bind(uuid_to_blob(&page_id))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("count_by_page: {}", e)))?;

        Ok(count as usize)
    }
}

// ── SqlitePageRepository ───────────────────────────────────────────────

/// SQLite implementation of the [`PageRepository`] trait.
///
/// This repository provides persistent storage for page entities
/// using SQLite via the sqlx async driver.
#[derive(Clone)]
pub struct SqlitePageRepository {
    pool: DbPool,
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
        Self { pool }
    }
}

#[async_trait]
impl PageRepository for SqlitePageRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query("SELECT * FROM pages WHERE id = ? AND deleted_at IS NULL")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_by_name(&self, name: &str) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query("SELECT * FROM pages WHERE name = ? AND deleted_at IS NULL")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_name: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_journal(&self, day: JournalDay) -> Result<Option<Page>, DomainError> {
        let row = sqlx::query(
            "SELECT * FROM pages WHERE journal_day = ? AND journal = 1 AND deleted_at IS NULL",
        )
        .bind(day.as_i32())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_journal: {}", e)))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE deleted_at IS NULL ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_all: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM pages WHERE namespace_id = ? AND deleted_at IS NULL ORDER BY name",
        )
        .bind(uuid_to_blob(&namespace_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_namespace_pages: {}", e)))?;

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
        .map_err(|e| DomainError::Database(format!("insert page: {}", e)))?;

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
        .map_err(|e| DomainError::Database(format!("update page: {}", e)))?;

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
            .map_err(|e| DomainError::Database(format!("rename page: {}", e)))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query("UPDATE pages SET deleted_at = ? WHERE id = ?")
            .bind(now)
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("soft_delete page: {}", e)))?;
        Ok(())
    }

    async fn soft_delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.delete(id).await
    }

    async fn hard_delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("hard_delete page: {}", e)))?;
        Ok(())
    }

    async fn restore(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE pages SET deleted_at = NULL WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("restore page: {}", e)))?;
        Ok(())
    }

    async fn recycle_bin(&self) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM pages WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("recycle_bin: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_deleted_since(&self, since: DateTime<Utc>) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM pages WHERE deleted_at IS NOT NULL AND deleted_at >= ? ORDER BY deleted_at DESC",
        )
        .bind(since.timestamp())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_deleted_since: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE updated_at > ? AND deleted_at IS NULL ORDER BY updated_at DESC")
            .bind(since.timestamp())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_updated_since: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM pages WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_recent: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE deleted_at IS NULL")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("count: {}", e)))?;

        Ok(count as usize)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError> {
        let like = format!("%{}%", query);
        let rows = sqlx::query(
            "SELECT * FROM pages WHERE name LIKE ? AND deleted_at IS NULL ORDER BY name LIMIT ?",
        )
        .bind(&like)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("search: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_orphan_pages(&self) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query(
            r#"
            SELECT p.* FROM pages p
            LEFT JOIN blocks b ON p.id = b.page_id AND b.deleted_at IS NULL
            WHERE b.id IS NULL AND p.deleted_at IS NULL
            ORDER BY p.updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_orphan_pages: {}", e)))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }
}

// ── SqliteFileRepository ───────────────────────────────────────────────

/// SQLite implementation of the [`FileRepository`] trait.
///
/// This repository provides persistent storage for file entities
/// using SQLite via the sqlx async driver.
#[derive(Clone)]
pub struct SqliteFileRepository {
    pool: DbPool,
}

impl SqliteFileRepository {
    /// Creates a new `SqliteFileRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FileRepository for SqliteFileRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<File>, DomainError> {
        let row = sqlx::query("SELECT * FROM files WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => {
                let fr = FileRow::from_row(&r)?;
                Ok(Some(fr.to_file()?))
            }
            None => Ok(None),
        }
    }

    async fn get_by_path(&self, path: &str) -> Result<Option<File>, DomainError> {
        let row = sqlx::query("SELECT * FROM files WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_path: {}", e)))?;

        match row {
            Some(r) => {
                let fr = FileRow::from_row(&r)?;
                Ok(Some(fr.to_file()?))
            }
            None => Ok(None),
        }
    }

    async fn get_by_directory(&self, dir: &str) -> Result<Vec<File>, DomainError> {
        let pattern = format!("{}/%", dir);
        let rows = sqlx::query("SELECT * FROM files WHERE path LIKE ? AND path != ? ORDER BY path")
            .bind(&pattern)
            .bind(dir)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_directory: {}", e)))?;

        rows.iter()
            .map(|r| FileRow::from_row(r)?.to_file())
            .collect()
    }

    async fn insert(&self, file: &File) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO files
            (id, path, content, hash, size_bytes, mime_type, created_at, last_modified_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&file.id))
        .bind(&file.path)
        .bind(&file.content)
        .bind(&file.hash)
        .bind(file.size_bytes)
        .bind(&file.mime_type)
        .bind(datetime_to_ts(&file.created_at))
        .bind(datetime_to_ts(&file.last_modified_at))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("insert file: {}", e)))?;

        Ok(())
    }

    async fn update(&self, file: &File) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE files SET
            path = ?, content = ?, hash = ?, size_bytes = ?,
            mime_type = ?, last_modified_at = ?
            WHERE id = ?"#,
        )
        .bind(&file.path)
        .bind(&file.content)
        .bind(&file.hash)
        .bind(file.size_bytes)
        .bind(&file.mime_type)
        .bind(datetime_to_ts(&file.last_modified_at))
        .bind(uuid_to_blob(&file.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("update file: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM files WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete file: {}", e)))?;
        Ok(())
    }

    async fn delete_by_path(&self, path: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM files WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete_by_path: {}", e)))?;
        Ok(())
    }

    async fn get_by_type(&self, mime_prefix: &str) -> Result<Vec<File>, DomainError> {
        let pattern = format!("{}%", mime_prefix);
        let rows = sqlx::query("SELECT * FROM files WHERE mime_type LIKE ? ORDER BY path")
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_type: {}", e)))?;

        rows.iter()
            .map(|r| FileRow::from_row(r)?.to_file())
            .collect()
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<File>, DomainError> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"SELECT * FROM files
            WHERE path LIKE ? OR content LIKE ?
            ORDER BY path
            LIMIT ?"#,
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("search: {}", e)))?;

        rows.iter()
            .map(|r| FileRow::from_row(r)?.to_file())
            .collect()
    }
}

// ── SqliteTagRepository ────────────────────────────────────────────────

/// SQLite implementation of the [`TagRepository`] trait.
///
/// This repository manages the many-to-many relationship between pages and tags,
/// providing efficient tag-based querying and searching.
#[derive(Clone)]
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
            .map_err(|e| DomainError::Database(format!("get_by_page: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query("SELECT page_id FROM tags WHERE tag = ?")
            .bind(tag)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_pages_with_tag: {}", e)))?;

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
            .map_err(|e| DomainError::Database(format!("add_tag: {}", e)))?;
        Ok(())
    }

    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM tags WHERE page_id = ? AND tag = ?")
            .bind(uuid_to_blob(&page_id))
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("remove_tag: {}", e)))?;
        Ok(())
    }

    async fn get_all_tags(&self) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query("SELECT DISTINCT tag FROM tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_all_tags: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_tag_counts(&self) -> Result<Vec<(String, usize)>, DomainError> {
        let rows =
            sqlx::query("SELECT tag, COUNT(*) as cnt FROM tags GROUP BY tag ORDER BY cnt DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("get_tag_counts: {}", e)))?;

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
                .map_err(|e| DomainError::Database(format!("search_tags: {}", e)))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }
}

// ── SqlitePropertyRepository ─────────────────────────────────────────────

/// SQLite implementation of the [`PropertyRepository`] trait.
pub struct SqlitePropertyRepository {
    pool: DbPool,
}

impl SqlitePropertyRepository {
    /// Creates a new `SqlitePropertyRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_property_def(
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<PropertyDefinition, DomainError> {
        let id = blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref())?;
        let property_type = PropertyType::from_str(&row.get::<String, _>("property_type"))
            .ok_or_else(|| {
                DomainError::InvalidData(format!(
                    "Invalid property type: {}",
                    row.get::<String, _>("property_type")
                ))
            })?;
        let cardinality =
            Cardinality::from_str(&row.get::<String, _>("cardinality")).ok_or_else(|| {
                DomainError::InvalidData(format!(
                    "Invalid cardinality: {}",
                    row.get::<String, _>("cardinality")
                ))
            })?;
        let view_context = ViewContext::from_str(&row.get::<String, _>("view_context"))
            .ok_or_else(|| {
                DomainError::InvalidData(format!(
                    "Invalid view context: {}",
                    row.get::<String, _>("view_context")
                ))
            })?;

        Ok(PropertyDefinition {
            id,
            db_ident: row.get("db_ident"),
            title: row.get("title"),
            property_type,
            cardinality,
            closed_values: Vec::new(), // Loaded separately
            view_context,
            public: row.get::<i64, _>("public") != 0,
            queryable: row.get::<i64, _>("queryable") != 0,
            hidden: row.get::<i64, _>("hidden") != 0,
            attribute: row.get("attribute"),
        })
    }
}

#[async_trait]
impl PropertyRepository for SqlitePropertyRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyDefinition>, DomainError> {
        let row = sqlx::query("SELECT * FROM property_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_property_def(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_by_db_ident(
        &self,
        ident: &str,
    ) -> Result<Option<PropertyDefinition>, DomainError> {
        let row = sqlx::query("SELECT * FROM property_definitions WHERE db_ident = ?")
            .bind(ident)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_db_ident: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_property_def(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
        let rows = sqlx::query("SELECT * FROM property_definitions ORDER BY title")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_all: {}", e)))?;

        let mut results = Vec::new();
        for row in rows {
            let mut def = Self::row_to_property_def(&row)?;
            // Load closed values
            let closed_values = self.get_closed_values(def.id).await?;
            def.closed_values = closed_values;
            results.push(def);
        }
        Ok(results)
    }

    async fn insert(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"INSERT INTO property_definitions
            (id, db_ident, title, property_type, cardinality, view_context, public, queryable, hidden, attribute, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
        .bind(&def.attribute)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("insert property_definitions: {}", e)))?;

        // Insert closed values
        for cv in &def.closed_values {
            sqlx::query(
                r#"INSERT INTO closed_values (id, property_id, db_ident, value, icon, "order", created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)"#,
            )
            .bind(uuid_to_blob(&cv.id))
            .bind(uuid_to_blob(&def.id))
            .bind(&cv.db_ident)
            .bind(&cv.value)
            .bind(&cv.icon)
            .bind(cv.order)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("insert closed_values: {}", e)))?;
        }

        Ok(())
    }

    async fn update(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"UPDATE property_definitions SET
            db_ident = ?, title = ?, property_type = ?, cardinality = ?, view_context = ?,
            public = ?, queryable = ?, hidden = ?, attribute = ?, updated_at = ?
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
        .bind(&def.attribute)
        .bind(now)
        .bind(uuid_to_blob(&def.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("update property_definitions: {}", e)))?;

        Ok(())
    }

    async fn get_closed_values(&self, property_id: Uuid) -> Result<Vec<ClosedValue>, DomainError> {
        let rows =
            sqlx::query("SELECT * FROM closed_values WHERE property_id = ? ORDER BY \"order\"")
                .bind(uuid_to_blob(&property_id))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("get_closed_values: {}", e)))?;

        rows.iter()
            .map(|row| {
                Ok(ClosedValue {
                    id: blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref())?,
                    db_ident: row.get("db_ident"),
                    value: row.get("value"),
                    icon: row.get("icon"),
                    order: row.get("order"),
                })
            })
            .collect()
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM property_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete property_definitions: {}", e)))?;
        Ok(())
    }
}

// ── SqliteClassRepository ────────────────────────────────────────────────

/// SQLite implementation of the [`ClassRepository`] trait.
pub struct SqliteClassRepository {
    pool: DbPool,
}

impl SqliteClassRepository {
    /// Creates a new `SqliteClassRepository` with the given connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_class(row: &sqlx::sqlite::SqliteRow) -> Result<Class, DomainError> {
        let id = blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref())?;
        let extends: Option<Uuid> = row
            .get::<Option<Vec<u8>>, _>("extends")
            .and_then(|b| if b.is_empty() { None } else { Some(b) })
            .map(|b| blob_to_uuid(&b))
            .transpose()?;

        Ok(Class {
            id,
            db_ident: row.get("db_ident"),
            title: row.get("title"),
            extends,
            required_properties: Vec::new(), // Loaded separately
            default_properties: Vec::new(),  // Loaded separately
            icon: row.get("icon"),
            builtin: row.get::<i64, _>("builtin") != 0,
            user_defined: row.get::<i64, _>("user_defined") != 0,
        })
    }
}

#[async_trait]
impl ClassRepository for SqliteClassRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Class>, DomainError> {
        let row = sqlx::query("SELECT * FROM class_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => {
                let mut class = Self::row_to_class(&r)?;
                class.required_properties = self.get_required_properties(class.id).await?;
                class.default_properties = self.get_default_properties(class.id).await?;
                Ok(Some(class))
            }
            None => Ok(None),
        }
    }

    async fn get_by_db_ident(&self, ident: &str) -> Result<Option<Class>, DomainError> {
        let row = sqlx::query("SELECT * FROM class_definitions WHERE db_ident = ?")
            .bind(ident)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_db_ident: {}", e)))?;

        match row {
            Some(r) => {
                let mut class = Self::row_to_class(&r)?;
                class.required_properties = self.get_required_properties(class.id).await?;
                class.default_properties = self.get_default_properties(class.id).await?;
                Ok(Some(class))
            }
            None => Ok(None),
        }
    }

    async fn get_ancestors(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        // Use recursive CTE to get all ancestors
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE ancestors AS (
                SELECT id, extends FROM class_definitions WHERE id = ?
                UNION ALL
                SELECT c.id, c.extends FROM class_definitions c
                JOIN ancestors a ON c.id = a.extends
            )
            SELECT id FROM ancestors
            "#,
        )
        .bind(uuid_to_blob(&class_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_ancestors: {}", e)))?;

        rows.iter()
            .map(|row| blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref()))
            .collect()
    }

    async fn get_required_properties(&self, class_id: Uuid) -> Result<Vec<Uuid>, DomainError> {
        let rows =
            sqlx::query("SELECT property_id FROM class_required_properties WHERE class_id = ?")
                .bind(uuid_to_blob(&class_id))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("get_required_properties: {}", e)))?;

        rows.iter()
            .map(|row| blob_to_uuid(row.get::<Vec<u8>, _>("property_id").as_ref()))
            .collect()
    }

    async fn get_default_properties(
        &self,
        class_id: Uuid,
    ) -> Result<Vec<(Uuid, String)>, DomainError> {
        let rows = sqlx::query("SELECT property_id, default_value_json FROM class_default_properties WHERE class_id = ?")
            .bind(uuid_to_blob(&class_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_default_properties: {}", e)))?;

        rows.iter()
            .map(|row| {
                let prop_id = blob_to_uuid(row.get::<Vec<u8>, _>("property_id").as_ref())?;
                let default_json: String = row.get("default_value_json");
                Ok((prop_id, default_json))
            })
            .collect()
    }

    async fn insert(&self, class: &Class) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"INSERT INTO class_definitions
            (id, db_ident, title, extends, icon, builtin, user_defined, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&class.id))
        .bind(&class.db_ident)
        .bind(&class.title)
        .bind(class.extends.as_ref().map(uuid_to_blob))
        .bind(&class.icon)
        .bind(class.builtin as i64)
        .bind(class.user_defined as i64)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("insert class_definitions: {}", e)))?;

        // Insert inheritance
        if let Some(parent_id) = class.extends {
            self.add_inheritance(class.id, parent_id).await?;
        }

        // Insert required properties
        for prop_id in &class.required_properties {
            self.add_required_property(class.id, *prop_id).await?;
        }

        // Insert default properties
        for (prop_id, default_json) in &class.default_properties {
            self.add_default_property(class.id, *prop_id, default_json)
                .await?;
        }

        Ok(())
    }

    async fn update(&self, class: &Class) -> Result<(), DomainError> {
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"UPDATE class_definitions SET
            db_ident = ?, title = ?, extends = ?, icon = ?, builtin = ?, user_defined = ?, updated_at = ?
            WHERE id = ?"#,
        )
        .bind(&class.db_ident)
        .bind(&class.title)
        .bind(class.extends.as_ref().map(uuid_to_blob))
        .bind(&class.icon)
        .bind(class.builtin as i64)
        .bind(class.user_defined as i64)
        .bind(now)
        .bind(uuid_to_blob(&class.id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("update class_definitions: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM class_definitions WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete class_definitions: {}", e)))?;
        Ok(())
    }

    async fn add_inheritance(&self, child_id: Uuid, parent_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("INSERT OR IGNORE INTO class_inheritance (class_id, parent_id) VALUES (?, ?)")
            .bind(uuid_to_blob(&child_id))
            .bind(uuid_to_blob(&parent_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("add_inheritance: {}", e)))?;
        Ok(())
    }

    async fn remove_inheritance(&self, child_id: Uuid, parent_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM class_inheritance WHERE class_id = ? AND parent_id = ?")
            .bind(uuid_to_blob(&child_id))
            .bind(uuid_to_blob(&parent_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("remove_inheritance: {}", e)))?;
        Ok(())
    }

    async fn add_required_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT OR IGNORE INTO class_required_properties (class_id, property_id) VALUES (?, ?)",
        )
        .bind(uuid_to_blob(&class_id))
        .bind(uuid_to_blob(&property_id))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("add_required_property: {}", e)))?;
        Ok(())
    }

    async fn remove_required_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM class_required_properties WHERE class_id = ? AND property_id = ?")
            .bind(uuid_to_blob(&class_id))
            .bind(uuid_to_blob(&property_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("remove_required_property: {}", e)))?;
        Ok(())
    }

    async fn add_default_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
        default_json: &str,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT OR REPLACE INTO class_default_properties (class_id, property_id, default_value_json) VALUES (?, ?, ?)",
        )
        .bind(uuid_to_blob(&class_id))
        .bind(uuid_to_blob(&property_id))
        .bind(default_json)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("add_default_property: {}", e)))?;
        Ok(())
    }

    async fn remove_default_property(
        &self,
        class_id: Uuid,
        property_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM class_default_properties WHERE class_id = ? AND property_id = ?")
            .bind(uuid_to_blob(&class_id))
            .bind(uuid_to_blob(&property_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("remove_default_property: {}", e)))?;
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
    async fn test_block_soft_delete_excluded_from_queries() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("soft-delete-test");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let b1 = make_block(page.id, "Active block");
        let b2 = make_block(page.id, "To be soft-deleted");
        repo.insert(&b1).await.unwrap();
        repo.insert(&b2).await.unwrap();

        // Soft-delete b2
        repo.delete(b2.id).await.unwrap();

        // get_by_id should return None for soft-deleted
        assert!(repo.get_by_id(b2.id).await.unwrap().is_none());

        // get_by_page should exclude soft-deleted
        let blocks = repo.get_by_page(page.id).await.unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, b1.id);

        // count_by_page should exclude soft-deleted
        assert_eq!(repo.count_by_page(page.id).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_block_hard_delete() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("hard-delete-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "To hard delete");
        repo.insert(&block).await.unwrap();

        repo.hard_delete(block.id).await.unwrap();

        let found = repo.get_by_id(block.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_block_restore() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("restore-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "To restore");
        repo.insert(&block).await.unwrap();

        // Soft-delete
        repo.delete(block.id).await.unwrap();
        assert!(repo.get_by_id(block.id).await.unwrap().is_none());

        // Restore
        repo.restore(block.id).await.unwrap();

        // Should be visible again
        let found = repo.get_by_id(block.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "To restore");
    }

    #[tokio::test]
    async fn test_block_get_deleted_since() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("deleted-since-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "To delete");
        repo.insert(&block).await.unwrap();

        let before_delete = chrono::Utc::now();

        // Soft-delete
        repo.delete(block.id).await.unwrap();

        // get_deleted_since should return the deleted block
        let deleted = repo.get_deleted_since(before_delete).await.unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, block.id);
    }

    #[tokio::test]
    async fn test_block_move_fails_if_soft_deleted() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("move-deleted-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "To move");
        repo.insert(&block).await.unwrap();

        // Soft-delete
        repo.delete(block.id).await.unwrap();

        // Move should fail
        let result = repo.move_block(block.id, None, 2.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_block_search_excludes_soft_deleted() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("search-soft-delete-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let b1 = make_block(page.id, "Rust programming");
        let b2 = make_block(page.id, "Rust to delete");
        repo.insert(&b1).await.unwrap();
        repo.insert(&b2).await.unwrap();

        // Soft-delete b2
        repo.delete(b2.id).await.unwrap();

        // Search should exclude soft-deleted
        let results = repo.search("\"Rust\"", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, b1.id);
    }

    #[tokio::test]
    async fn test_block_get_updated_since_excludes_soft_deleted() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("updated-since-soft-delete-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "Recent block");
        repo.insert(&block).await.unwrap();

        // Soft-delete
        repo.delete(block.id).await.unwrap();

        let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
        let results = repo.get_updated_since(one_hour_ago).await.unwrap();
        assert!(results.is_empty());
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
        let page = Page::new_journal(day, BlockFormat::Markdown).unwrap();
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

    // ── FileRepository Tests ────────────────────────────────────────

    fn make_file(path: &str, content: Option<String>, mime_type: Option<String>) -> File {
        File::new(path, content, mime_type)
    }

    #[tokio::test]
    async fn test_file_insert_and_get_by_id() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let file = make_file(
            "notes/test.md",
            Some("# Test".to_string()),
            Some("text/markdown".to_string()),
        );
        repo.insert(&file).await.unwrap();

        let found = repo.get_by_id(file.id).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "notes/test.md");
        assert_eq!(found.mime_type, Some("text/markdown".to_string()));
    }

    #[tokio::test]
    async fn test_file_get_by_path() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let file = make_file(
            "docs/readme.txt",
            Some("Hello".to_string()),
            Some("text/plain".to_string()),
        );
        repo.insert(&file).await.unwrap();

        let found = repo.get_by_path("docs/readme.txt").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, "docs/readme.txt");
    }

    #[tokio::test]
    async fn test_file_get_by_path_not_found() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let found = repo.get_by_path("nonexistent.md").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_file_get_by_directory() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        repo.insert(&make_file(
            "assets/img/a.png",
            None,
            Some("image/png".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "assets/img/b.jpg",
            None,
            Some("image/jpeg".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "assets/doc/c.txt",
            None,
            Some("text/plain".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "other/d.txt",
            None,
            Some("text/plain".to_string()),
        ))
        .await
        .unwrap();

        let files = repo.get_by_directory("assets").await.unwrap();
        assert_eq!(files.len(), 3);
        // Should not include the directory itself
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"assets/img/a.png"));
        assert!(paths.contains(&"assets/img/b.jpg"));
        assert!(paths.contains(&"assets/doc/c.txt"));
    }

    #[tokio::test]
    async fn test_file_get_by_directory_empty() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let files = repo.get_by_directory("nonexistent").await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_file_update() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let mut file = make_file(
            "update/me.md",
            Some("Original".to_string()),
            Some("text/markdown".to_string()),
        );
        repo.insert(&file).await.unwrap();

        file.content = Some("Updated".to_string());
        file.path = "update/me.md".to_string(); // path stays same
        repo.update(&file).await.unwrap();

        let found = repo.get_by_id(file.id).await.unwrap().unwrap();
        assert_eq!(found.content, Some("Updated".to_string()));
    }

    #[tokio::test]
    async fn test_file_delete() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let file = make_file("to_delete.md", Some("Content".to_string()), None);
        repo.insert(&file).await.unwrap();

        repo.delete(file.id).await.unwrap();

        let found = repo.get_by_id(file.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_file_delete_by_path() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        let file = make_file("delete_me.md", Some("Content".to_string()), None);
        repo.insert(&file).await.unwrap();

        repo.delete_by_path("delete_me.md").await.unwrap();

        let found = repo.get_by_path("delete_me.md").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_file_get_by_type() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        repo.insert(&make_file("a.png", None, Some("image/png".to_string())))
            .await
            .unwrap();
        repo.insert(&make_file("b.jpg", None, Some("image/jpeg".to_string())))
            .await
            .unwrap();
        repo.insert(&make_file("c.txt", None, Some("text/plain".to_string())))
            .await
            .unwrap();
        repo.insert(&make_file("d.md", None, Some("text/markdown".to_string())))
            .await
            .unwrap();

        let images = repo.get_by_type("image").await.unwrap();
        assert_eq!(images.len(), 2);
        let paths: Vec<_> = images.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"a.png"));
        assert!(paths.contains(&"b.jpg"));
    }

    #[tokio::test]
    async fn test_file_search() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        repo.insert(&make_file(
            "pages/rust.md",
            Some("# Rust".to_string()),
            Some("text/markdown".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "pages/python.md",
            Some("# Python".to_string()),
            Some("text/markdown".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "notes/rust.txt",
            Some("Rust notes".to_string()),
            Some("text/plain".to_string()),
        ))
        .await
        .unwrap();

        let results = repo.search("rust", 10).await.unwrap();
        assert_eq!(results.len(), 2);
        let paths: Vec<_> = results.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"pages/rust.md"));
        assert!(paths.contains(&"notes/rust.txt"));
    }

    #[tokio::test]
    async fn test_file_search_with_limit() {
        let pool = setup_test_db().await;
        let repo = SqliteFileRepository::new(pool);

        repo.insert(&make_file(
            "a.md",
            Some("content".to_string()),
            Some("text/markdown".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "b.md",
            Some("content".to_string()),
            Some("text/markdown".to_string()),
        ))
        .await
        .unwrap();
        repo.insert(&make_file(
            "c.md",
            Some("content".to_string()),
            Some("text/markdown".to_string()),
        ))
        .await
        .unwrap();

        let results = repo.search("content", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    // ── Additional BlockRepository Tests ───────────────────────────────

    #[tokio::test]
    async fn test_block_get_children() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("children-test");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let parent = make_block(page.id, "Parent");
        let child1 = make_block(page.id, "Child 1");
        let child2 = make_block(page.id, "Child 2");
        repo.insert(&parent).await.unwrap();
        repo.insert(&child1).await.unwrap();
        repo.insert(&child2).await.unwrap();

        // Move children under parent
        repo.move_block(child1.id, Some(parent.id), 1.0)
            .await
            .unwrap();
        repo.move_block(child2.id, Some(parent.id), 2.0)
            .await
            .unwrap();

        let children = repo.get_children(parent.id).await.unwrap();
        assert_eq!(children.len(), 2);
    }

    #[tokio::test]
    async fn test_block_get_children_empty() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("empty-children-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let block = make_block(page.id, "Orphan");
        repo.insert(&block).await.unwrap();

        let children = repo.get_children(block.id).await.unwrap();
        assert!(children.is_empty());
    }

    #[tokio::test]
    async fn test_block_get_with_refs() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("refs-test");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let block = make_block(page.id, "Block with refs");
        repo.insert(&block).await.unwrap();

        let (found, refs) = repo.get_with_refs(block.id).await.unwrap();
        assert_eq!(found.id, block.id);
        assert!(refs.is_empty()); // No refs in this simple test
    }

    #[tokio::test]
    async fn test_block_get_with_refs_not_found() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let result = repo.get_with_refs(Uuid::new_v4()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_block_get_updated_since() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("updated-test");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let block = make_block(page.id, "Recent block");
        repo.insert(&block).await.unwrap();

        let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
        let results = repo.get_updated_since(one_hour_ago).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_block_get_updated_since_empty() {
        let pool = setup_test_db().await;
        let repo = SqliteBlockRepository::new(pool.clone());

        let page = make_page("no-updates-test");
        SqlitePageRepository::new(pool).insert(&page).await.unwrap();

        let one_hour_from_now = chrono::Utc::now() + chrono::Duration::hours(1);
        let results = repo.get_updated_since(one_hour_from_now).await.unwrap();
        assert!(results.is_empty());
    }

    // ── Additional PageRepository Tests ────────────────────────────────

    #[tokio::test]
    async fn test_page_get_by_id() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool.clone());

        let page = make_page("get-by-id-test");
        repo.insert(&page).await.unwrap();

        let found = repo.get_by_id(page.id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, page.id);
    }

    #[tokio::test]
    async fn test_page_get_by_id_not_found() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let found = repo.get_by_id(Uuid::new_v4()).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_page_get_updated_since() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool.clone());

        let page = make_page("recent-page");
        repo.insert(&page).await.unwrap();

        let one_hour_ago = chrono::Utc::now() - chrono::Duration::hours(1);
        let results = repo.get_updated_since(one_hour_ago).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_page_get_updated_since_empty() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool.clone());

        let page = make_page("old-page");
        repo.insert(&page).await.unwrap();

        let one_hour_from_now = chrono::Utc::now() + chrono::Duration::hours(1);
        let results = repo.get_updated_since(one_hour_from_now).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_page_get_recent() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool.clone());

        repo.insert(&make_page("page-a")).await.unwrap();
        repo.insert(&make_page("page-b")).await.unwrap();
        repo.insert(&make_page("page-c")).await.unwrap();

        let recent = repo.get_recent(2).await.unwrap();
        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn test_page_get_recent_empty() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool);

        let recent = repo.get_recent(10).await.unwrap();
        assert!(recent.is_empty());
    }

    #[tokio::test]
    async fn test_page_get_namespace_pages() {
        let pool = setup_test_db().await;
        let repo = SqlitePageRepository::new(pool.clone());

        let namespace_id = Uuid::new_v4();
        let _page1 = make_page("ns-page-1");
        let _page2 = make_page("ns-page-2");

        // Note: Page doesn't have namespace_id field in this test helper
        // This test would require modifying make_page or using direct insertion
        // For now, test that the method works with empty result
        let pages = repo.get_namespace_pages(namespace_id).await.unwrap();
        assert!(pages.is_empty());
    }

    // ── Additional TagRepository Tests ────────────────────────────────

    #[tokio::test]
    async fn test_tag_get_pages_with_tag() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let page = make_page("tagged-for-search");
        page_repo.insert(&page).await.unwrap();

        tag_repo.add_tag(page.id, "searchable").await.unwrap();

        let pages = tag_repo.get_pages_with_tag("searchable").await.unwrap();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0], page.id);
    }

    #[tokio::test]
    async fn test_tag_get_pages_with_tag_not_found() {
        let pool = setup_test_db().await;
        let tag_repo = SqliteTagRepository::new(pool);

        let pages = tag_repo.get_pages_with_tag("nonexistent").await.unwrap();
        assert!(pages.is_empty());
    }

    #[tokio::test]
    async fn test_tag_get_all_tags() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let p1 = make_page("page1");
        let p2 = make_page("page2");
        page_repo.insert(&p1).await.unwrap();
        page_repo.insert(&p2).await.unwrap();

        tag_repo.add_tag(p1.id, "alpha").await.unwrap();
        tag_repo.add_tag(p2.id, "beta").await.unwrap();
        tag_repo.add_tag(p1.id, "gamma").await.unwrap();

        let all_tags = tag_repo.get_all_tags().await.unwrap();
        assert_eq!(all_tags.len(), 3);
        assert!(all_tags.contains(&"alpha".to_string()));
        assert!(all_tags.contains(&"beta".to_string()));
        assert!(all_tags.contains(&"gamma".to_string()));
    }

    #[tokio::test]
    async fn test_tag_search_tags() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let page = make_page("search-test");
        page_repo.insert(&page).await.unwrap();

        tag_repo.add_tag(page.id, "programming").await.unwrap();
        tag_repo.add_tag(page.id, "procrastination").await.unwrap();
        tag_repo.add_tag(page.id, "productivity").await.unwrap();

        let results = tag_repo.search_tags("pro", 10).await.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results.contains(&"programming".to_string()));
        assert!(results.contains(&"procrastination".to_string()));
        assert!(results.contains(&"productivity".to_string()));
    }

    #[tokio::test]
    async fn test_tag_search_tags_with_limit() {
        let pool = setup_test_db().await;
        let page_repo = SqlitePageRepository::new(pool.clone());
        let tag_repo = SqliteTagRepository::new(pool);

        let page = make_page("limit-test");
        page_repo.insert(&page).await.unwrap();

        tag_repo.add_tag(page.id, "tag-one").await.unwrap();
        tag_repo.add_tag(page.id, "tag-two").await.unwrap();
        tag_repo.add_tag(page.id, "tag-three").await.unwrap();

        let results = tag_repo.search_tags("tag", 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

} // end of mod tests

// ── SqliteBlockSummaryRepository ───────────────────────────────────────────

/// SQLite implementation of the [`BlockSummaryRepository`] trait.
///
/// Stores LLM-generated block summaries with content hashes for staleness detection.
#[derive(Clone)]
pub struct SqliteBlockSummaryRepository {
    pool: DbPool,
}

impl SqliteBlockSummaryRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_summary(row: &sqlx::sqlite::SqliteRow) -> Result<BlockSummary, DomainError> {
        let block_id = blob_to_uuid(row.get::<Vec<u8>, _>("block_id").as_ref())?;
        let content_hash: Vec<u8> = row.get("content_hash");
        Ok(BlockSummary {
            block_id,
            summary: row.get("summary"),
            content_hash,
            generated_at: ts_to_datetime(row.get::<i64, _>("generated_at")),
        })
    }
}

#[async_trait]
impl BlockSummaryRepository for SqliteBlockSummaryRepository {
    async fn get(&self, block_id: Uuid) -> Result<Option<BlockSummary>, DomainError> {
        let row = sqlx::query("SELECT * FROM block_summaries WHERE block_id = ?")
            .bind(uuid_to_blob(&block_id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get summary: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_summary(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_batch(&self, block_ids: &[Uuid]) -> Result<Vec<BlockSummary>, DomainError> {
        if block_ids.is_empty() {
            return Ok(vec![]);
        }
        let blobs: Vec<Vec<u8>> = block_ids.iter().map(uuid_to_blob).collect();
        let placeholders: Vec<String> = blobs.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT * FROM block_summaries WHERE block_id IN ({})",
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for blob in &blobs {
            query = query.bind(blob);
        }

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_batch summaries: {}", e)))?;

        rows.iter()
            .map(Self::row_to_summary)
            .collect()
    }

    async fn upsert(&self, summary: &BlockSummary) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO block_summaries (block_id, summary, content_hash, generated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(block_id) DO UPDATE SET
            summary = excluded.summary,
            content_hash = excluded.content_hash,
            generated_at = excluded.generated_at"#,
        )
        .bind(uuid_to_blob(&summary.block_id))
        .bind(&summary.summary)
        .bind(&summary.content_hash)
        .bind(datetime_to_ts(&summary.generated_at))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("upsert summary: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, block_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM block_summaries WHERE block_id = ?")
            .bind(uuid_to_blob(&block_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete summary: {}", e)))?;
        Ok(())
    }

    async fn list_stale(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Uuid>, DomainError> {
        let rows =
            sqlx::query("SELECT block_id FROM block_summaries WHERE generated_at < ?")
                .bind(datetime_to_ts(&before))
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("list_stale: {}", e)))?;

        rows.iter()
            .map(|r| {
                let blob: Vec<u8> = r.get("block_id");
                blob_to_uuid(&blob)
            })
            .collect()
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM block_summaries")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| DomainError::Database(format!("count summaries: {}", e)))?;
        Ok(count as usize)
    }

    async fn count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM block_summaries WHERE generated_at >= ?",
        )
        .bind(datetime_to_ts(&since))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("count_since: {}", e)))?;
        Ok(count as usize)
    }
}

// ── SqliteScheduledTaskRepository ──────────────────────────────────────────

/// SQLite implementation of the [`ScheduledTaskRepository`] trait.
#[derive(Clone)]
pub struct SqliteScheduledTaskRepository {
    pool: DbPool,
}

impl SqliteScheduledTaskRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn row_to_task(row: &sqlx::sqlite::SqliteRow) -> Result<ScheduledTask, DomainError> {
        let task_type_str: String = row.get("task_type");
        let task_type = match task_type_str.as_str() {
            "RebuildIndex" => TaskType::RebuildIndex,
            "CleanStaleSummaries" => TaskType::CleanStaleSummaries,
            "HealthCheck" => TaskType::HealthCheck,
            _ => {
                return Err(DomainError::InvalidData(format!(
                    "Unknown task_type: {}",
                    task_type_str
                )))
            }
        };

        Ok(ScheduledTask {
            id: blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref())?,
            name: row.get("name"),
            cron_expr: row.get("cron_expr"),
            task_type,
            enabled: row.get::<i64, _>("enabled") != 0,
            last_run: optional_ts_to_datetime(row.get("last_run")),
            next_run: ts_to_datetime(row.get::<i64, _>("next_run")),
            created_at: ts_to_datetime(row.get::<i64, _>("created_at")),
        })
    }
}

#[async_trait]
impl ScheduledTaskRepository for SqliteScheduledTaskRepository {
    async fn get_by_name(&self, name: &str) -> Result<Option<ScheduledTask>, DomainError> {
        let row = sqlx::query("SELECT * FROM scheduled_tasks WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_name task: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_task(&r)?)),
            None => Ok(None),
        }
    }

    async fn list_due(&self) -> Result<Vec<ScheduledTask>, DomainError> {
        let now = chrono::Utc::now().timestamp();
        let rows = sqlx::query(
            "SELECT * FROM scheduled_tasks WHERE enabled = 1 AND next_run <= ? ORDER BY next_run",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("list_due: {}", e)))?;

        rows.iter().map(Self::row_to_task).collect()
    }

    async fn list_all(&self) -> Result<Vec<ScheduledTask>, DomainError> {
        let rows = sqlx::query("SELECT * FROM scheduled_tasks ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("list_all tasks: {}", e)))?;

        rows.iter().map(Self::row_to_task).collect()
    }

    async fn upsert(&self, task: &ScheduledTask) -> Result<(), DomainError> {
        let type_str = match &task.task_type {
            TaskType::RebuildIndex => "RebuildIndex",
            TaskType::CleanStaleSummaries => "CleanStaleSummaries",
            TaskType::HealthCheck => "HealthCheck",
        };

        sqlx::query(
            r#"INSERT INTO scheduled_tasks
            (id, name, cron_expr, task_type, task_config_json, enabled, last_run, next_run, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
            cron_expr = excluded.cron_expr,
            task_type = excluded.task_type,
            task_config_json = excluded.task_config_json,
            enabled = excluded.enabled,
            last_run = excluded.last_run,
            next_run = excluded.next_run"#,
        )
        .bind(uuid_to_blob(&task.id))
        .bind(&task.name)
        .bind(&task.cron_expr)
        .bind(type_str)
        .bind(String::new())
        .bind(task.enabled as i64)
        .bind(task.last_run.as_ref().map(datetime_to_ts))
        .bind(datetime_to_ts(&task.next_run))
        .bind(datetime_to_ts(&task.created_at))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("upsert task: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, name: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM scheduled_tasks WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete task: {}", e)))?;
        Ok(())
    }

    async fn mark_executed(
        &self,
        name: &str,
        next_run: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE scheduled_tasks SET last_run = ?, next_run = ? WHERE name = ?")
            .bind(now)
            .bind(datetime_to_ts(&next_run))
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("mark_executed: {}", e)))?;
        Ok(())
    }
}

// ── SqliteDeepLinkRepository ──────────────────────────────────────────

/// SQLite implementation of the [`DeepLinkRepository`] trait.
///
/// This repository provides persistent storage for deep links
/// using SQLite via the sqlx async driver.
#[derive(Clone)]
pub struct SqliteDeepLinkRepository {
    pool: DbPool,
}

impl SqliteDeepLinkRepository {
    /// Creates a new `SqliteDeepLinkRepository` with the given connection pool.
    ///
    /// # Arguments
    ///
    /// * `pool` - A SQLite connection pool ([`DbPool`])
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_infrastructure::database::sqlite::repositories::SqliteDeepLinkRepository;
    /// use quilt_infrastructure::database::sqlite::connection::create_pool;
    ///
    /// async {
    ///     let pool = create_pool("/tmp/test.db").await.unwrap();
    ///     let repo = SqliteDeepLinkRepository::new(pool);
    /// };
    /// ```
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeepLinkRepository for SqliteDeepLinkRepository {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<DeepLink>, DomainError> {
        let row = sqlx::query("SELECT * FROM deep_links WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("get_by_id: {}", e)))?;

        match row {
            Some(r) => Ok(Some(self.row_to_deep_link(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<Vec<DeepLink>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE source_id = ? AND source_type = ? ORDER BY created_at DESC",
        )
        .bind(uuid_to_blob(&source_id))
        .bind(source_type.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_by_source: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }

    async fn get_by_target(&self, target_id: Uuid) -> Result<Vec<DeepLink>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE target_id = ? ORDER BY created_at DESC",
        )
        .bind(uuid_to_blob(&target_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_by_target: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }

    async fn get_by_type(&self, link_type: LinkType) -> Result<Vec<DeepLink>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE link_type = ? ORDER BY created_at DESC",
        )
        .bind(link_type.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_by_type: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }

    async fn get_by_source_and_type(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
        link_type: LinkType,
    ) -> Result<Vec<DeepLink>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE source_id = ? AND source_type = ? AND link_type = ? ORDER BY created_at DESC",
        )
        .bind(uuid_to_blob(&source_id))
        .bind(source_type.as_str())
        .bind(link_type.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_by_source_and_type: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }

    async fn insert(&self, link: &DeepLink) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO deep_links
            (id, source_id, source_type, target_id, target_page_name, link_type, external_url, link_text, context, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&link.id))
        .bind(uuid_to_blob(&link.source_id))
        .bind(link.source_type.as_str())
        .bind(link.target_id.as_ref().map(uuid_to_blob))
        .bind(&link.target_page_name)
        .bind(link.link_type.as_str())
        .bind(&link.external_url)
        .bind(&link.link_text)
        .bind(&link.context)
        .bind(datetime_to_ts(&link.created_at))
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("insert deep_link: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM deep_links WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete deep_link: {}", e)))?;
        Ok(())
    }

    async fn delete_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM deep_links WHERE source_id = ? AND source_type = ?")
            .bind(uuid_to_blob(&source_id))
            .bind(source_type.as_str())
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete_by_source: {}", e)))?;
        Ok(())
    }

    async fn delete_by_target(&self, target_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM deep_links WHERE target_id = ?")
            .bind(uuid_to_blob(&target_id))
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Database(format!("delete_by_target: {}", e)))?;
        Ok(())
    }

    async fn count_by_source(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
    ) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deep_links WHERE source_id = ? AND source_type = ?",
        )
        .bind(uuid_to_blob(&source_id))
        .bind(source_type.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("count_by_source: {}", e)))?;

        Ok(count as usize)
    }

    async fn count_by_target(&self, target_id: Uuid) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM deep_links WHERE target_id = ?",
        )
        .bind(uuid_to_blob(&target_id))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("count_by_target: {}", e)))?;

        Ok(count as usize)
    }

    async fn get_page(
        &self,
        source_id: Uuid,
        source_type: LinkSourceType,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<DeepLink>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE source_id = ? AND source_type = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(uuid_to_blob(&source_id))
        .bind(source_type.as_str())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("get_page: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }

    async fn search_by_text(&self, query: &str, limit: usize) -> Result<Vec<DeepLink>, DomainError> {
        let pattern = format!("%{}%", query);
        let rows = sqlx::query(
            "SELECT * FROM deep_links WHERE link_text LIKE ? OR context LIKE ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Database(format!("search_by_text: {}", e)))?;

        rows.iter().map(|r| self.row_to_deep_link(r)).collect()
    }
}

impl SqliteDeepLinkRepository {
    fn row_to_deep_link(&self, row: &sqlx::sqlite::SqliteRow) -> Result<DeepLink, DomainError> {
        let source_type_str: String = row.get("source_type");
        let link_type_str: String = row.get("link_type");

        let source_type = LinkSourceType::try_from_str(&source_type_str)
            .ok_or_else(|| DomainError::InvalidData(format!("Invalid source type: {}", source_type_str)))?;
        let link_type = LinkType::try_from_str(&link_type_str)
            .ok_or_else(|| DomainError::InvalidData(format!("Invalid link type: {}", link_type_str)))?;

        Ok(DeepLink {
            id: blob_to_uuid(row.get::<Vec<u8>, _>("id").as_ref())?,
            source_id: blob_to_uuid(row.get::<Vec<u8>, _>("source_id").as_ref())?,
            source_type,
            target_id: optional_blob_to_uuid(row.get::<Option<Vec<u8>>, _>("target_id").as_deref())?,
            target_page_name: row.get("target_page_name"),
            link_type,
            external_url: row.get("external_url"),
            link_text: row.get("link_text"),
            context: row.get("context"),
            created_at: ts_to_datetime(row.get("created_at")),
        })
    }
}
