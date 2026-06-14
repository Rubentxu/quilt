use crate::ast::QueryAst;
use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// Graph-augmented result for intent matches that use graph algorithms.
///
/// Returned when a graph-aware rule (RelatedTo, ConnectedTo, MostCentral, PathBetween)
/// successfully matches and produces graph-based results beyond what the DSL can express.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphResult {
    /// Type of graph result
    pub result_type: GraphResultType,
    /// The target block ID that was used as the center of the query
    pub center_block_id: Option<Uuid>,
    /// Block IDs returned by the graph algorithm
    pub block_ids: Vec<Uuid>,
    /// Human-readable names for the blocks (if available)
    pub block_names: Vec<String>,
    /// For PathBetween: ordered list of block IDs forming the path
    pub path: Option<Vec<Uuid>>,
    /// For PathBetween: distance in hops
    pub distance: Option<i32>,
}

/// Type of graph result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphResultType {
    /// 1-hop or 2-hop neighbors of a block
    Neighbors,
    /// All blocks in the same strongly connected component
    ConnectedComponent,
    /// Blocks with highest eigenvector centrality
    MostCentral,
    /// Shortest path between two blocks
    PathBetween,
}

/// Result of a heuristic intent match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentResult {
    /// The generated QueryAst
    pub ast: QueryAst,
    /// DSL string representation (for user feedback loop)
    pub dsl: String,
    /// Confidence score 0.0–1.0
    pub confidence: f32,
    /// Human-readable explanation of what was detected
    pub explanation: String,
    /// Graph-augmented result (only present for graph-aware rules)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_result: Option<GraphResult>,
}

/// Trait for heuristic intent matching rules.
pub trait IntentRule: Send + Sync {
    /// Try to match the input text. Returns Some((ast, confidence)) if matched.
    fn matches(&self, input: &str) -> Option<(QueryAst, f32)>;
    /// Human-readable name for this rule
    fn name(&self) -> &str;
}

// ─────────────────────────────────────────────────────────────────────────────
// Rule: Task Status (English + Spanish)
// ─────────────────────────────────────────────────────────────────────────────

