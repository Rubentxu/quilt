//! Relation use cases (PI-8 — semantic property relations).
//!
//! Thin glue over [`RelationRepository`] that:
//! - Parses the wire `relation_type` string into the domain enum
//!   (`precedes` / `broadens` / `implies` / `requires` / `custom:<name>`).
//! - Generates the relation UUID on the application boundary.
//! - Returns the persisted entity to the caller.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::relation::{PropertyRelation, RelationType};
use quilt_domain::repositories::RelationRepository;
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Use cases for semantic property relations (PI-8).
#[async_trait]
pub trait RelationUseCases: Send + Sync {
    /// List every relation in the graph.
    async fn list_all(&self) -> Result<Vec<PropertyRelation>, ApplicationError>;

    /// Get a single relation by id. Returns `None` when not found.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyRelation>, ApplicationError>;

    /// Get every relation whose `source_key` matches `key`.
    async fn get_by_key(&self, key: &str) -> Result<Vec<PropertyRelation>, ApplicationError>;

    /// Get every relation whose `source_key` AND `source_value` match.
    async fn get_from(
        &self,
        key: &str,
        value: &str,
    ) -> Result<Vec<PropertyRelation>, ApplicationError>;

    /// Create a new relation from the wire form. The relation UUID is
    /// generated here so the HTTP layer never has to think about
    /// domain types.
    async fn create(
        &self,
        source_key: String,
        source_value: String,
        target_key: String,
        target_value: String,
        relation_type: RelationType,
        description: String,
        confidence: f64,
    ) -> Result<PropertyRelation, ApplicationError>;

    /// Delete a relation by id. Idempotent — the underlying repo
    /// returns `Ok(())` whether or not the row existed.
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;
}

/// Parse a wire relation-type string. Mirrors the prior handler logic
/// — the wire vocabulary is open-ended, so anything not matching the
/// four built-ins becomes `Custom(<name>)`.
pub fn parse_relation_type(s: &str) -> RelationType {
    match s {
        "precedes" => RelationType::Precedes,
        "broadens" => RelationType::Broadens,
        "implies" => RelationType::Implies,
        "requires" => RelationType::Requires,
        other => RelationType::Custom(other.to_string()),
    }
}

/// Implementation of [`RelationUseCases`] for any [`RelationRepository`].
pub struct RelationUseCasesImpl<R: RelationRepository> {
    repo: Arc<R>,
}

impl<R: RelationRepository> RelationUseCasesImpl<R> {
    /// Create a new use-case instance.
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: RelationRepository + 'static> RelationUseCases for RelationUseCasesImpl<R> {
    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<PropertyRelation>, ApplicationError> {
        self.repo.list_all().await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertyRelation>, ApplicationError> {
        self.repo.get_by_id(id).await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn get_by_key(&self, key: &str) -> Result<Vec<PropertyRelation>, ApplicationError> {
        self.repo.get_by_key(key).await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn get_from(
        &self,
        key: &str,
        value: &str,
    ) -> Result<Vec<PropertyRelation>, ApplicationError> {
        self.repo
            .get_from(key, value)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn create(
        &self,
        source_key: String,
        source_value: String,
        target_key: String,
        target_value: String,
        relation_type: RelationType,
        description: String,
        confidence: f64,
    ) -> Result<PropertyRelation, ApplicationError> {
        let relation = PropertyRelation::new(
            Uuid::new_v4(),
            source_key,
            source_value,
            target_key,
            target_value,
            relation_type,
            description,
            confidence,
        );

        self.repo
            .insert(&relation)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(relation)
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        self.repo.delete(id).await.map_err(ApplicationError::Domain)
    }
}
