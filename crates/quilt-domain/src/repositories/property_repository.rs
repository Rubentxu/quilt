//! PropertyRepository trait - abstraction for property persistence

use crate::errors::DomainError;
use crate::properties::analytics::{PropertyCoOccurrence, PropertyTrend};
use crate::properties::definition::PropertyDefinition;
use crate::properties::types::ClosedValue;
use crate::value_objects::Uuid;
use async_trait::async_trait;
use std::sync::Arc;

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

    // ── PI-5: Analytics methods ──

    /// Compute co-occurrence counts for properties that appear together on blocks.
    ///
    /// Returns raw pairs sorted by co-occurrence count descending.
    /// PMI calculation is done in the application layer.
    async fn get_co_occurrences(
        &self,
        limit: usize,
    ) -> Result<Vec<PropertyCoOccurrence>, DomainError>;

    /// Get property usage trends comparing current vs previous period.
    ///
    /// The implementation counts blocks with each property key in two
    /// time windows: [now - period_days, now] and [now - 2*period_days, now - period_days].
    async fn get_trends(
        &self,
        period_days: u32,
        limit: usize,
    ) -> Result<Vec<PropertyTrend>, DomainError>;

    /// Count total distinct property keys in use across all blocks.
    async fn count_distinct_properties(&self) -> Result<u64, DomainError>;

    /// Count blocks that have at least one property.
    async fn count_blocks_with_properties(&self) -> Result<u64, DomainError>;
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

/// Blanket impl so Arc<dyn PropertyRepository> can be used as PropertyRepository.
/// This enables dynamic dispatch with PropertyKeyResolver.
#[async_trait]
impl<T: PropertyRepository + ?Sized> PropertyRepository for Arc<T> {
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyDefinition>, DomainError> {
        self.as_ref().get_by_id(id).await
    }

    async fn get_by_db_ident(&self, ident: &str) -> Result<Option<PropertyDefinition>, DomainError> {
        self.as_ref().get_by_db_ident(ident).await
    }

    async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
        self.as_ref().get_all().await
    }

    async fn insert(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        self.as_ref().insert(def).await
    }

    async fn update(&self, def: &PropertyDefinition) -> Result<(), DomainError> {
        self.as_ref().update(def).await
    }

    async fn get_closed_values(&self, property_id: Uuid) -> Result<Vec<ClosedValue>, DomainError> {
        self.as_ref().get_closed_values(property_id).await
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.as_ref().delete(id).await
    }

    async fn get_by_db_idents(&self, idents: &[&str]) -> Result<Vec<PropertyDefinition>, DomainError> {
        self.as_ref().get_by_db_idents(idents).await
    }

    async fn search(&self, query: &str, limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
        self.as_ref().search(query, limit).await
    }

    async fn list_by_usage(&self, limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
        self.as_ref().list_by_usage(limit).await
    }

    async fn get_co_occurrences(&self, limit: usize) -> Result<Vec<PropertyCoOccurrence>, DomainError> {
        self.as_ref().get_co_occurrences(limit).await
    }

    async fn get_trends(&self, period_days: u32, limit: usize) -> Result<Vec<PropertyTrend>, DomainError> {
        self.as_ref().get_trends(period_days, limit).await
    }

    async fn count_distinct_properties(&self) -> Result<u64, DomainError> {
        self.as_ref().count_distinct_properties().await
    }

    async fn count_blocks_with_properties(&self) -> Result<u64, DomainError> {
        self.as_ref().count_blocks_with_properties().await
    }
}

#[cfg(test)]
mod tests {
    // Trait tests would require a mock implementation
    // Integration tests are in quilt-infrastructure
}
