//! SQLite implementation of the [`RefRepository`] trait.

use async_trait::async_trait;
use sqlx::Row;

use super::helpers::*;
use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::errors::DomainError;
use quilt_domain::references::RefType;
use quilt_domain::repositories::{RefRepository, RefRow};
use quilt_domain::value_objects::Uuid;

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
        .map_err(|e| map_sqlx_error("get_forward_refs", e))?;

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
        .map_err(|e| map_sqlx_error("get_backlinks", e))?;

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
            .map_err(|e| map_sqlx_error("begin transaction", e))?;

        // Delete all existing refs for this source
        sqlx::query("DELETE FROM refs WHERE source_id = ?")
            .bind(uuid_to_blob(&source_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error("delete refs", e))?;

        // Insert new refs
        for (target_id, ref_type) in refs {
            sqlx::query("INSERT INTO refs (source_id, target_id, ref_type) VALUES (?, ?, ?)")
                .bind(uuid_to_blob(&source_id))
                .bind(uuid_to_blob(target_id))
                .bind(Self::ref_type_to_str(ref_type))
                .execute(&mut *tx)
                .await
                .map_err(|e| map_sqlx_error("insert ref", e))?;
        }

        tx.commit()
            .await
            .map_err(|e| map_sqlx_error("commit transaction", e))?;

        Ok(())
    }

    async fn rebuild_index(&self) -> Result<Vec<RefRow>, DomainError> {
        let rows = sqlx::query(
            "SELECT source_id, target_id, ref_type, custom_context FROM refs ORDER BY source_id, target_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("rebuild_index", e))?;

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
            .map_err(|e| map_sqlx_error("insert_ref", e))?;
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
        .map_err(|e| map_sqlx_error("get_unlinked_references", e))?;

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
        .map_err(|e| map_sqlx_error("set_custom_context", e))?;

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
        .map_err(|e| map_sqlx_error("get_custom_context", e))?;

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
        .map_err(|e| map_sqlx_error("get_custom_contexts_for_target", e))?;

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
