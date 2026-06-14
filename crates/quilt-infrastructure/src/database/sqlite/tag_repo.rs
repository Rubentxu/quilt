//! SQLite implementation of the [`TagRepository`] trait.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;

use super::helpers::*;
use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::TagRepository;
use quilt_domain::value_objects::Uuid;

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
            .map_err(|e| map_sqlx_error("get_by_page", e))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_pages_with_tag(&self, tag: &str) -> Result<Vec<Uuid>, DomainError> {
        let rows = sqlx::query("SELECT page_id FROM tags WHERE tag = ?")
            .bind(tag)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_pages_with_tag", e))?;

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
            .map_err(|e| map_sqlx_error("add_tag", e))?;
        Ok(())
    }

    async fn remove_tag(&self, page_id: Uuid, tag: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM tags WHERE page_id = ? AND tag = ?")
            .bind(uuid_to_blob(&page_id))
            .bind(tag)
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("remove_tag", e))?;
        Ok(())
    }

    async fn get_all_tags(&self) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query("SELECT DISTINCT tag FROM tags ORDER BY tag")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("get_all_tags", e))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }

    async fn get_tag_counts(&self) -> Result<Vec<(String, usize)>, DomainError> {
        let rows =
            sqlx::query("SELECT tag, COUNT(*) as cnt FROM tags GROUP BY tag ORDER BY cnt DESC")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| map_sqlx_error("get_tag_counts", e))?;

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
                .map_err(|e| map_sqlx_error("search_tags", e))?;

        Ok(rows.iter().map(|r| r.get::<String, _>("tag")).collect())
    }
}
