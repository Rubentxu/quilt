//! SQLite implementation of the [`PageRepository`] trait.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

use super::helpers::*;
use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::entities::Page;
use quilt_domain::errors::DomainError;
use quilt_domain::properties::entry::{DefaultPropertyEntry, HasValue};
use quilt_domain::value_objects::PropertyValue;
use quilt_domain::repositories::PageRepository;
use quilt_domain::repositories::PageRepositoryExt;
use quilt_domain::repositories::PropertyRepository;
use quilt_domain::value_objects::{JournalDay, Uuid};

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
            .map_err(|e| map_sqlx_error("get_by_id", e))?;

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
            .map_err(|e| map_sqlx_error("get_by_name", e))?;

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
            .map_err(|e| map_sqlx_error("get_journal", e))?;

        match row {
            Some(r) => Ok(Some(PageRow::from_row(&r)?.to_page()?)),
            None => Ok(None),
        }
    }

    async fn get_all(&self) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_all", e))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_namespace_pages(&self, namespace_id: Uuid) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE namespace_id = ? ORDER BY name")
            .bind(uuid_to_blob(&namespace_id))
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_namespace_pages", e))?;

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
        .map_err(|e| map_sqlx_error("insert page", e))?;

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
        .map_err(|e| map_sqlx_error("update page", e))?;

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
            .map_err(|e| map_sqlx_error("rename page", e))?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("delete page", e))?;
        Ok(())
    }

    async fn get_updated_since(&self, since: DateTime<Utc>) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages WHERE updated_at > ? ORDER BY updated_at DESC")
            .bind(since.timestamp())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_updated_since", e))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn get_recent(&self, limit: usize) -> Result<Vec<Page>, DomainError> {
        let rows = sqlx::query("SELECT * FROM pages ORDER BY updated_at DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_recent", e))?;

        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }

    async fn count(&self) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("count", e))?;

        Ok(count as usize)
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Page>, DomainError> {
        let like = format!("%{}%", query);
        let rows = sqlx::query("SELECT * FROM pages WHERE name LIKE ? ORDER BY name LIMIT ?")
            .bind(&like)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("search", e))?;

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
                return Err(DomainError::PropertyReadOnly(key.to_string()));
            }
        }

        // 2. Load page, merge, persist.
        let row = sqlx::query("SELECT * FROM pages WHERE id = ?")
            .bind(uuid_to_blob(&page_id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("update_properties load", e))?;

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
            .map_err(|e| map_sqlx_error("update_properties save", e))?;

        Ok(page)
    }
}

#[async_trait]
impl PageRepositoryExt for SqlitePageRepository {
    async fn search_by_name_or_title(
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
        .map_err(|e| map_sqlx_error("search_by_name_or_title", e))?;
        rows.iter()
            .map(|r| PageRow::from_row(r)?.to_page())
            .collect()
    }
}
