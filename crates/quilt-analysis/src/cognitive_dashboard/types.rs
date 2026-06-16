//! Types for the Cognitive Dashboard graph view.

use serde::{Deserialize, Serialize};

/// A node in the cognitive graph — represents a block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content_preview: String,
    /// Influence score [0, 1], higher = more central.
    pub influence_score: f32,
    /// Whether this node is a frontier (highly connected hub).
    pub is_frontier: bool,
    /// Whether this node is isolated (gap / orphan).
    pub is_gap: bool,
    /// Cluster ID if the node belongs to a cluster, else null.
    pub cluster_id: Option<String>,
}

/// An edge in the cognitive graph — represents a reference between blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

/// A detected knowledge cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphCluster {
    pub id: String,
    pub block_ids: Vec<String>,
    pub theme: Option<String>,
    pub coherence_score: f32,
}

/// Response body for `GET /api/v1/cognitive/graph`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveGraphDto {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub clusters: Vec<GraphCluster>,
    pub frontier_nodes: Vec<String>,
    pub gap_nodes: Vec<String>,
    pub generated_at: String,
}

impl Default for CognitiveGraphDto {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            clusters: Vec::new(),
            frontier_nodes: Vec::new(),
            gap_nodes: Vec::new(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
