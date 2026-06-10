//! PropertyService — application-layer orchestration for property definitions.
//!
//! Provides CRUD, batch retrieval, search, usage-sorted listing,
//! and fuzzy suggestion (PI-4) via the `PropertyRepository` trait.

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::analytics::{
    AnalyticsParams, PropertyAnalytics,
};
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::properties::types::PropertyStatus;
use quilt_domain::repositories::PropertyRepository;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::instrument;

/// A single suggestion result for property discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySuggestion {
    /// The canonical key.
    pub key: String,
    /// Display title.
    pub title: String,
    /// Property type (Text, Number, etc).
    pub property_type: String,
    /// How many blocks use this property (relevance signal).
    pub usage_count: u64,
    /// Lifecycle status.
    pub status: String,
}

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

    /// Suggest properties matching a partial input (PI-4 discovery).
    ///
    /// Returns candidates sorted by:
    /// 1. Exact prefix match on key (highest priority)
    /// 2. Substring match on key or title
    /// 3. Usage count (block_count) as tiebreaker
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

    // ── PI-6: Lifecycle management ──

    /// Deprecate a property — marks it as Deprecated.
    /// Existing blocks still carry the value; writes should warn.
    async fn deprecate(
        &self,
        key: &str,
    ) -> Result<PropertyDefinition, ApplicationError>;

    /// Merge source_key into target_key — source becomes Merged,
    /// blocks with source key get migrated to target key.
    async fn merge(
        &self,
        source_key: &str,
        target_key: &str,
    ) -> Result<PropertyDefinition, ApplicationError>;

    /// Create an alias — new_key transparently redirects to target_key.
    async fn alias(
        &self,
        new_key: &str,
        target_key: &str,
    ) -> Result<PropertyDefinition, ApplicationError>;
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

    #[instrument(skip(self))]
    async fn suggest(
        &self,
        partial: &str,
        limit: usize,
    ) -> Result<Vec<PropertySuggestion>, ApplicationError> {
        let partial_lower = partial.to_lowercase();

        // Use search to get candidates, then score and sort
        let all = self.repo.search(&partial_lower, limit * 3).await.map_err(ApplicationError::Domain)?;

        let mut scored: Vec<(PropertySuggestion, i32)> = all
            .into_iter()
            .filter(|d| d.is_active())
            .map(|d| {
                let key_lower = d.db_ident.to_lowercase();
                let title_lower = d.title.to_lowercase();

                // Scoring: prefix match > contains > usage-based tiebreak
                let score = if key_lower == partial_lower {
                    1000
                } else if key_lower.starts_with(&partial_lower) {
                    500
                } else if title_lower.starts_with(&partial_lower) {
                    400
                } else if key_lower.contains(&partial_lower) {
                    200
                } else if title_lower.contains(&partial_lower) {
                    100
                } else {
                    0
                } + (d.block_count.min(100) as i32); // usage bonus, capped

                let suggestion = PropertySuggestion {
                    key: d.db_ident.clone(),
                    title: d.title.clone(),
                    property_type: d.property_type.to_string(),
                    usage_count: d.block_count,
                    status: d.status.to_string(),
                };
                (suggestion, score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(limit);

        Ok(scored.into_iter().map(|(s, _)| s).collect())
    }

    #[instrument(skip(self))]
    async fn analytics(
        &self,
        params: &AnalyticsParams,
    ) -> Result<PropertyAnalytics, ApplicationError> {
        let co_occurrences = self
            .repo
            .get_co_occurrences(params.co_occurrence_limit)
            .await
            .map_err(ApplicationError::Domain)?;

        let trends = self
            .repo
            .get_trends(params.trend_period_days, params.trend_limit)
            .await
            .map_err(ApplicationError::Domain)?;

        let total_properties = self
            .repo
            .count_distinct_properties()
            .await
            .map_err(ApplicationError::Domain)?;

        let total_blocks_with_properties = self
            .repo
            .count_blocks_with_properties()
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(PropertyAnalytics {
            co_occurrences,
            trends,
            total_properties,
            total_blocks_with_properties,
        })
    }

    #[instrument(skip(self))]
    async fn deprecate(
        &self,
        key: &str,
    ) -> Result<PropertyDefinition, ApplicationError> {
        let mut def = self
            .repo
            .get_by_db_ident(key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| ApplicationError::Validation(format!("Property '{}' not found", key)))?;

        def.status = PropertyStatus::Deprecated;
        self.repo
            .update(&def)
            .await
            .map_err(ApplicationError::Domain)?;
        Ok(def)
    }

    #[instrument(skip(self))]
    async fn merge(
        &self,
        source_key: &str,
        target_key: &str,
    ) -> Result<PropertyDefinition, ApplicationError> {
        let mut source = self
            .repo
            .get_by_db_ident(source_key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Validation(format!("Source property '{}' not found", source_key))
            })?;

        // Verify target exists
        let _target = self
            .repo
            .get_by_db_ident(target_key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Validation(format!(
                    "Target property '{}' not found",
                    target_key
                ))
            })?;

        source.status = PropertyStatus::Merged;
        source.alias_of = Some(target_key.to_string());
        self.repo
            .update(&source)
            .await
            .map_err(ApplicationError::Domain)?;

        // Note: actual block property migration is a separate concern
        // (would need a batch update on blocks.properties JSON).
        // The definition-level merge marks the source as redirected.

        Ok(source)
    }

    #[instrument(skip(self))]
    async fn alias(
        &self,
        new_key: &str,
        target_key: &str,
    ) -> Result<PropertyDefinition, ApplicationError> {
        // Verify target exists
        let _target = self
            .repo
            .get_by_db_ident(target_key)
            .await
            .map_err(ApplicationError::Domain)?
            .ok_or_else(|| {
                ApplicationError::Validation(format!(
                    "Target property '{}' not found",
                    target_key
                ))
            })?;

        // Check new_key doesn't already exist
        if self
            .repo
            .get_by_db_ident(new_key)
            .await
            .map_err(ApplicationError::Domain)?
            .is_some()
        {
            return Err(ApplicationError::Validation(format!(
                "Property '{}' already exists",
                new_key
            )));
        }

        let alias_def = PropertyDefinition::new(
            quilt_domain::value_objects::Uuid::new_v4(),
            new_key.to_string(),
            format!("Alias → {}", target_key),
            quilt_domain::properties::types::PropertyType::Text,
        )
        .with_status(PropertyStatus::Alias)
        .with_alias_of(target_key.to_string());

        self.repo
            .insert(&alias_def)
            .await
            .map_err(ApplicationError::Domain)?;

        Ok(alias_def)
    }
}
