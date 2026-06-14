//! SQLite implementation of the SchemaRepository trait.

use sqlx::Row;

use super::helpers::*;
use crate::errors::{map_sqlx_error, map_storage_error};

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
        let id_bytes: Vec<u8> = row.try_get("id").map_err(|e| map_storage_error(format!("schema id: {}", e)))?;
        let id = blob_to_uuid(&id_bytes)?;

        let name: String = row.try_get("name").map_err(|e| map_storage_error(format!("schema name: {}", e)))?;
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
            .map_err(|e| map_sqlx_error("schema get_by_id", e))?;

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
            .map_err(|e| map_sqlx_error("schema get_by_name", e))?;

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
            .map_err(|e| map_sqlx_error("schema list_all", e))?;

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
            .map_err(|e| map_sqlx_error("schema list_auto_detected", e))?;

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
            .map_err(|e| map_storage_error(format!("serialize keys: {}", e)))?;

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
        .map_err(|e| map_sqlx_error("schema insert", e))?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn update(
        &self,
        schema: &quilt_domain::properties::schema::PropertySchema,
    ) -> Result<(), quilt_domain::DomainError> {
        let keys_json = serde_json::to_string(&schema.property_keys)
            .map_err(|e| map_storage_error(format!("serialize keys: {}", e)))?;

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
        .map_err(|e| map_sqlx_error("schema update", e))?;

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
            .map_err(|e| map_sqlx_error("schema delete", e))?;
        Ok(())
    }
}
