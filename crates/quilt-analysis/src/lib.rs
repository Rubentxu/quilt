//! Quilt Analysis — Structural Analysis Engine
//!
//! Provides pure structural analysis of the knowledge graph (no AI/LLM):
//! - **StructuralMirror**: Analyzes block reference graphs to produce structural maps
//!   with clusters, density metrics, frontiers, gaps, and influence scores.
//! - **ConnectionEngine**: Discovers unexpected connections between knowledge blocks
//!   using structural and temporal similarity (no AI/LLM dependency).
//! - **AgentStore**: Persistent memory system for AI agents using the existing
//!   BlockRepository infrastructure.
//! - **StructureMapper**: Maps argument structure (claims, evidence, rebuttals)
//!   using heuristic analysis of block content.
//! - **StructureGardener**: Tracks belief evolution, detects contradictions,
//!   and suggests areas for deeper exploration.
//!
//! All modules depend on `quilt-domain` for types and repository traits.
//! Per ADR-0001, Quilt does NOT integrate AI/LLM providers — only structural analysis.

pub mod agent_room;
pub mod agent_store;
pub mod cognitive_dashboard;
pub mod connection_engine;
pub mod decay_monitor;
pub mod morning_briefing;
pub mod serendipity_monitor;
pub mod shared_decay;
pub mod structural_mirror;
pub mod structure_gardener;
pub mod structure_mapper;
pub mod weekly_review;

// Re-export error types for convenience
pub use agent_store::StoreError;
pub use connection_engine::ConnectionError;
pub use structural_mirror::StructuralError;
pub use structure_gardener::StructureGardenerError;
pub use structure_mapper::StructureMapperError;

/// Unified error type for all analysis engines.
/// Each engine's specific error wraps this via conversion.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Block not found: {0}")]
    BlockNotFound(String),

    #[error("Storage error: {0}")]
    Storage(#[from] quilt_domain::errors::DomainError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub use agent_store::AgentStore;
pub use cognitive_dashboard::{CognitiveDashboardService, CognitiveGraphDto};
pub use connection_engine::ConnectionEngine;
pub use decay_monitor::{DecayMonitorDto, DecayMonitorService, SeverityCounts};
pub use morning_briefing::{
    AgendaItem, DecayAlert, MorningBriefing, MorningBriefingDto, SerendipityHighlight,
};
pub use serendipity_monitor::{
    SerendipityHighlightDetail, SerendipityMonitorDto, SerendipityMonitorService,
};
pub use structural_mirror::StructuralMirror;
pub use structure_gardener::{
    Belief, BeliefSnapshot, Contradiction, DeepeningSuggestion, DefaultRegistry, DiagnosisReport,
    GardenerCare, Issue, IssueKind, MentalModel, RepairAction, RepairReport, StructureGardener,
    TemplateDoctor, TemplateState,
};
pub use structure_mapper::{StructureEdge, StructureGraph, StructureMapper, StructureNode};
pub use weekly_review::{DecayTrend, WeeklyReviewDto, WeeklyReviewService};
