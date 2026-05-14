//! Cognitive engine types for Quilt MCP server.
//!
//! DTOs for cognitive engine status and operations.

use serde::{Deserialize, Serialize};

/// Status of all cognitive engines in the MCP server.
///
/// Returned by [`McpServer::cognitive_engine_status`] to allow Tauri commands
/// to check availability without triggering engine initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveEngineStatus {
    /// Whether the CognitiveMirror engine is available.
    pub cognitive_mirror: bool,
    /// Whether the SerendipityEngine is available.
    pub serendipity_engine: bool,
    /// Whether the AgentMemory is available.
    pub agent_memory: bool,
    /// Whether the ArgumentCartographer is available.
    pub argument_cartographer: bool,
    /// Whether the MentalModelGardener is available.
    pub mental_model_gardener: bool,
    /// Whether the CounterfactualExplorer is available.
    pub counterfactual_explorer: bool,
    /// Whether the KnowledgeEvolutionTracker is available.
    pub knowledge_evolution_tracker: bool,
    /// Whether the MorningBriefing service is available.
    pub morning_briefing: bool,
    /// Whether the TreeRAG engine is available.
    pub tree_rag: bool,
    /// Whether the TaskScheduler is available.
    pub task_scheduler: bool,
}
