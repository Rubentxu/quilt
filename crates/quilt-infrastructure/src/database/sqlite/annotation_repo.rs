//! SQLite implementation of [`AnnotationRepository`].
//!
//! Persists [`Annotation`] entities to the `annotations` table created
//! by migration 008 (`migrations/008_annotations.sql`). The full SQL
//! schema is also inlined in
//! `crate::database::sqlite::connection::run_migrations` for
//! idempotent re-runs.
//!
//! # Storage format
//!
//! - UUIDs are stored as 16-byte BLOB
//! - Timestamps are i64 epoch seconds (consistent with the `blocks`
//!   table and `datetime_to_ts` in `repositories.rs`)
//! - Enum columns are stored as their `as_str()` lowercase form
//! - Optional offsets (`highlight_start`, `highlight_end`,
//!   `parent_annotation_id`, `resolved_at`, `resolved_by`) are
//!   nullable columns; `None` is stored as SQL `NULL`.
//!
//! # Row mapping
//!
//! `AnnotationRow` is a private intermediate struct that holds the raw
//! row columns. `from_row` parses SQLite row → `AnnotationRow`,
//! `to_annotation` converts the row → domain `Annotation`. The
//! separation keeps the SQL-specific parsing isolated from the domain
//! entity.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::{QueryBuilder, Row, Sqlite};
use tracing::instrument;

use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::entities::{Annotation, AnnotationScope, AnnotationStatus, AuthorType};
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::{AnnotationFilters, AnnotationRepository};
use quilt_domain::value_objects::Uuid;

// ── Row representation ─────────────────────────────────────────────────

/// Raw row data from the `annotations` table.
struct AnnotationRow {
    id: Vec<u8>,
    block_id: Vec<u8>,
    scope: String,
    author_type: String,
    author_name: String,
    content: String,
    status: String,
    highlight_start: Option<i64>,
    highlight_end: Option<i64>,
    parent_annotation_id: Option<Vec<u8>>,
    created_at: i64,
    resolved_at: Option<i64>,
    resolved_by: Option<String>,
}

impl AnnotationRow {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, DomainError> {
        Ok(Self {
            id: row.get("id"),
            block_id: row.get("block_id"),
            scope: row.get("scope"),
            author_type: row.get("author_type"),
            author_name: row.get("author_name"),
            content: row.get("content"),
            status: row.get("status"),
            highlight_start: row.get("highlight_start"),
            highlight_end: row.get("highlight_end"),
            parent_annotation_id: row.get("parent_annotation_id"),
            created_at: row.get("created_at"),
            resolved_at: row.get("resolved_at"),
            resolved_by: row.get("resolved_by"),
        })
    }

    fn to_annotation(&self) -> Result<Annotation, DomainError> {
        // Bypassing `Annotation::new()` on read: that constructor
        // generates a fresh v4 id and stamps `created_at = now()`, both
        // of which would destroy the row's actual values. We construct
        // the entity directly, having already validated the row's
        // column values (via `try_from_str` and `bytes_to_uuid`).
        let scope = AnnotationScope::try_from_str(&self.scope).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown annotation scope: {}", self.scope))
        })?;
        let author_type = AuthorType::try_from_str(&self.author_type).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown author_type: {}", self.author_type))
        })?;
        let status = AnnotationStatus::try_from_str(&self.status).ok_or_else(|| {
            DomainError::InvalidData(format!("Unknown annotation status: {}", self.status))
        })?;
        let id = bytes_to_uuid(&self.id)?;
        let block_id = bytes_to_uuid(&self.block_id)?;
        let parent = match self.parent_annotation_id.as_deref() {
            Some(b) if !b.is_empty() => Some(bytes_to_uuid(b)?),
            _ => None,
        };
        let highlight_start = self.highlight_start.map(|n| n as u32);
        let highlight_end = self.highlight_end.map(|n| n as u32);
        let created_at = ts_to_datetime(self.created_at);
        let resolved_at = self.resolved_at.map(ts_to_datetime);
        Ok(Annotation {
            id,
            block_id,
            scope,
            author_type,
            author_name: self.author_name.clone(),
            content: self.content.clone(),
            status,
            highlight_start,
            highlight_end,
            parent_annotation_id: parent,
            created_at,
            resolved_at,
            resolved_by: self.resolved_by.clone(),
        })
    }
}

// ── Helpers (private to this module) ────────────────────────────────────

fn bytes_to_uuid(blob: &[u8]) -> Result<Uuid, DomainError> {
    let bytes: [u8; 16] = blob.try_into().map_err(|_| {
        DomainError::InvalidData(format!("Invalid UUID blob length: {}", blob.len()))
    })?;
    Ok(Uuid::from_bytes(bytes))
}

fn uuid_to_blob(id: &Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn ts_to_datetime(ts: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| chrono::DateTime::<Utc>::from_timestamp(ts, 0).unwrap_or_else(Utc::now))
}

fn datetime_to_ts(dt: &chrono::DateTime<Utc>) -> i64 {
    dt.timestamp()
}

fn scope_to_str(s: &AnnotationScope) -> &'static str {
    s.as_str()
}

fn author_type_to_str(a: &AuthorType) -> &'static str {
    a.as_str()
}

fn status_to_str(s: &AnnotationStatus) -> &'static str {
    s.as_str()
}

fn rows_to_annotations(rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Vec<Annotation>, DomainError> {
    rows.iter()
        .map(|r| AnnotationRow::from_row(r)?.to_annotation())
        .collect()
}

// ── SqliteAnnotationRepository ─────────────────────────────────────────

/// SQLite-backed [`AnnotationRepository`].
///
/// Persists annotations to the `annotations` table. Use
/// [`SqliteAnnotationRepository::new`] to construct from a [`DbPool`].
pub struct SqliteAnnotationRepository {
    pool: DbPool,
}

impl SqliteAnnotationRepository {
    /// Create a new repository over the given pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnnotationRepository for SqliteAnnotationRepository {
    #[instrument(skip(self, annotation), fields(annotation_id = %annotation.id))]
    async fn insert(&self, annotation: &Annotation) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO annotations
            (id, block_id, scope, author_type, author_name, content, status,
             highlight_start, highlight_end, parent_annotation_id,
             created_at, resolved_at, resolved_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&annotation.id))
        .bind(uuid_to_blob(&annotation.block_id))
        .bind(scope_to_str(&annotation.scope))
        .bind(author_type_to_str(&annotation.author_type))
        .bind(&annotation.author_name)
        .bind(&annotation.content)
        .bind(status_to_str(&annotation.status))
        .bind(annotation.highlight_start.map(|n| n as i64))
        .bind(annotation.highlight_end.map(|n| n as i64))
        .bind(annotation.parent_annotation_id.as_ref().map(uuid_to_blob))
        .bind(datetime_to_ts(&annotation.created_at))
        .bind(annotation.resolved_at.as_ref().map(datetime_to_ts))
        .bind(annotation.resolved_by.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("insert annotation", e))?;
        Ok(())
    }

    #[instrument(skip(self, annotation), fields(annotation_id = %annotation.id))]
    async fn update(&self, annotation: &Annotation) -> Result<(), DomainError> {
        sqlx::query(
            r#"UPDATE annotations SET
            block_id = ?, scope = ?, author_type = ?, author_name = ?,
            content = ?, status = ?, highlight_start = ?, highlight_end = ?,
            parent_annotation_id = ?, created_at = ?, resolved_at = ?, resolved_by = ?
            WHERE id = ?"#,
        )
        .bind(uuid_to_blob(&annotation.block_id))
        .bind(scope_to_str(&annotation.scope))
        .bind(author_type_to_str(&annotation.author_type))
        .bind(&annotation.author_name)
        .bind(&annotation.content)
        .bind(status_to_str(&annotation.status))
        .bind(annotation.highlight_start.map(|n| n as i64))
        .bind(annotation.highlight_end.map(|n| n as i64))
        .bind(annotation.parent_annotation_id.as_ref().map(uuid_to_blob))
        .bind(datetime_to_ts(&annotation.created_at))
        .bind(annotation.resolved_at.as_ref().map(datetime_to_ts))
        .bind(annotation.resolved_by.as_deref())
        .bind(uuid_to_blob(&annotation.id))
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("update annotation", e))?;
        Ok(())
    }

    #[instrument(skip(self), fields(annotation_id = %id))]
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM annotations WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("delete annotation", e))?;
        Ok(())
    }

    #[instrument(skip(self), fields(annotation_id = %id))]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Annotation>, DomainError> {
        let row = sqlx::query("SELECT * FROM annotations WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_by_id", e))?;
        match row {
            Some(r) => Ok(Some(AnnotationRow::from_row(&r)?.to_annotation()?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(block_id = %block_id))]
    async fn get_by_block(&self, block_id: Uuid) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM annotations WHERE block_id = ? ORDER BY created_at ASC, id ASC",
        )
        .bind(uuid_to_blob(&block_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_by_block", e))?;
        rows_to_annotations(rows)
    }

    #[instrument(skip(self, author_name))]
    async fn get_by_author(&self, author_name: &str) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM annotations WHERE author_name = ? ORDER BY created_at DESC, id DESC",
        )
        .bind(author_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_by_author", e))?;
        rows_to_annotations(rows)
    }

    #[instrument(skip(self, status))]
    async fn get_by_status(&self, status: &str) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM annotations WHERE status = ? ORDER BY created_at DESC, id DESC",
        )
        .bind(status)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_by_status", e))?;
        rows_to_annotations(rows)
    }

    #[instrument(skip(self))]
    async fn get_root_annotations(&self) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM annotations WHERE parent_annotation_id IS NULL \
             ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_root_annotations", e))?;
        rows_to_annotations(rows)
    }

    #[instrument(skip(self), fields(parent_id = %parent_id))]
    async fn get_thread_replies(&self, parent_id: Uuid) -> Result<Vec<Annotation>, DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM annotations WHERE parent_annotation_id = ? \
             ORDER BY created_at ASC, id ASC",
        )
        .bind(uuid_to_blob(&parent_id))
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("get_thread_replies", e))?;
        rows_to_annotations(rows)
    }

    #[instrument(skip(self, filters))]
    async fn get_by_filters(
        &self,
        filters: &AnnotationFilters,
    ) -> Result<Vec<Annotation>, DomainError> {
        // Use `QueryBuilder` for the dynamic WHERE clause. Each filter
        // is a separate `push` and bind; missing filters contribute
        // no clause.
        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new("SELECT * FROM annotations WHERE 1=1");
        if let Some(block_id) = filters.block_id {
            qb.push(" AND block_id = ")
                .push_bind(uuid_to_blob(&block_id));
        }
        if let Some(ref status) = filters.status {
            qb.push(" AND status = ").push_bind(status.clone());
        }
        if let Some(ref scope) = filters.scope {
            qb.push(" AND scope = ")
                .push_bind(scope_to_str(scope).to_string());
        }
        if let Some(ref author_name) = filters.author_name {
            qb.push(" AND author_name = ")
                .push_bind(author_name.clone());
        }
        qb.push(" ORDER BY created_at DESC, id DESC");

        let rows = qb
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_by_filters", e))?;
        rows_to_annotations(rows)
    }
}
