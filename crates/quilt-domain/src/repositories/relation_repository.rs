//! RelationRepository trait — persistence for semantic property relations.

use crate::errors::DomainError;
use crate::properties::relation::PropertyRelation;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// Repository for semantic property relations.
#[async_trait]
pub trait RelationRepository: Send + Sync {
    /// Get a relation by ID.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyRelation>, DomainError>;

    /// Get all relations for a given property key.
    async fn get_by_key(&self, key: &str) -> Result<Vec<PropertyRelation>, DomainError>;

    /// Get all relations where a specific key+value is the source.
    async fn get_from(&self, key: &str, value: &str) -> Result<Vec<PropertyRelation>, DomainError>;

    /// Get all relations.
    async fn list_all(&self) -> Result<Vec<PropertyRelation>, DomainError>;

    /// Insert a new relation.
    async fn insert(&self, relation: &PropertyRelation) -> Result<(), DomainError>;

    /// Delete a relation by ID.
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
