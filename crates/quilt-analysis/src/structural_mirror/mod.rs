//! Structural Mirror — Knowledge Graph Analysis
//!
//! Analyzes a page's block reference graph to produce a `StructureMap` with:
//! - **Clusters**: Groups of densely-connected blocks (connected components)
//! - **Density**: Reference density per block
//! - **Frontiers**: Blocks with many outgoing but few incoming refs
//! - **Gaps**: Structural gaps — pairs sharing common refs but no direct connection
//! - **Influence**: PageRank-lite influence scores

mod engine;
mod graph;
mod types;

pub use engine::{StructuralError, StructuralMirror};
pub use graph::LightweightGraph;
pub use types::{InfluenceScore, KnowledgeCluster, KnowledgeGap, StructureMap};
