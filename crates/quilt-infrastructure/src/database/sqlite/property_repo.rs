//! SQLite implementation of the [`PropertyRepository`] trait.

use async_trait::async_trait;
use sqlx::Row;

use tracing::instrument;

use super::helpers::*;
use crate::database::sqlite::connection::DbPool;
use crate::errors::map_sqlx_error;
use quilt_domain::errors::DomainError;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::properties::types::{Cardinality, ClosedValue, PropertyStatus, PropertyType, ViewContext};
use quilt_domain::repositories::PropertyRepository;
use quilt_domain::value_objects::Uuid;

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

        // ADR-0025: Use from_legacy_fields for the row mapper, then set
        // usage metadata via builders. This ensures the 4 new first-class
        // config fields (visibility, mutability, derived_from, merge_policy)
        // are derived correctly from the legacy flags.
        Ok(PropertyDefinition::from_legacy_fields(
            id,
            row.get::<String, _>("db_ident"),
            row.get::<String, _>("title"),
            property_type,
            view_context,
            public != 0,
            queryable != 0,
            hidden != 0,
            read_only != 0,
        )
        .with_usage(block_count.max(0) as u64, page_count.max(0) as u64)
        .with_seen_at(first_seen_at.map(ts_to_datetime), last_seen_at.map(ts_to_datetime)))
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
        .map_err(|e| map_sqlx_error("load_closed_values", e))?;

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
            .map_err(|e| map_sqlx_error("fetch_one_with_closed_values", e))?;

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
            .map_err(|e| map_sqlx_error("get_by_db_ident", e))?;

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
        .map_err(|e| map_sqlx_error("get_all", e))?;

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
        .map_err(|e| map_sqlx_error("insert property", e))?;

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
            .map_err(|e| map_sqlx_error("insert closed value", e))?;
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
        .map_err(|e| map_sqlx_error("update property", e))?;

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
            .map_err(|e| map_sqlx_error("delete closed values", e))?;

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
            .map_err(|e| map_sqlx_error("insert closed value on update", e))?;
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
            .map_err(|e| map_sqlx_error("delete property", e))?;

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
            .map_err(|e| map_sqlx_error("get_by_db_idents", e))?;

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
        .map_err(|e| map_sqlx_error("search properties", e))?;

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
        .map_err(|e| map_sqlx_error("list_by_usage", e))?;

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
        .map_err(|e| map_sqlx_error("get_co_occurrences", e))?;

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
        .map_err(|e| map_sqlx_error("get_trends", e))?;

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
        .map_err(|e| map_sqlx_error("count_distinct_properties", e))?;

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
        .map_err(|e| map_sqlx_error("count_blocks_with_properties", e))?;

        let count: i64 = row.get("cnt");
        Ok(count as u64)
    }
}
