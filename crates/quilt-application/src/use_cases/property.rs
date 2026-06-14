//! Property use cases — PI-3, PI-4, PI-5, PI-6
//!
//! Thin glue over [`PropertyRepository`] that adds:
//! - Input validation and parsing
//! - Business logic orchestration (merge, alias, deprecate)
//! - Cross-cutting concerns (analytics)

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::analytics::{AnalyticsParams, PropertyAnalytics};
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::repositories::PropertyRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// Property use cases trait — PI-3 (CRUD), PI-4 (suggestions),
/// PI-5 (analytics), PI-6 (lifecycle).
///
/// Object-safe (`Send + Sync`) and uses `#[async_trait]`.
#[async_trait]
pub trait PropertyUseCases: Send + Sync {
    /// List distinct top-level property keys with pagination.
    async fn list_keys(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, ApplicationError>;

    /// Batch get property definitions by keys.
    async fn batch_get(&self, keys: &[String]) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// Search properties by substring match on key or title.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// List all property definitions sorted by usage (block_count desc).
    async fn list_by_usage(&self, limit: usize) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// List all property definitions.
    async fn list_all(&self) -> Result<Vec<PropertyDefinition>, ApplicationError>;

    /// Create or update a property definition.
    async fn upsert(&self, def: &PropertyDefinition) -> Result<(), ApplicationError>;

    /// Delete a property definition by ID.
    async fn delete(&self, id: quilt_domain::value_objects::Uuid) -> Result<(), ApplicationError>;

    /// Suggest properties matching a partial input (PI-4 discovery).
    async fn suggest(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<PropertySuggestion>, ApplicationError>;

    /// Get property analytics: co-occurrence, trends, totals (PI-5).
    async fn analytics(
        &self,
        params: &AnalyticsParams,
    ) -> Result<PropertyAnalytics, ApplicationError>;

    /// Deprecate a property — marks it as Deprecated (PI-6).
    async fn deprecate(&self, key: &str) -> Result<PropertyDefinition, ApplicationError>;

    /// Merge source_key into target_key — source becomes Merged (PI-6).
    async fn merge(&self, source_key: &str, target_key: &str) -> Result<PropertyDefinition, ApplicationError>;

    /// Create an alias — new_key transparently redirects to target_key (PI-6).
    async fn alias(&self, new_key: &str, target_key: &str) -> Result<PropertyDefinition, ApplicationError>;
}

/// A single suggestion result for property discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySuggestion {
    pub key: String,
    pub title: String,
    pub property_type: String,
    pub usage_count: u64,
    pub status: String,
}

/// Implementation of [`PropertyUseCases`] for any [`PropertyRepository`].
pub struct PropertyUseCasesImpl<R: PropertyRepository> {
    repo: Arc<R>,
}

impl<R: PropertyRepository> PropertyUseCasesImpl<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R: PropertyRepository + 'static> PropertyUseCases for PropertyUseCasesImpl<R> {
    #[instrument(skip(self))]
    async fn list_keys(
        &self,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, ApplicationError> {
        self.repo
            .list_distinct_keys(cursor, limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn batch_get(&self, keys: &[String]) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        let key_refs: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        self.repo
            .get_by_db_idents(&key_refs)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo
            .search(query, limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_by_usage(&self, limit: usize) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo
            .list_by_usage(limit)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<PropertyDefinition>, ApplicationError> {
        self.repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn upsert(&self, def: &PropertyDefinition) -> Result<(), ApplicationError> {
        self.repo
            .insert(def)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: quilt_domain::value_objects::Uuid) -> Result<(), ApplicationError> {
        self.repo.delete(id).await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn suggest(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<PropertySuggestion>, ApplicationError> {
        let all = self.repo.get_all().await.map_err(ApplicationError::Domain)?;
        let all_sorted = {
            let mut v: Vec<_> = all;
            v.sort_by(|a, b| b.block_count.cmp(&a.block_count));
            v
        };

        let partial_lower = partial.to_lowercase();
        let mut suggestions: Vec<PropertySuggestion> = all_sorted
            .into_iter()
            .filter(|def| {
                let key_lower = def.key.to_lowercase();
                key_lower.contains(&partial_lower) || def.title.to_lowercase().contains(&partial_lower)
            })
            .take(limit)
            .map(|def| PropertySuggestion {
                key: def.key,
                title: def.title,
                property_type: format!("{:?}", def.property_type),
                usage_count: def.block_count,
                status: format!("{:?}", def.status),
            })
            .collect();

        suggestions.sort_by(|a, b| {
            let a_prefix = a.key.to_lowercase().starts_with(&partial_lower);
            let b_prefix = b.key.to_lowercase().starts_with(&partial_lower);
            match (a_prefix, b_prefix) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.usage_count.cmp(&a.usage_count),
            }
        });

        Ok(suggestions)
    }

    #[instrument(skip(self))]
    async fn analytics(
        &self,
        params: &AnalyticsParams,
    ) -> Result<PropertyAnalytics, ApplicationError> {
        let co_occurrences = self
            .repo
            .get_co_occurrences(params.limit)
            .await
            .map_err(ApplicationError::Domain)?;

        let trends = self
            .repo
            .get_trends(params.period_days, params.limit)
            .await
            .map_err(ApplicationError::Domain)?;

        let total_properties = self
            .repo
            .count_distinct_properties()
            .await
            .map_err(ApplicationError::Domain)?;

        let blocks_with_properties = self
            .repo
            .count_blocks_with_properties()
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(PropertyAnalytics {
            co_occurrences,
            trends,
            total_properties,
            blocks_with_properties,
        })
    }

    #[instrument(skip(self))]
    async fn deprecate(&self, key: &str) -> Result<PropertyDefinition, ApplicationError> {
        let mut def = self
            .repo
            .get_by_db_ident(key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Property", quilt_domain::value_objects::Uuid::nil()))?;

        def.status = quilt_domain::properties::types::PropertyStatus::Deprecated;
        self.repo
            .update(&def)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(def)
    }

    #[instrument(skip(self))]
    async fn merge(&self, source_key: &str, target_key: &str) -> Result<PropertyDefinition, ApplicationError> {
        let mut source = self
            .repo
            .get_by_db_ident(source_key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Property", quilt_domain::value_objects::Uuid::nil()))?;

        source.status = quilt_domain::properties::types::PropertyStatus::Merged;
        source.merged_into = Some(target_key.to_string());
        self.repo
            .update(&source)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(source)
    }

    #[instrument(skip(self))]
    async fn alias(&self, new_key: &str, target_key: &str) -> Result<PropertyDefinition, ApplicationError> {
        let target = self
            .repo
            .get_by_db_ident(target_key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::NotFound("Property", quilt_domain::value_objects::Uuid::nil()))?;

        let alias_def = PropertyDefinition {
            key: new_key.to_string(),
            title: format!("Alias of {}", target.title),
            description: format!("Alias that redirects to {}", target_key),
            property_type: target.property_type.clone(),
            status: quilt_domain::properties::types::PropertyStatus::Active,
            cardinality: target.cardinality.clone(),
            allowed_values: target.allowed_values.clone(),
            default_value: target.default_value.clone(),
            block_count: 0,
            merged_into: Some(target_key.to_string()),
            source: None,
            schema_id: target.schema_id,
            aliases: vec![],
            views: target.views.clone(),
        };

        self.repo
            .insert(&alias_def)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(alias_def)
    }
}
