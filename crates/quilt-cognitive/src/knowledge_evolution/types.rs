//! Knowledge Evolution Tracker types
//!
//! Defines types for tracking how knowledge and beliefs evolve over time.

use chrono::{DateTime, Utc};
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A change in belief or knowledge about a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefChange {
    /// When this change occurred.
    pub timestamp: DateTime<Utc>,
    /// Previous belief or understanding.
    pub from: String,
    /// New belief or understanding.
    pub to: String,
    /// Confidence in this change (0.0-1.0).
    pub confidence: f32,
    /// Block ID that provided evidence for this change (optional).
    pub evidence_block_id: Option<Uuid>,
}

impl BeliefChange {
    /// Create a new belief change.
    pub fn new(
        timestamp: DateTime<Utc>,
        from: impl Into<String>,
        to: impl Into<String>,
        confidence: f32,
        evidence_block_id: Option<Uuid>,
    ) -> Self {
        Self {
            timestamp,
            from: from.into(),
            to: to.into(),
            confidence,
            evidence_block_id,
        }
    }
}

/// A timeline of knowledge evolution for a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTimeline {
    /// The topic being tracked.
    pub topic: String,
    /// All recorded belief changes.
    pub belief_changes: Vec<BeliefChange>,
    /// Ideas that were abandoned over time.
    pub abandoned_ideas: Vec<String>,
    /// Ideas that were reinforced over time.
    pub reinforced_ideas: Vec<String>,
}

impl KnowledgeTimeline {
    /// Create a new knowledge timeline for a topic.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            belief_changes: Vec::new(),
            abandoned_ideas: Vec::new(),
            reinforced_ideas: Vec::new(),
        }
    }

    /// Add a belief change to the timeline.
    pub fn add_change(&mut self, change: BeliefChange) {
        self.belief_changes.push(change);
    }

    /// Record an abandoned idea.
    pub fn add_abandoned(&mut self, idea: impl Into<String>) {
        self.abandoned_ideas.push(idea.into());
    }

    /// Record a reinforced idea.
    pub fn add_reinforced(&mut self, idea: impl Into<String>) {
        self.reinforced_ideas.push(idea.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belief_change_new() {
        let change = BeliefChange::new(
            Utc::now(),
            "Old understanding",
            "New understanding",
            0.85,
            None,
        );
        assert_eq!(change.from, "Old understanding");
        assert_eq!(change.to, "New understanding");
        assert_eq!(change.confidence, 0.85);
        assert!(change.evidence_block_id.is_none());
    }

    #[test]
    fn test_knowledge_timeline_new() {
        let timeline = KnowledgeTimeline::new("Rust async programming");
        assert_eq!(timeline.topic, "Rust async programming");
        assert!(timeline.belief_changes.is_empty());
        assert!(timeline.abandoned_ideas.is_empty());
        assert!(timeline.reinforced_ideas.is_empty());
    }

    #[test]
    fn test_knowledge_timeline_add_change() {
        let mut timeline = KnowledgeTimeline::new("Test topic");
        let change = BeliefChange::new(Utc::now(), "From A", "To B", 0.7, None);
        timeline.add_change(change);
        assert_eq!(timeline.belief_changes.len(), 1);
    }

    #[test]
    fn test_knowledge_timeline_add_abandoned() {
        let mut timeline = KnowledgeTimeline::new("Test");
        timeline.add_abandoned("Wrong idea");
        assert_eq!(timeline.abandoned_ideas.len(), 1);
    }

    #[test]
    fn test_knowledge_timeline_add_reinforced() {
        let mut timeline = KnowledgeTimeline::new("Test");
        timeline.add_reinforced("Correct idea");
        assert_eq!(timeline.reinforced_ideas.len(), 1);
    }

    #[test]
    fn test_confidence_range() {
        let change = BeliefChange::new(Utc::now(), "from", "to", 0.95, None);
        assert!(change.confidence >= 0.0 && change.confidence <= 1.0);
    }
}
