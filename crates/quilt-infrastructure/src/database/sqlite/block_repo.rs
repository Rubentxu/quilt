//! SQLite implementation of the [`BlockRepository`] trait.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;


use super::helpers::*;
use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::{BlockQueryRepository, BlockRepository, BulkBlockRepository};
use quilt_domain::value_objects::Uuid;

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
                .and_then(quilt_domain::value_objects::BlockType::parse_str)
                .unwrap_or_default(),
            marker: self.marker.as_deref().and_then(parse_marker),
            priority: self.priority.as_deref().and_then(parse_priority),
            content: self.content.clone(),
            properties: parse_properties(&self.properties),
            scheduled: self.scheduled.map(ts_to_datetime),
            deadline: self.deadline.map(ts_to_datetime),
            start_time: self.start_time.map(ts_to_datetime),
            repeated: self.repeated.map(ts_to_datetime),
            logbook: self.logbook.map(ts_to_datetime),
            collapsed: self.collapsed != 0,
            created_at: ts_to_datetime(self.created_at),
            updated_at: ts_to_datetime(self.updated_at),
            refs: parse_uuid_list(&self.refs),
            tags: parse_tag_list(&self.tags),
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
            .map_err(|e| map_sqlx_error("get_by_id", e))?;

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
            .map_err(|e| map_sqlx_error("get_by_page", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_children(&self, parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query("SELECT * FROM blocks WHERE parent_id = ? ORDER BY order_index")
            .bind(uuid_to_blob(&parent_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_children", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn get_with_refs(&self, id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
        let row = sqlx::query("SELECT * FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_with_refs", e))?;

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
        .bind(block.block_type.as_str())
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
        .map_err(|e| map_sqlx_error("insert block", e))?;

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
        .bind(block.block_type.as_str())
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
        .map_err(|e| map_sqlx_error("update block", e))?;

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("delete block", e))?;
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
        .map_err(|e| map_sqlx_error("move_block", e))?;
        Ok(())
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Block>, DomainError> {
        let rows =
            sqlx::query("SELECT * FROM blocks WHERE updated_at > ? ORDER BY updated_at DESC")
                .bind(since.timestamp())
                .fetch_all(&self.pool)
                .await
                .map_err(|e| map_sqlx_error("get_updated_since", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn count_by_page(&self, page_id: Uuid) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocks WHERE page_id = ?")
            .bind(uuid_to_blob(&page_id))
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("count_by_page", e))?;

        Ok(count as usize)
    }
}

#[async_trait]
impl BlockQueryRepository for SqliteBlockRepository {
    async fn get_backlinks(&self, block_id: Uuid) -> Result<Vec<Block>, DomainError> {
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
        .map_err(|e| map_sqlx_error("get_backlinks", e))?;

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
        .map_err(|e| map_sqlx_error("search", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r)?.to_block())
            .collect()
    }

    async fn count_all(&self) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM blocks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("count_all", e))?;

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
            .map_err(|e| map_sqlx_error("query_dsl", e))?;
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
            .map_err(|e| map_sqlx_error("list_by_property", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }

    async fn list_by_property_key(&self, key: &str, limit: u32) -> Result<Vec<Block>, DomainError> {
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
            .map_err(|e| map_sqlx_error("list_by_property_key", e))?;

        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }

    async fn list_distinct_keys(
        &self,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<Vec<String>, DomainError> {
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
        .map_err(|e| map_sqlx_error("list_distinct_keys", e))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("key")).collect())
    }

    async fn list_distinct_authors(
        &self,
        prefix: Option<&str>,
    ) -> Result<Vec<String>, DomainError> {
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
        .map_err(|e| map_sqlx_error("list_distinct_authors", e))?;

        Ok(rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("author").ok())
            .filter(|s| !s.is_empty())
            .collect())
    }
}

// ── BulkBlockRepository ───────────────────────────────────────────────────────

#[async_trait]
impl BulkBlockRepository for SqliteBlockRepository {
    async fn get_all(&self) -> Result<Vec<Block>, DomainError> {
        let rows = sqlx::query("SELECT * FROM blocks")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_all", e))?;
        rows.iter()
            .map(|r| BlockRow::from_row(r).and_then(|br| br.to_block()))
            .collect()
    }
}
