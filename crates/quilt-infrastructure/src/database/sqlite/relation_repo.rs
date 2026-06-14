//! SQLite implementation of the RelationRepository trait.

use sqlx::Row;

use super::helpers::*;
use crate::errors::{map_sqlx_error, map_storage_error};

/// SQLite implementation of the RelationRepository trait.
pub struct SqliteRelationRepository {
    pool: sqlx::SqlitePool,
}

impl SqliteRelationRepository {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    fn row_to_relation(
        &self,
        row: &sqlx::sqlite::SqliteRow,
    ) -> Result<quilt_domain::properties::relation::PropertyRelation, quilt_domain::DomainError> {
        let id_bytes: Vec<u8> = row.try_get("id").map_err(|e| map_storage_error(format!("relation id: {}", e)))?;
        let id = blob_to_uuid(&id_bytes)?;

        let source_key: String = row.try_get("source_key").unwrap_or_default();
        let source_value: String = row.try_get("source_value").unwrap_or_default();
        let target_key: String = row.try_get("target_key").unwrap_or_default();
        let target_value: String = row.try_get("target_value").unwrap_or_default();
        let relation_type_str: String = row.try_get("relation_type").unwrap_or_else(|_| "precedes".to_string());
        let description: String = row.try_get("description").unwrap_or_default();
        let confidence: f64 = row.try_get("confidence").unwrap_or(1.0);
        let created_at: i64 = row.try_get("created_at").unwrap_or(0);

        let relation_type = match relation_type_str.as_str() {
            "precedes" => quilt_domain::properties::relation::RelationType::Precedes,
            "broadens" => quilt_domain::properties::relation::RelationType::Broadens,
            "implies" => quilt_domain::properties::relation::RelationType::Implies,
            "requires" => quilt_domain::properties::relation::RelationType::Requires,
            other => quilt_domain::properties::relation::RelationType::Custom(other.to_string()),
        };

        Ok(quilt_domain::properties::relation::PropertyRelation {
            id,
            source_key,
            source_value,
            target_key,
            target_value,
            relation_type,
            description,
            confidence,
            created_at,
        })
    }
}

#[async_trait::async_trait]
impl quilt_domain::repositories::RelationRepository for SqliteRelationRepository {
    #[tracing::instrument(skip(self))]
    async fn get_by_id(
        &self,
        id: quilt_domain::value_objects::Uuid,
    ) -> Result<Option<quilt_domain::properties::relation::PropertyRelation>, quilt_domain::DomainError> {
        let row = sqlx::query("SELECT * FROM property_relations WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("relation get_by_id", e))?;

        match row {
            Some(r) => Ok(Some(self.row_to_relation(&r)?)),
            None => Ok(None),
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get_by_key(
        &self,
        key: &str,
    ) -> Result<Vec<quilt_domain::properties::relation::PropertyRelation>, quilt_domain::DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM property_relations WHERE source_key = ? OR target_key = ? ORDER BY created_at ASC",
        )
        .bind(key)
        .bind(key)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("relation get_by_key", e))?;

        rows.iter().map(|r| self.row_to_relation(r)).collect()
    }

    #[tracing::instrument(skip(self))]
    async fn get_from(
        &self,
        key: &str,
        value: &str,
    ) -> Result<Vec<quilt_domain::properties::relation::PropertyRelation>, quilt_domain::DomainError> {
        let rows = sqlx::query(
            "SELECT * FROM property_relations WHERE source_key = ? AND source_value = ? ORDER BY created_at ASC",
        )
        .bind(key)
        .bind(value)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("relation get_from", e))?;

        rows.iter().map(|r| self.row_to_relation(r)).collect()
    }

    #[tracing::instrument(skip(self))]
    async fn list_all(
        &self,
    ) -> Result<Vec<quilt_domain::properties::relation::PropertyRelation>, quilt_domain::DomainError> {
        let rows = sqlx::query("SELECT * FROM property_relations ORDER BY source_key, source_value, created_at ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("relation list_all", e))?;

        rows.iter().map(|r| self.row_to_relation(r)).collect()
    }

    #[tracing::instrument(skip(self))]
    async fn insert(
        &self,
        relation: &quilt_domain::properties::relation::PropertyRelation,
    ) -> Result<(), quilt_domain::DomainError> {
        sqlx::query(
            r#"INSERT INTO property_relations (id, source_key, source_value, target_key, target_value, relation_type, description, confidence, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(uuid_to_blob(&relation.id))
        .bind(&relation.source_key)
        .bind(&relation.source_value)
        .bind(&relation.target_key)
        .bind(&relation.target_value)
        .bind(relation.relation_type.as_str())
        .bind(&relation.description)
        .bind(relation.confidence)
        .bind(relation.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| map_sqlx_error("relation insert", e))?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn delete(
        &self,
        id: quilt_domain::value_objects::Uuid,
    ) -> Result<(), quilt_domain::DomainError> {
        sqlx::query("DELETE FROM property_relations WHERE id = ?")
            .bind(uuid_to_blob(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| map_sqlx_error("relation delete", e))?;
        Ok(())
    }
}
