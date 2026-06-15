//! Semantic property relations — directed relationships between property values.
//!
//! Captures semantic knowledge like workflows (TODO → DOING → DONE),
//! hierarchies (epic → story → task), or implications (priority:A → priority:B).
//! Relations are typed, directed, and versioned.

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// Type of semantic relation between two property values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Sequential workflow: A comes before B (e.g., TODO → DOING).
    Precedes,
    /// Generalization: A is a broader category of B.
    Broadens,
    /// Implication: if A then B (e.g., priority:A implies status:urgent).
    Implies,
    /// Dependency: A requires B.
    Requires,
    /// Custom user-defined relation.
    Custom(String),
}

impl RelationType {
    pub fn as_str(&self) -> &str {
        match self {
            RelationType::Precedes => "precedes",
            RelationType::Broadens => "broadens",
            RelationType::Implies => "implies",
            RelationType::Requires => "requires",
            RelationType::Custom(name) => name.as_str(),
        }
    }
}

/// A directed semantic relation between two property values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyRelation {
    /// Unique identifier.
    pub id: Uuid,
    /// Source property key.
    pub source_key: String,
    /// Source property value.
    pub source_value: String,
    /// Target property key.
    pub target_key: String,
    /// Target property value.
    pub target_value: String,
    /// Type of relation.
    pub relation_type: RelationType,
    /// Human-readable description of why this relation exists.
    pub description: String,
    /// Confidence score [0.0, 1.0] — 1.0 for user-defined, <1.0 for inferred.
    pub confidence: f64,
    /// Creation timestamp (ms since epoch).
    pub created_at: i64,
}

impl PropertyRelation {
    /// Create a new property relation.
    pub fn new(
        id: Uuid,
        source_key: impl Into<String>,
        source_value: impl Into<String>,
        target_key: impl Into<String>,
        target_value: impl Into<String>,
        relation_type: RelationType,
        description: impl Into<String>,
        confidence: f64,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id,
            source_key: source_key.into(),
            source_value: source_value.into(),
            target_key: target_key.into(),
            target_value: target_value.into(),
            relation_type,
            description: description.into(),
            confidence: confidence.clamp(0.0, 1.0),
            created_at: now,
        }
    }

    /// Create a workflow step relation (user-defined, confidence = 1.0).
    pub fn workflow_step(
        id: Uuid,
        key: impl Into<String>,
        from_value: impl Into<String>,
        to_value: impl Into<String>,
    ) -> Self {
        let key_str = key.into();
        Self::new(
            id,
            key_str.clone(),
            from_value,
            key_str,
            to_value,
            RelationType::Precedes,
            "Workflow step",
            1.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_step_relation() {
        let rel = PropertyRelation::workflow_step(Uuid::new_v4(), "status", "todo", "doing");
        assert_eq!(rel.source_key, "status");
        assert_eq!(rel.source_value, "todo");
        assert_eq!(rel.target_key, "status");
        assert_eq!(rel.target_value, "doing");
        assert_eq!(rel.relation_type, RelationType::Precedes);
        assert_eq!(rel.confidence, 1.0);
    }

    #[test]
    fn test_relation_type_as_str() {
        assert_eq!(RelationType::Precedes.as_str(), "precedes");
        assert_eq!(
            RelationType::Custom("ranked".to_string()).as_str(),
            "ranked"
        );
    }

    #[test]
    fn test_confidence_clamped() {
        let rel = PropertyRelation::new(
            Uuid::new_v4(),
            "p",
            "a",
            "p",
            "b",
            RelationType::Implies,
            "test",
            1.5, // should clamp to 1.0
        );
        assert_eq!(rel.confidence, 1.0);
    }

    #[test]
    fn test_serialization() {
        let rel = PropertyRelation::workflow_step(Uuid::new_v4(), "status", "todo", "doing");
        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("precedes"));
        assert!(json.contains("todo"));
    }
}
