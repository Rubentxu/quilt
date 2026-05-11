//! Types for Cognitive Mirror analysis
//!
//! Uses `quilt_domain::Uuid` throughout for consistency.

use quilt_domain::value_objects::Uuid as DomainUuid;
use serde::{Deserialize, Serialize};

/// Result of a full cognitive analysis of a page's block graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveMap {
    /// Detected knowledge clusters
    pub clusters: Vec<KnowledgeCluster>,
    /// Reference density per block: block_id -> density score [0, 1]
    pub density: std::collections::HashMap<DomainUuid, f32>,
    /// Block IDs flagged as knowledge frontiers
    pub frontiers: Vec<DomainUuid>,
    /// Detected structural gaps
    pub gaps: Vec<KnowledgeGap>,
    /// Influence/centrality scores per block, sorted descending
    pub influences: Vec<InfluenceScore>,
}

/// A cluster of densely-connected blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCluster {
    pub block_ids: Vec<DomainUuid>,
    pub theme: Option<String>,
    pub coherence_score: f32,
}

impl Default for KnowledgeCluster {
    fn default() -> Self {
        Self {
            block_ids: Vec::new(),
            theme: None,
            coherence_score: 0.0,
        }
    }
}

/// A structural gap between two blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    pub from: DomainUuid,
    pub to: DomainUuid,
    pub shared_refs: Vec<DomainUuid>,
}

impl Default for KnowledgeGap {
    fn default() -> Self {
        Self {
            from: DomainUuid::nil(),
            to: DomainUuid::nil(),
            shared_refs: Vec::new(),
        }
    }
}

/// Influence score for a single block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfluenceScore {
    pub block_id: DomainUuid,
    pub influence_score: f32,
}

impl Default for InfluenceScore {
    fn default() -> Self {
        Self {
            block_id: DomainUuid::nil(),
            influence_score: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_map_default() {
        let map = CognitiveMap::default();
        assert!(map.clusters.is_empty());
        assert!(map.density.is_empty());
        assert!(map.frontiers.is_empty());
        assert!(map.gaps.is_empty());
        assert!(map.influences.is_empty());
    }

    #[test]
    fn test_knowledge_cluster_clone() {
        let cluster = KnowledgeCluster {
            block_ids: vec![DomainUuid::new_v4(), DomainUuid::new_v4()],
            theme: Some("Rust".to_string()),
            coherence_score: 0.85,
        };
        let cloned = cluster.clone();
        assert_eq!(cloned.block_ids.len(), 2);
        assert_eq!(cloned.theme.as_deref(), Some("Rust"));
        assert_eq!(cloned.coherence_score, 0.85);
    }

    #[test]
    fn test_knowledge_gap_default() {
        let gap = KnowledgeGap::default();
        assert_eq!(gap.from, DomainUuid::nil());
        assert_eq!(gap.to, DomainUuid::nil());
        assert!(gap.shared_refs.is_empty());
    }

    #[test]
    fn test_influence_score_default() {
        let score = InfluenceScore::default();
        assert_eq!(score.block_id, DomainUuid::nil());
        assert_eq!(score.influence_score, 0.0);
    }
}
