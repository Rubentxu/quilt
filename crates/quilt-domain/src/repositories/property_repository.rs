//! PropertyRepository trait - abstraction for property persistence

use crate::errors::DomainError;
use crate::properties::definition::PropertyDefinition;
use crate::properties::types::ClosedValue;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// PropertyRepository provides access to property definitions.
///
/// This trait defines the contract for persisting and retrieving
/// typed property definitions.
#[async_trait]
pub trait PropertyRepository: Send + Sync {
    /// Get a property definition by ID
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyDefinition>, DomainError>;

    /// Get a property definition by database identifier
    async fn get_by_db_ident(&self, ident: &str)
    -> Result<Option<PropertyDefinition>, DomainError>;

    /// Get all property definitions
    async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError>;

    /// Insert a new property definition
    async fn insert(&self, def: &PropertyDefinition) -> Result<(), DomainError>;

    /// Update an existing property definition
    async fn update(&self, def: &PropertyDefinition) -> Result<(), DomainError>;

    /// Get the closed values for a property
    async fn get_closed_values(&self, property_id: Uuid) -> Result<Vec<ClosedValue>, DomainError>;

    /// Delete a property definition
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    // ── PI-2: Batch & search methods ──

    /// Get multiple property definitions by their database identifiers.
    async fn get_by_db_idents(
        &self,
        idents: &[&str],
    ) -> Result<Vec<PropertyDefinition>, DomainError>;

    /// Search properties by name (case-insensitive, substring match).
    async fn search(&self, query: &str, limit: usize)
    -> Result<Vec<PropertyDefinition>, DomainError>;

    /// Get property definitions sorted by usage (block_count descending).
    async fn list_by_usage(&self, limit: usize)
    -> Result<Vec<PropertyDefinition>, DomainError>;
}

/// PropertyRepositoryExt provides convenience methods built on PropertyRepository.
#[async_trait]
pub trait PropertyRepositoryExt: PropertyRepository {
    /// Check if a property with the given database identifier exists
    async fn exists(&self, db_ident: &str) -> Result<bool, DomainError> {
        Ok(self.get_by_db_ident(db_ident).await?.is_some())
    }

    /// Find a property or return a builtin property if not found in custom definitions
    async fn find_or_builtin(
        &self,
        db_ident: &str,
    ) -> Result<Option<PropertyDefinition>, DomainError> {
        if let Some(def) = self.get_by_db_ident(db_ident).await? {
            return Ok(Some(def));
        }
        // Check builtin properties
        Ok(crate::properties::builtin::get_builtin_property(db_ident).cloned())
    }
}

impl<T: PropertyRepository + ?Sized> PropertyRepositoryExt for T {}

#[cfg(test)]
mod tests {
    // Trait tests would require a mock implementation
    // Integration tests are in quilt-infrastructure
}
