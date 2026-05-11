//! Mental Model Gardener types
//!
//! Defines the type system for tracking and analyzing belief evolution
//! in a user's journal pages over time.

use chrono::{DateTime, Utc};
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A belief extracted from journal entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedBelief {
    /// The concept or statement this belief is about.
    pub concept: String,
    /// Confidence in this belief extraction 0.0–1.0.
    pub confidence: f64,
    /// Block IDs that support this belief.
    pub source_ids: Vec<Uuid>,
    /// When this belief was first observed.
    pub first_seen: DateTime<Utc>,
}

/// A belief in the mental model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Belief {
    /// Unique identifier for this belief.
    pub id: Uuid,
    /// The statement or concept.
    pub statement: String,
    /// Confidence 0.0–1.0.
    pub confidence: f64,
    /// Blocks that are the source of this belief.
    pub source_blocks: Vec<Uuid>,
    /// When this belief was last updated.
    pub last_updated: DateTime<Utc>,
}

/// State of a belief in the evolution timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeliefState {
    /// Newly observed belief.
    New,
    /// Repeatedly reinforced through multiple observations.
    Strengthened,
    /// Contradicted by new evidence.
    Weakened,
    /// No mention for more than 30 days.
    Abandoned,
    /// No significant change in observation pattern.
    Unchanged,
}

/// A snapshot of a belief at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefSnapshot {
    /// Belief this snapshot is for.
    pub belief_id: Uuid,
    /// Confidence at time of snapshot.
    pub confidence: f64,
    /// Number of blocks supporting this belief at snapshot time.
    pub supporting_blocks: usize,
    /// When this snapshot was taken.
    pub timestamp: DateTime<Utc>,
    /// State of the belief at this point.
    pub state: BeliefState,
}

/// A contradiction between two beliefs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    /// First belief in the contradiction.
    pub belief_a: Uuid,
    /// Second belief in the contradiction.
    pub belief_b: Uuid,
    /// Explanation of the contradiction.
    pub explanation: String,
    /// Severity 0.0–1.0 (high = mutually exclusive).
    pub severity: f64,
}

/// Severity level for contradictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContradictionSeverity {
    /// Nuanced update, not a real contradiction.
    Low,
    /// Partially incompatible beliefs.
    Medium,
    /// Mutually exclusive beliefs.
    High,
}

/// A suggestion for deepening understanding in a shallow area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepeningSuggestion {
    /// Concept that could be deepened.
    pub concept: String,
    /// Current depth (number of observations).
    pub current_depth: usize,
    /// Suggested questions to deepen understanding.
    pub suggested_questions: Vec<String>,
}

/// Complete mental model for an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MentalModel {
    /// Agent/journals prefix this model is for.
    pub agent_id: String,
    /// All tracked beliefs.
    pub beliefs: Vec<Belief>,
    /// Evolution snapshots over time.
    pub evolution: Vec<BeliefSnapshot>,
}

impl MentalModel {
    /// Create a new empty mental model for an agent.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            beliefs: Vec::new(),
            evolution: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mental_model_empty() {
        let model = MentalModel::new("user");
        assert!(model.beliefs.is_empty());
        assert!(model.evolution.is_empty());
        assert_eq!(model.agent_id, "user");
    }

    #[test]
    fn test_belief_extraction_fields() {
        let belief = ExtractedBelief {
            concept: "Rust async is the future".to_string(),
            confidence: 0.85,
            source_ids: vec![Uuid::new_v4()],
            first_seen: Utc::now(),
        };
        assert!(belief.confidence >= 0.0 && belief.confidence <= 1.0);
        assert!(!belief.concept.is_empty());
    }

    #[test]
    fn test_belief_snapshot_fields() {
        let snapshot = BeliefSnapshot {
            belief_id: Uuid::new_v4(),
            confidence: 0.8,
            supporting_blocks: 3,
            timestamp: Utc::now(),
            state: BeliefState::Strengthened,
        };
        assert_eq!(snapshot.state, BeliefState::Strengthened);
        assert!(snapshot.supporting_blocks > 0);
    }

    #[test]
    fn test_contradiction_fields() {
        let contradiction = Contradiction {
            belief_a: Uuid::new_v4(),
            belief_b: Uuid::new_v4(),
            explanation: "X is claimed to be both good and bad".to_string(),
            severity: 0.7,
        };
        assert!(contradiction.severity >= 0.0 && contradiction.severity <= 1.0);
    }

    #[test]
    fn test_mental_model_evolution() {
        let belief_id = Uuid::new_v4();
        let model = MentalModel {
            agent_id: "user".to_string(),
            beliefs: vec![Belief {
                id: belief_id,
                statement: "Rust is great".to_string(),
                confidence: 0.8,
                source_blocks: vec![],
                last_updated: Utc::now(),
            }],
            evolution: vec![
                BeliefSnapshot {
                    belief_id,
                    confidence: 0.5,
                    supporting_blocks: 1,
                    timestamp: Utc::now(),
                    state: BeliefState::New,
                },
                BeliefSnapshot {
                    belief_id,
                    confidence: 0.8,
                    supporting_blocks: 3,
                    timestamp: Utc::now(),
                    state: BeliefState::Strengthened,
                },
            ],
        };
        assert_eq!(model.beliefs.len(), 1);
        assert_eq!(model.evolution.len(), 2);
    }

    #[test]
    fn test_confidence_range() {
        let valid_belief = ExtractedBelief {
            concept: "Test".to_string(),
            confidence: 0.75,
            source_ids: vec![],
            first_seen: Utc::now(),
        };
        assert!(valid_belief.confidence >= 0.0 && valid_belief.confidence <= 1.0);

        // Confidence outside [0,1] should be rejected
        let out_of_range = valid_belief.confidence < 0.0 || valid_belief.confidence > 1.0;
        assert!(!out_of_range);
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(
            serde_json::to_string(&ContradictionSeverity::Low).unwrap(),
            "\"low\""
        );
        assert_eq!(
            serde_json::to_string(&ContradictionSeverity::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::to_string(&ContradictionSeverity::High).unwrap(),
            "\"high\""
        );
    }

    #[test]
    fn test_belief_state_serialization() {
        assert_eq!(serde_json::to_string(&BeliefState::New).unwrap(), "\"new\"");
        assert_eq!(
            serde_json::to_string(&BeliefState::Strengthened).unwrap(),
            "\"strengthened\""
        );
        assert_eq!(
            serde_json::to_string(&BeliefState::Weakened).unwrap(),
            "\"weakened\""
        );
        assert_eq!(
            serde_json::to_string(&BeliefState::Abandoned).unwrap(),
            "\"abandoned\""
        );
        assert_eq!(
            serde_json::to_string(&BeliefState::Unchanged).unwrap(),
            "\"unchanged\""
        );
    }

    #[test]
    fn test_deepening_suggestion_fields() {
        let suggestion = DeepeningSuggestion {
            concept: "Rust macros".to_string(),
            current_depth: 1,
            suggested_questions: vec![
                "How do declarative macros work?".to_string(),
                "How do procedural macros differ?".to_string(),
            ],
        };
        assert_eq!(suggestion.current_depth, 1);
        assert!(!suggestion.suggested_questions.is_empty());
    }

    #[test]
    fn test_mental_model_with_beliefs_round_trip() {
        let model = MentalModel::new("test-agent");
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: MentalModel = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.agent_id, "test-agent");
    }
}
