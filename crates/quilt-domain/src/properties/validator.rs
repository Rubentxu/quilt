//! PropertyValidator - validates property values against definitions

use crate::errors::DomainError;
use crate::properties::definition::PropertyDefinition;
use crate::properties::types::{Cardinality, PropertyType};
use crate::repositories::{PropertyRepository, PropertyRepositoryExt};
use crate::value_objects::PropertyValue;
use std::collections::HashMap;
use std::sync::Arc;

/// PropertyValidator validates property values against their definitions.
///
/// It checks:
/// - Type compatibility (value matches property type)
/// - Cardinality (single value vs array)
/// - Closed value membership (if restricted)
pub struct PropertyValidator<P: PropertyRepository> {
    repo: Arc<P>,
}

impl<P: PropertyRepository> PropertyValidator<P> {
    /// Create a new PropertyValidator with the given repository
    pub fn new(repo: Arc<P>) -> Self {
        Self { repo }
    }

    /// Validate a map of property values.
    ///
    /// For each key-value pair, looks up the property definition and validates.
    /// Unknown properties (not in registry) are accepted without validation.
    pub async fn validate(
        &self,
        props: &HashMap<String, PropertyValue>,
    ) -> Result<(), DomainError> {
        for (key, value) in props {
            self.validate_property(key, value).await?;
        }
        Ok(())
    }

    /// Validate a single property value against its definition.
    ///
    /// Returns Ok(()) if:
    /// - The property is not registered (unknown properties are allowed)
    /// - The value is valid for the property type
    ///
    /// Returns Err(DomainError::PropertyValidationError) if:
    /// - Type mismatch
    /// - Cardinality violation
    /// - Value not in closed set
    pub async fn validate_property(
        &self,
        key: &str,
        value: &PropertyValue,
    ) -> Result<(), DomainError> {
        // Try to find the property definition
        let def = match self.repo.find_or_builtin(key).await {
            Ok(Some(def)) => def,
            Ok(None) => {
                // Unknown property - accept without validation (backward compatible)
                return Ok(());
            }
            Err(e) => {
                return Err(DomainError::Database(format!(
                    "Failed to lookup property '{}': {}",
                    key, e
                )));
            }
        };

        // Check type compatibility
        self.validate_type(&def, value)?;

        // Check cardinality
        self.validate_cardinality(&def, value)?;

        // Check closed values if applicable
        self.validate_closed_set(&def, value)?;

        Ok(())
    }

    /// Check if the value type matches the property type
    fn validate_type(
        &self,
        def: &PropertyDefinition,
        value: &PropertyValue,
    ) -> Result<(), DomainError> {
        let compatible = match (&def.property_type, value) {
            (PropertyType::Text, PropertyValue::String(_)) => true,
            (PropertyType::Number, PropertyValue::Integer(_)) => true,
            (PropertyType::Number, PropertyValue::Float(_)) => true,
            (PropertyType::Date, PropertyValue::Date(_)) => true,
            (PropertyType::Date, PropertyValue::NaiveDate(_)) => true, // Canonical
            (PropertyType::DateTime, PropertyValue::Date(_)) => true,
            (PropertyType::Url, PropertyValue::Url(_)) => true,        // First-class
            (PropertyType::Url, PropertyValue::String(_)) => true,     // Backward compat
            (PropertyType::Checkbox, PropertyValue::Boolean(_)) => true, // Backward compat
            (PropertyType::Node, PropertyValue::Ref(_)) => true,   // Backward compat
            _ => false,
        };

        if !compatible {
            return Err(DomainError::PropertyValidationError {
                property: def.db_ident.clone(),
                error: format!(
                    "Type mismatch: expected {} but got {}",
                    def.property_type.as_str(),
                    value.type_name()
                ),
            });
        }

        Ok(())
    }

    /// Check if the cardinality is respected
    fn validate_cardinality(
        &self,
        def: &PropertyDefinition,
        value: &PropertyValue,
    ) -> Result<(), DomainError> {
        match (&def.cardinality, value) {
            (Cardinality::One, PropertyValue::Array(_)) => {
                Err(DomainError::PropertyValidationError {
                    property: def.db_ident.clone(),
                    error: format!(
                        "Cardinality violation: expected single value but got array (cardinality is {})",
                        def.cardinality.as_str()
                    ),
                })
            }
            (Cardinality::Many, _) => Ok(()), // Any value is allowed, including arrays
            _ => Ok(()),
        }
    }

    /// Check if the value is in the closed set (if defined)
    fn validate_closed_set(
        &self,
        def: &PropertyDefinition,
        value: &PropertyValue,
    ) -> Result<(), DomainError> {
        if def.closed_values.is_empty() {
            return Ok(());
        }

        let value_str = match value {
            PropertyValue::String(s) => Some(s.as_str()),
            _ => None,
        };

        if let Some(value_str) = value_str {
            if !def.is_value_allowed(value_str) {
                return Err(DomainError::PropertyValidationError {
                    property: def.db_ident.clone(),
                    error: format!(
                        "Value '{}' is not in the closed set: {:?}",
                        value_str,
                        def.closed_values
                            .iter()
                            .map(|cv| cv.value.as_str())
                            .collect::<Vec<_>>()
                    ),
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::types::ClosedValue;
    use crate::value_objects::Uuid;
    use async_trait::async_trait;
    use chrono::TimeZone;
    use std::collections::HashMap;
    use url::Url;

    // Mock repository for testing
    struct MockPropertyRepository {
        properties: HashMap<String, PropertyDefinition>,
    }

    impl MockPropertyRepository {
        fn new() -> Self {
            Self {
                properties: HashMap::new(),
            }
        }

        fn with_property(mut self, def: PropertyDefinition) -> Self {
            self.properties.insert(def.db_ident.clone(), def);
            self
        }
    }

    #[async_trait]
    impl PropertyRepository for MockPropertyRepository {
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<PropertyDefinition>, DomainError> {
            Ok(None)
        }

        async fn get_by_db_ident(
            &self,
            ident: &str,
        ) -> Result<Option<PropertyDefinition>, DomainError> {
            Ok(self.properties.get(ident).cloned())
        }

        async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(self.properties.values().cloned().collect())
        }

        async fn insert(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
            Ok(())
        }

        async fn update(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_closed_values(
            &self,
            _property_id: Uuid,
        ) -> Result<Vec<ClosedValue>, DomainError> {
            Ok(Vec::new())
        }

        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_by_db_idents(
            &self,
            _idents: &[&str],
        ) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn list_by_usage(
            &self,
            _limit: usize,
        ) -> Result<Vec<PropertyDefinition>, DomainError> {
            Ok(Vec::new())
        }
        async fn get_co_occurrences(
            &self,
            _limit: usize,
        ) -> Result<Vec<super::super::analytics::PropertyCoOccurrence>, DomainError> {
            Ok(vec![])
        }
        async fn get_trends(
            &self,
            _period_days: u32,
            _limit: usize,
        ) -> Result<Vec<super::super::analytics::PropertyTrend>, DomainError> {
            Ok(vec![])
        }
        async fn count_distinct_properties(&self) -> Result<u64, DomainError> {
            Ok(0)
        }
        async fn count_blocks_with_properties(&self) -> Result<u64, DomainError> {
            Ok(0)
        }
    }

    fn create_status_prop() -> PropertyDefinition {
        let closed_values = vec![
            ClosedValue::new(Uuid::new_v4(), "todo", "To Do"),
            ClosedValue::new(Uuid::new_v4(), "doing", "Doing"),
            ClosedValue::new(Uuid::new_v4(), "done", "Done"),
        ];
        PropertyDefinition::new(
            Uuid::new_v4(),
            "status",
            "Status",
            crate::properties::types::PropertyType::Text,
        )
        .with_closed_values(closed_values)
    }

    #[tokio::test]
    async fn test_validate_unknown_property_passes() {
        let repo = MockPropertyRepository::new();
        let validator = PropertyValidator::new(Arc::new(repo));

        let result = validator
            .validate_property("custom-field", &PropertyValue::string("any value"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_string_value_for_text_property() {
        let prop = create_status_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        let result = validator
            .validate_property("status", &PropertyValue::string("To Do"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_type_mismatch() {
        let prop = create_status_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // Try to pass an integer to a text property
        let result = validator
            .validate_property("status", &PropertyValue::integer(42))
            .await;
        assert!(result.is_err());
        if let Err(DomainError::PropertyValidationError { property, error }) = result {
            assert_eq!(property, "status");
            assert!(error.contains("Type mismatch"));
        } else {
            panic!("Expected PropertyValidationError");
        }
    }

    #[tokio::test]
    async fn test_validate_closed_set_violation() {
        let prop = create_status_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // Try to pass an invalid status value
        let result = validator
            .validate_property("status", &PropertyValue::string("INVALID"))
            .await;
        assert!(result.is_err());
        if let Err(DomainError::PropertyValidationError { property, error }) = result {
            assert_eq!(property, "status");
            assert!(error.contains("not in the closed set"));
        } else {
            panic!("Expected PropertyValidationError");
        }
    }

    #[tokio::test]
    async fn test_validate_closed_set_valid() {
        let prop = create_status_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // Valid status values
        assert!(
            validator
                .validate_property("status", &PropertyValue::string("To Do"))
                .await
                .is_ok()
        );
        assert!(
            validator
                .validate_property("status", &PropertyValue::string("Doing"))
                .await
                .is_ok()
        );
        assert!(
            validator
                .validate_property("status", &PropertyValue::string("Done"))
                .await
                .is_ok()
        );

        // Also accept by db_ident
        assert!(
            validator
                .validate_property("status", &PropertyValue::string("todo"))
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn test_validate_multiple_properties() {
        let repo = MockPropertyRepository::new().with_property(create_status_prop());
        let validator = PropertyValidator::new(Arc::new(repo));

        let mut props = HashMap::new();
        props.insert("status".to_string(), PropertyValue::string("To Do"));
        props.insert(
            "custom-field".to_string(),
            PropertyValue::string("any value"),
        );

        let result = validator.validate(&props).await;
        assert!(result.is_ok());
    }

    // ── ADR-0027: typed PropertyValue variants ─────────────────────────────────

    fn make_url_prop() -> PropertyDefinition {
        PropertyDefinition::new(
            Uuid::new_v4(),
            "source-url",
            "Source URL",
            crate::properties::types::PropertyType::Url,
        )
    }

    fn make_date_prop() -> PropertyDefinition {
        PropertyDefinition::new(
            Uuid::new_v4(),
            "scheduled",
            "Scheduled",
            crate::properties::types::PropertyType::Date,
        )
    }

    fn make_datetime_prop() -> PropertyDefinition {
        PropertyDefinition::new(
            Uuid::new_v4(),
            "deadline",
            "Deadline",
            crate::properties::types::PropertyType::DateTime,
        )
    }

    #[tokio::test]
    async fn validate_url_value_for_url_property() {
        let prop = make_url_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        let url = Url::parse("https://quilt.dev").unwrap();
        let result = validator
            .validate_property("source-url", &PropertyValue::url(url))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_string_value_for_url_property_backward_compat() {
        let prop = make_url_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // Legacy: String URL value still accepted
        let result = validator
            .validate_property("source-url", &PropertyValue::string("https://legacy.com"))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_naive_date_for_date_property() {
        let prop = make_date_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let result = validator
            .validate_property("scheduled", &PropertyValue::naive_date(d))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_date_for_date_property_backward_compat() {
        let prop = make_date_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // Legacy: Date value still accepted
        let dt = chrono::Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap();
        let result = validator
            .validate_property("scheduled", &PropertyValue::date(dt))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn validate_naive_date_rejected_for_datetime_property() {
        let prop = make_datetime_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        // NaiveDate is not valid for DateTime (semantic guard)
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        let result = validator
            .validate_property("deadline", &PropertyValue::naive_date(d))
            .await;
        assert!(result.is_err());
        if let Err(DomainError::PropertyValidationError { error, .. }) = result {
            assert!(error.contains("DateTime"), "error should mention expected type: {}", error);
            assert!(error.contains("date"), "error should mention got type: {}", error);
        }
    }

    #[tokio::test]
    async fn validate_integer_rejected_for_url_property() {
        let prop = make_url_prop();
        let repo = MockPropertyRepository::new().with_property(prop);
        let validator = PropertyValidator::new(Arc::new(repo));

        let result = validator
            .validate_property("source-url", &PropertyValue::integer(42))
            .await;
        assert!(result.is_err());
        if let Err(DomainError::PropertyValidationError { error, .. }) = result {
            assert!(error.contains("Url") || error.contains("url"), "error should mention expected type: {}", error);
            assert!(error.contains("integer"), "error should mention got type: {}", error);
        }
    }
}
