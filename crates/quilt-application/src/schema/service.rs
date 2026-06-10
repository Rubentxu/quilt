//! SchemaService — application-layer orchestration for property schemas.
//!
//! Provides CRUD for property schema templates and auto-detection
//! of property clusters from co-occurrence analytics (PI-5).

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::analytics::AnalyticsParams;
use quilt_domain::properties::schema::{AutoDetectParams, PropertySchema};
use quilt_domain::repositories::{PropertyRepository, SchemaRepository};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Application service for property schema templates.
pub struct SchemaService {
    schema_repo: Arc<dyn SchemaRepository>,
    property_repo: Arc<dyn PropertyRepository>,
}

impl SchemaService {
    pub fn new(
        schema_repo: Arc<dyn SchemaRepository>,
        property_repo: Arc<dyn PropertyRepository>,
    ) -> Self {
        Self {
            schema_repo,
            property_repo,
        }
    }
}

/// Trait for schema service operations (object-safe).
#[async_trait]
pub trait SchemaServiceTrait: Send + Sync {
    /// Get a schema by ID.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertySchema>, ApplicationError>;

    /// Get a schema by name.
    async fn get_by_name(&self, name: &str) -> Result<Option<PropertySchema>, ApplicationError>;

    /// List all schemas.
    async fn list_all(&self) -> Result<Vec<PropertySchema>, ApplicationError>;

    /// Create a new schema.
    async fn create(&self, schema: &PropertySchema) -> Result<(), ApplicationError>;

    /// Update an existing schema.
    async fn update(&self, schema: &PropertySchema) -> Result<(), ApplicationError>;

    /// Delete a schema.
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;

    /// Auto-detect schemas from co-occurrence analytics.
    ///
    /// Uses the co-occurrence data (PI-5) to find property clusters
    /// that appear together more than expected by chance (high PMI).
    /// Returns newly detected schemas (does not duplicate existing ones).
    async fn auto_detect(
        &self,
        params: &AutoDetectParams,
    ) -> Result<Vec<PropertySchema>, ApplicationError>;
}

#[async_trait]
impl SchemaServiceTrait for SchemaService {
    #[instrument(skip(self))]
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertySchema>, ApplicationError> {
        self.schema_repo
            .get_by_id(id)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn get_by_name(&self, name: &str) -> Result<Option<PropertySchema>, ApplicationError> {
        self.schema_repo
            .get_by_name(name)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<PropertySchema>, ApplicationError> {
        self.schema_repo
            .list_all()
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn create(&self, schema: &PropertySchema) -> Result<(), ApplicationError> {
        // Check name uniqueness
        if self
            .schema_repo
            .get_by_name(&schema.name)
            .await
            .map_err(ApplicationError::Domain)?
            .is_some()
        {
            return Err(ApplicationError::Validation(format!(
                "Schema '{}' already exists",
                schema.name
            )));
        }
        self.schema_repo
            .insert(schema)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn update(&self, schema: &PropertySchema) -> Result<(), ApplicationError> {
        self.schema_repo
            .update(schema)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError> {
        self.schema_repo
            .delete(id)
            .await
            .map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn auto_detect(
        &self,
        params: &AutoDetectParams,
    ) -> Result<Vec<PropertySchema>, ApplicationError> {
        // Get co-occurrence data from analytics
        let analytics_params = AnalyticsParams {
            co_occurrence_limit: 100,
            trend_limit: 0, // We don't need trends for schema detection
            trend_period_days: 30,
        };
        let co_occurrences = self
            .property_repo
            .get_co_occurrences(analytics_params.co_occurrence_limit)
            .await
            .map_err(ApplicationError::Domain)?;

        // Get existing schemas to avoid duplicates
        let existing = self
            .schema_repo
            .list_all()
            .await
            .map_err(ApplicationError::Domain)?;

        // Build a map of existing schema key sets for dedup
        let existing_key_sets: Vec<std::collections::HashSet<String>> = existing
            .iter()
            .map(|s| s.property_keys.iter().cloned().collect())
            .collect();

        // Cluster co-occurring properties using union-find
        let mut clusters = ClusterBuilder::new();

        for co in &co_occurrences {
            if co.co_occurrence_count >= params.min_co_occurrence && co.pmi >= params.min_pmi {
                clusters.union(&co.key_a, &co.key_b);
            }
        }

        let clusters = clusters.build(params.min_properties);

        // Filter out clusters that already exist as schemas
        let mut new_schemas = Vec::new();
        for (idx, cluster) in clusters.into_iter().enumerate() {
            if new_schemas.len() >= params.max_schemas {
                break;
            }

            let key_set: std::collections::HashSet<String> =
                cluster.iter().cloned().collect();

            // Skip if this cluster is a subset of an existing schema
            let is_duplicate = existing_key_sets.iter().any(|existing| {
                key_set.iter().all(|k| existing.contains(k))
            });
            if is_duplicate {
                continue;
            }

            let mut keys: Vec<String> = cluster;
            keys.sort();

            let schema = PropertySchema::new(
                Uuid::new_v4(),
                format!("auto-cluster-{}", idx + 1),
                format!("Auto-detected cluster: {}", keys.join(", ")),
                keys,
                true,
            );

            self.schema_repo
                .insert(&schema)
                .await
                .map_err(ApplicationError::Domain)?;

            new_schemas.push(schema);
        }

        Ok(new_schemas)
    }
}

/// Simple union-find for clustering co-occurring properties.
struct ClusterBuilder {
    parent: std::collections::HashMap<String, String>,
}

impl ClusterBuilder {
    fn new() -> Self {
        Self {
            parent: std::collections::HashMap::new(),
        }
    }

    fn find(&mut self, key: &str) -> String {
        if !self.parent.contains_key(key) {
            self.parent.insert(key.to_string(), key.to_string());
            return key.to_string();
        }
        let parent = self.parent.get(key).unwrap().clone();
        if parent == key {
            return parent;
        }
        let root = self.find(&parent);
        self.parent.insert(key.to_string(), root.clone());
        root
    }

    fn union(&mut self, a: &str, b: &str) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_b, root_a);
        }
    }

    fn build(self, min_size: usize) -> Vec<Vec<String>> {
        let mut groups: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for key in self.parent.keys() {
            // Need a mutable copy to find
            let mut temp_parent = self.parent.clone();
            let root = if let Some(p) = temp_parent.get(key) {
                // Trace to root
                let mut current = p.clone();
                loop {
                    if let Some(next) = temp_parent.get(&current) {
                        if next == &current {
                            break;
                        }
                        current = next.clone();
                    } else {
                        break;
                    }
                }
                current
            } else {
                key.clone()
            };
            groups.entry(root).or_default().push(key.clone());
        }
        groups
            .into_values()
            .filter(|g| g.len() >= min_size)
            .collect()
    }
}
