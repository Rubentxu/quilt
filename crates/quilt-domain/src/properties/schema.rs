//! Property schema templates — reusable property clusters.
//!
//! A `PropertySchema` captures a named set of property keys that frequently
//! appear together. Schemas can be auto-detected from co-occurrence analytics
//! or manually defined by users/agents. Applying a schema to a block/page
//! pre-populates those properties.

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A reusable property schema template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Unique identifier.
    pub id: Uuid,
    /// Canonical name (snake_case, e.g. "task", "meeting-notes").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Property keys included in this schema.
    pub property_keys: Vec<String>,
    /// Whether this schema was auto-detected from analytics.
    pub auto_detected: bool,
    /// Creation timestamp (ms since epoch).
    pub created_at: i64,
    /// Last updated timestamp (ms since epoch).
    pub updated_at: i64,
}

impl PropertySchema {
    /// Create a new property schema.
    pub fn new(
        id: Uuid,
        name: impl Into<String>,
        description: impl Into<String>,
        property_keys: Vec<String>,
        auto_detected: bool,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id,
            name: name.into(),
            description: description.into(),
            property_keys,
            auto_detected,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if this schema contains a given property key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.property_keys.iter().any(|k| k.eq_ignore_ascii_case(key))
    }
}

/// Parameters for auto-detecting schemas from co-occurrence data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDetectParams {
    /// Minimum co-occurrence count to consider a cluster.
    pub min_co_occurrence: u64,
    /// Minimum PMI score (positive = more than chance).
    pub min_pmi: f64,
    /// Maximum number of schemas to detect.
    pub max_schemas: usize,
    /// Minimum number of properties per schema.
    pub min_properties: usize,
}

impl Default for AutoDetectParams {
    fn default() -> Self {
        Self {
            min_co_occurrence: 3,
            min_pmi: 0.5,
            max_schemas: 10,
            min_properties: 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_contains_key() {
        let schema = PropertySchema::new(
            Uuid::new_v4(),
            "task",
            "Task tracking",
            vec!["status".to_string(), "priority".to_string(), "deadline".to_string()],
            false,
        );
        assert!(schema.contains_key("status"));
        assert!(schema.contains_key("Priority"));
        assert!(!schema.contains_key("author"));
    }

    #[test]
    fn test_auto_detect_params_default() {
        let params = AutoDetectParams::default();
        assert_eq!(params.min_co_occurrence, 3);
        assert_eq!(params.min_pmi, 0.5);
        assert_eq!(params.max_schemas, 10);
        assert_eq!(params.min_properties, 2);
    }

    #[test]
    fn test_schema_serialization() {
        let schema = PropertySchema::new(
            Uuid::new_v4(),
            "meeting",
            "Meeting notes",
            vec!["date".to_string(), "attendees".to_string()],
            true,
        );
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("meeting"));
        assert!(json.contains("auto_detected"));
    }
}
