//! Types for Agent Memory

use chrono::{DateTime, Utc};
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A single memory entry stored for an agent.
///
/// Stored as a Block with properties encoding the memory data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier for this memory
    pub id: Uuid,
    /// Agent that created this memory
    pub agent_id: String,
    /// Domain/context this memory belongs to (e.g., "rust", "ml", "architecture")
    pub context: String,
    /// The actual memory content
    pub content: String,
    /// Initial confidence/importance score [0, 1]
    pub importance: f32,
    /// Decay rate constant λ (halflife = ln(2)/λ)
    pub decay_rate: f32,
    /// When this memory was first created
    pub created_at: DateTime<Utc>,
    /// When this memory was last accessed
    pub last_accessed: DateTime<Utc>,
}

impl MemoryEntry {
    /// Compute the current relevance score using exponential decay.
    ///
    /// relevance = importance * exp(-λ * days_since_last_access)
    pub fn relevance_score(&self) -> f32 {
        let days = (Utc::now() - self.last_accessed).num_seconds() as f64 / (24.0 * 3600.0);
        let decay = (-self.decay_rate as f64 * days).exp();
        (self.importance as f64 * decay).clamp(0.0, 1.0) as f32
    }
}

/// A recurring thought/analysis structure an agent prefers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingPattern {
    /// Domain this pattern applies to (e.g., "rust", "ml")
    pub domain: String,
    /// Preferred structure for organizing thoughts
    pub preferred_structure: String,
    /// Abstraction level 0.0 (concrete) to 1.0 (highly abstract)
    pub abstraction_level: f32,
    /// Topics this agent tends to engage with
    pub topic_affinities: Vec<String>,
}

impl Default for ThinkingPattern {
    fn default() -> Self {
        Self {
            domain: String::new(),
            preferred_structure: "hierarchical".to_string(),
            abstraction_level: 0.5,
            topic_affinities: Vec::new(),
        }
    }
}

/// A cognitive bias observed in an agent's behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBias {
    /// Type of bias (e.g., "confirmation", "anchoring", "availability")
    pub bias_type: String,
    /// Human-readable description
    pub description: String,
    /// Strength of the bias [0, 1]
    pub strength: f32,
}

impl Default for CognitiveBias {
    fn default() -> Self {
        Self {
            bias_type: String::new(),
            description: String::new(),
            strength: 0.5,
        }
    }
}

/// An agent's complete interaction profile.
///
/// Stored as a single block with JSON content containing all sub-structures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InteractionProfile {
    /// The agent's thinking patterns
    pub thinking_pattern: ThinkingPattern,
    /// Observed cognitive biases
    pub cognitive_biases: Vec<CognitiveBias>,
    /// Per-domain knowledge levels
    pub knowledge_levels: std::collections::HashMap<String, f32>,
}

/// Query parameters for retrieving agent memories.
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// Agent ID to filter by
    pub agent_id: String,
    /// Optional domain/context filter
    pub context: Option<String>,
    /// Optional free-text search query (uses FTS5)
    pub query: Option<String>,
    /// Maximum results to return
    pub limit: usize,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            agent_id: String::new(),
            context: None,
            query: None,
            limit: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_memory_entry_fresh() {
        let now = Utc::now();
        let entry = MemoryEntry {
            id: uuid(),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Rust ownership is powerful".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: now,
            last_accessed: now,
        };
        let score = entry.relevance_score();
        assert!((score - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_memory_entry_decays() {
        let now = Utc::now();
        let old = now - chrono::Duration::days(30);
        let entry = MemoryEntry {
            id: uuid(),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Rust ownership".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: old,
            last_accessed: old,
        };
        let score = entry.relevance_score();
        // ~30 days, λ=0.05: decay = exp(-0.05*30) = exp(-1.5) ≈ 0.223
        // relevance ≈ 0.9 * 0.223 ≈ 0.201
        assert!(score < 0.25);
        assert!(score > 0.15);
    }

    #[test]
    fn test_thinking_pattern_default() {
        let tp = ThinkingPattern::default();
        assert_eq!(tp.domain, "");
        assert_eq!(tp.abstraction_level, 0.5);
        assert!(tp.topic_affinities.is_empty());
    }

    #[test]
    fn test_interaction_profile_default() {
        let profile = InteractionProfile::default();
        assert!(profile.cognitive_biases.is_empty());
        assert!(profile.knowledge_levels.is_empty());
    }

    #[test]
    fn test_memory_query_default() {
        let q = MemoryQuery::default();
        assert_eq!(q.agent_id, "");
        assert!(q.context.is_none());
        assert!(q.query.is_none());
        assert_eq!(q.limit, 20);
    }
}
