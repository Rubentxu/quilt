//! SchemaRepository trait — persistence for property schema templates.

use crate::errors::DomainError;
use crate::properties::schema::PropertySchema;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// Repository for property schema templates.
#[async_trait]
pub trait SchemaRepository: Send + Sync {
    /// Get a schema by ID.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertySchema>, DomainError>;

    /// Get a schema by name.
    async fn get_by_name(&self, name: &str) -> Result<Option<PropertySchema>, DomainError>;

    /// List all schemas.
    async fn list_all(&self) -> Result<Vec<PropertySchema>, DomainError>;

    /// List auto-detected schemas only.
    async fn list_auto_detected(&self) -> Result<Vec<PropertySchema>, DomainError>;

    /// Insert a new schema.
    async fn insert(&self, schema: &PropertySchema) -> Result<(), DomainError>;

    /// Update an existing schema.
    async fn update(&self, schema: &PropertySchema) -> Result<(), DomainError>;

    /// Delete a schema by ID.
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}

#[cfg(test)]
mod tests {
    // Integration tests in quilt-infrastructure
}
