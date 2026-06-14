//! Schema use cases — PI-7
//!
//! Thin glue over [`SchemaRepository`] + [`PropertyRepository`] that adds:
//! - Input validation and parsing
//! - Schema CRUD operations
//! - Auto-detection of property clusters

use crate::errors::ApplicationError;
use async_trait::async_trait;
use quilt_domain::properties::analytics::AutoDetectParams;
use quilt_domain::properties::schema::PropertySchema;
use quilt_domain::repositories::{PropertyRepository, SchemaRepository};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Schema use cases trait — PI-7 (schema templates).
///
/// Object-safe (`Send + Sync`) and uses `#[async_trait]`.
#[async_trait]
pub trait SchemaUseCases: Send + Sync {
    /// List all schemas.
    async fn list_all(&self) -> Result<Vec<PropertySchema>, ApplicationError>;

    /// Get a schema by ID.
    async fn get_by_id(&self, id: Uuid) -> Result<Option<PropertySchema>, ApplicationError>;

    /// Get a schema by name.
    async fn get_by_name(&self, name: &str) -> Result<Option<PropertySchema>, ApplicationError>;

    /// Create a new schema.
    async fn create(&self, schema: &PropertySchema) -> Result<(), ApplicationError>;

    /// Update an existing schema.
    async fn update(&self, schema: &PropertySchema) -> Result<(), ApplicationError>;

    /// Delete a schema.
    async fn delete(&self, id: Uuid) -> Result<(), ApplicationError>;

    /// Auto-detect schemas from co-occurrence analytics.
    async fn auto_detect(
        &self,
        params: &AutoDetectParams,
    ) -> Result<Vec<PropertySchema>, ApplicationError>;
}

/// Implementation of [`SchemaUseCases`] for any repositories.
pub struct SchemaUseCasesImpl<SR: SchemaRepository, PR: PropertyRepository> {
    schema_repo: Arc<SR>,
    property_repo: Arc<PR>,
}

impl<SR: SchemaRepository, PR: PropertyRepository> SchemaUseCasesImpl<SR, PR> {
    pub fn new(schema_repo: Arc<SR>, property_repo: Arc<PR>) -> Self {
        Self {
            schema_repo,
            property_repo,
        }
    }
}

#[async_trait]
impl<SR: SchemaRepository + 'static, PR: PropertyRepository + 'static> SchemaUseCases
    for SchemaUseCasesImpl<SR, PR>
{
    #[instrument(skip(self))]
    async fn list_all(&self) -> Result<Vec<PropertySchema>, ApplicationError> {
        self.schema_repo
            .list_all()
            .await
            .map_err(ApplicationError::Domain)
    }

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
                "Schema with name '{}' already exists",
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
        self.schema_repo.delete(id).await.map_err(ApplicationError::Domain)
    }

    #[instrument(skip(self))]
    async fn auto_detect(
        &self,
        params: &AutoDetectParams,
    ) -> Result<Vec<PropertySchema>, ApplicationError> {
        // Get co-occurrences from property repo
        let co_occurrences = self
            .property_repo
            .get_co_occurrences(1000) // Get more for analysis
            .await
            .map_err(ApplicationError::Domain)?;

        // Get all property definitions
        let all_properties = self
            .property_repo
            .get_all()
            .await
            .map_err(ApplicationError::Domain)?;

        // Filter by min_co_occurrence
        let significant: Vec<_> = co_occurrences
            .into_iter()
            .filter(|co| co.co_occurrence as u64 >= params.min_co_occurrence)
            .collect();

        // Group by property set
        let mut seen_sets: std::collections::HashSet<Vec<String>> = std::collections::HashSet::new();
        let mut detected_schemas: Vec<PropertySchema> = Vec::new();

        for co in significant {
            let key_a = &co.property_key_a;
            let key_b = &co.property_key_b;

            // Calculate PMI (Pointwise Mutual Information)
            let total_pairs: f64 = significant.len() as f64;
            let pmi_a = co.co_occurrence as f64 / (all_properties.iter().find(|p| &p.key == key_a).map(|p| p.block_count as f64).unwrap_or(1.0));
            let pmi_b = co.co_occurrence as f64 / (all_properties.iter().find(|p| &p.key == key_b).map(|p| p.block_count as f64).unwrap_or(1.0));
            let pmi = (pmi_a * pmi_b).sqrt();

            if pmi >= params.min_pmi {
                let mut keys = vec![key_a.clone(), key_b.clone()];
                keys.sort();
                if seen_sets.insert(keys.clone()) && keys.len() >= params.min_properties && detected_schemas.len() < params.max_schemas {
                    let schema = PropertySchema::new(
                        Uuid::new_v4(),
                        format!("Auto-detected: {}", keys.join(" + ")),
                        format!("Auto-detected schema for properties: {}", keys.join(", ")),
                        keys,
                        false,
                    );
                    detected_schemas.push(schema);
                }
            }
        }

        Ok(detected_schemas)
    }
}
