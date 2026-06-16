//! Cognitive Dashboard — global knowledge graph analysis
//!
//! Provides a global (cross-page) view of the knowledge graph for the
//! Cognitive Dashboard / Graph View feature (CG-2).
//!
//! Produces a graph of all blocks in the system with clusters, frontier
//! nodes (highly connected), and gap nodes (isolated).

pub mod service;
pub mod types;

pub use service::CognitiveDashboardService;
pub use types::CognitiveGraphDto;
