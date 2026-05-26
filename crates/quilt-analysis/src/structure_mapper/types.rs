//! Structure Mapper types
//!
//! Defines the type system for mapping structure within a page:
//! nodes (claims, evidence, rebuttals), typed edges (supports, refutes, qualifies),
//! and derived structures (consensus zones, detected tensions).

use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// Role of a block in a structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgumentRole {
    /// A claim or thesis being argued.
    Claim,
    /// Evidence or data supporting a claim.
    Evidence,
    /// A rebuttal or counter-argument.
    Rebuttal,
    /// A qualification or nuance to a claim.
    Qualification,
    /// An underlying assumption.
    Assumption,
}

/// Type of relationship between two structure nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArgumentEdgeType {
    /// Source supports the target claim.
    Supports,
    /// Source refutes or contradicts the target claim.
    Refutes,
    /// Source qualifies or nuances the target.
    Qualifies,
}

/// A node in a structure graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureNode {
    /// Block ID this node was derived from.
    pub block_id: Uuid,
    /// Role of this block in the structure.
    pub role: ArgumentRole,
    /// Strength score 0.0–1.0.
    pub strength: f64,
    /// Whether this node is in an "in" position (supporting), "out" (opposing),
    /// or "neutral" position in the structure graph.
    pub position: Position,
}

/// Position of a node in the structure graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    In,
    Out,
    Neutral,
}

/// An edge connecting two structure nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureEdge {
    /// Source node ID.
    pub source: Uuid,
    /// Target node ID.
    pub target: Uuid,
    /// Type of the relationship.
    pub edge_type: ArgumentEdgeType,
    /// Confidence in this relationship 0.0–1.0.
    pub confidence: f64,
}

/// A zone of high coherence (mutual support) among nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusZone {
    /// Block IDs belonging to this consensus zone.
    pub block_ids: Vec<Uuid>,
    /// Coherence score 0.0–1.0.
    pub coherence_score: f64,
}

/// A detected argument in a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentDetection {
    /// Classification of the block.
    pub classification: ArgumentRole,
    /// Confidence in this classification 0.0–1.0.
    pub confidence: f64,
    /// IDs of blocks that provide evidence for this argument.
    pub evidence_refs: Vec<Uuid>,
}

/// A detected logical fallacy in an argument.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedFallacy {
    /// Type of fallacy detected.
    pub fallacy_type: FallacyType,
    /// Block ID where the fallacy was found.
    pub block_id: Uuid,
    /// Human-readable explanation of the fallacy.
    pub explanation: String,
}

/// Types of logical fallacies detectable by AI analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallacyType {
    /// Misrepresents an opponent's argument to easier defeat it.
    StrawMan,
    /// Attacks the person rather than the argument.
    AdHominem,
    /// Presents only two options when more exist.
    FalseDichotomy,
    /// Assumes a chain of events without justification.
    SlipperySlope,
    /// Conclusion is assumed in the premise (circular reasoning).
    Circular,
}

/// A pair of contradictory claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionPair {
    /// First block in the contradiction.
    pub a: Uuid,
    /// Second block in the contradiction.
    pub b: Uuid,
    /// Category of the conflict.
    pub conflict_type: ConflictType,
    /// Severity of the contradiction 0.0–1.0.
    pub severity: f64,
}

/// Category of contradiction between two claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// One claim directly negates the other.
    DirectNegation,
    /// Claims disagree on factual matters.
    FactualDisagreement,
    /// Claims reflect incompatible values or priorities.
    ValueConflict,
}

/// Complete structure map for a page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureGraph {
    /// Page this graph was built for.
    pub page_id: Uuid,
    /// All structure nodes.
    pub nodes: Vec<StructureNode>,
    /// All structure edges.
    pub edges: Vec<StructureEdge>,
    /// Zones of high coherence.
    pub consensus_zones: Vec<ConsensusZone>,
}

impl Default for StructureGraph {
    fn default() -> Self {
        Self {
            page_id: Uuid::nil(),
            nodes: Vec::new(),
            edges: Vec::new(),
            consensus_zones: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argument_role_equality() {
        assert_eq!(ArgumentRole::Claim, ArgumentRole::Claim);
        assert_eq!(ArgumentRole::Evidence, ArgumentRole::Evidence);
        assert_ne!(ArgumentRole::Claim, ArgumentRole::Rebuttal);
    }

    #[test]
    fn test_argument_role_serialization() {
        let role = ArgumentRole::Claim;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"claim\"");
        let deserialized: ArgumentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ArgumentRole::Claim);
    }

    #[test]
    fn test_argument_edge_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ArgumentEdgeType::Supports).unwrap(),
            "\"supports\""
        );
        assert_eq!(
            serde_json::to_string(&ArgumentEdgeType::Refutes).unwrap(),
            "\"refutes\""
        );
        assert_eq!(
            serde_json::to_string(&ArgumentEdgeType::Qualifies).unwrap(),
            "\"qualifies\""
        );
    }

    #[test]
    fn test_fallacy_type_serialization() {
        assert_eq!(
            serde_json::to_string(&FallacyType::StrawMan).unwrap(),
            "\"straw_man\""
        );
        assert_eq!(
            serde_json::to_string(&FallacyType::AdHominem).unwrap(),
            "\"ad_hominem\""
        );
        assert_eq!(
            serde_json::to_string(&FallacyType::FalseDichotomy).unwrap(),
            "\"false_dichotomy\""
        );
    }

    #[test]
    fn test_conflict_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ConflictType::DirectNegation).unwrap(),
            "\"direct_negation\""
        );
        assert_eq!(
            serde_json::to_string(&ConflictType::FactualDisagreement).unwrap(),
            "\"factual_disagreement\""
        );
    }

    #[test]
    fn test_consensus_zone_serialization() {
        let zone = ConsensusZone {
            block_ids: vec![Uuid::new_v4()],
            coherence_score: 0.85,
        };
        let json = serde_json::to_string(&zone).unwrap();
        assert!(json.contains("coherence_score"));
        let deserialized: ConsensusZone = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.coherence_score, 0.85);
    }

    #[test]
    fn test_structure_graph_empty() {
        let graph = StructureGraph::default();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.consensus_zones.is_empty());
    }

    #[test]
    fn test_structure_graph_with_nodes_and_edges() {
        let page_id = Uuid::new_v4();
        let node_a = Uuid::new_v4();
        let node_b = Uuid::new_v4();
        let node_c = Uuid::new_v4();

        let graph = StructureGraph {
            page_id,
            nodes: vec![
                StructureNode {
                    block_id: node_a,
                    role: ArgumentRole::Claim,
                    strength: 0.9,
                    position: Position::Neutral,
                },
                StructureNode {
                    block_id: node_b,
                    role: ArgumentRole::Evidence,
                    strength: 0.8,
                    position: Position::In,
                },
                StructureNode {
                    block_id: node_c,
                    role: ArgumentRole::Rebuttal,
                    strength: 0.5,
                    position: Position::Out,
                },
            ],
            edges: vec![
                StructureEdge {
                    source: node_b,
                    target: node_a,
                    edge_type: ArgumentEdgeType::Supports,
                    confidence: 0.85,
                },
                StructureEdge {
                    source: node_c,
                    target: node_a,
                    edge_type: ArgumentEdgeType::Refutes,
                    confidence: 0.7,
                },
            ],
            consensus_zones: vec![],
        };

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.consensus_zones.is_empty());
    }

    #[test]
    fn test_argument_detection_serialization() {
        let detection = ArgumentDetection {
            classification: ArgumentRole::Claim,
            confidence: 0.75,
            evidence_refs: vec![],
        };
        let json = serde_json::to_string(&detection).unwrap();
        assert!(json.contains("\"classification\":\"claim\""));
        assert!(json.contains("0.75"));
    }

    #[test]
    fn test_detected_fallacy_serialization() {
        let fallacy = DetectedFallacy {
            fallacy_type: FallacyType::StrawMan,
            block_id: Uuid::new_v4(),
            explanation: "Misrepresents opponent's position".to_string(),
        };
        let json = serde_json::to_string(&fallacy).unwrap();
        assert!(json.contains("\"fallacy_type\":\"straw_man\""));
    }

    #[test]
    fn test_contradiction_pair_serialization() {
        let pair = ContradictionPair {
            a: Uuid::new_v4(),
            b: Uuid::new_v4(),
            conflict_type: ConflictType::DirectNegation,
            severity: 0.9,
        };
        let json = serde_json::to_string(&pair).unwrap();
        assert!(json.contains("\"direct_negation\""));
    }
}
