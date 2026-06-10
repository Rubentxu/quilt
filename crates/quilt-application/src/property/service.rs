//! PropertyService — application-layer orchestration for property definitions.
//!
//! Provides CRUD, batch retrieval, search, and usage-sorted listing.
//! Delegates persistence to a [`PropertyRepository`] implementation.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::repositories::PropertyRepository;
use std::sync::Arc;
use tracing::instrument;

/// Application service for property definitions.
pub struct PropertyService {
    repo: Arc<dyn PropertyRepository>,
}

impl PropertyService {
    /// Create a new PropertyService with the given repository.
    pub fn new(repo: Arc<dyn PropertyRepository>) -> Self {
        Self { repo }
    }
}

/// Trait for property service operations (object-safe for dependency injection).
#[async_trait]
pub trait PropertyServiceTrait: Send + Sync {
    /// Get a property definition by its database identifier.
    async fn get_by_key(&self, key: &str) -> Result<Option<PropertyDefinition>, ApplicationError>;

    /// Get multiple property definitions by their keys.
    async fn batch_get(&self, keys: &[String]) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// Search properties by substring match on key or title.
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// List all property definitions sorted by usage (block_count desc).
    async fn list_by_usage(
        &self,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// List all property definitions.
    async fn list_all(&self) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// Create or update a property definition.
    async fn upsert(&self, def: &PropertyDefinition) -> Result<(), ApplicationError>;

    /// Delete a property definition by ID.
    async fn delete(&self, id: quilt_domain::value_objects::Uuid) -> Result<(), ApplicationError>;
}

#[async_trait]
impl PropertyServiceTrait for PropertyService {
    #[instrument(skip(self))]
    async fn get_by_key(&self, key: &str) -> Result<Option<PropertyDefinition>, ApplicationError> {
        self.repo
            .get_by_db_ident(key)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn batch_get(
        &self,
        keys: &[String],
    ) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        self.repo
            .get_by_db_idents(&key_refs)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo
            .search(query, limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_by_usage(
        &self,
        limit: usize,
    ) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo
            .list_by_usage(limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo.get_all().await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self, def))]
    async fn upsert(&self, def: &PropertyDefinition) -> Result<(), ApplicationError> {
        // Try update first; if not found, insert.
        match self.repo.update(def).await {
            Ok(()) => Ok(()),
            Err(quilt_domain::errors::DomainError::NotFound(_)) => {
                self.repo.insert(def).await.map_err(ApplicationError::Domain)
            }
            Err(e) => Err(ApplicationError::Domain(e)),
        }
    }

    #[instrument(skip(self))]
    async fn delete(
        &self,
        id: quilt_domain::value_objects::Uuid,
    ) -> Result<(), ApplicationError> {
        self.repo
            .delete(id)
            .await
            .map_err(ApplicationError::Domain)
    }
}
