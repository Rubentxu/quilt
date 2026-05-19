//! Quilt Cognitive Features
//!
//! Provides AI-driven cognitive analysis of the knowledge graph:
//! - **CognitiveMirror**: Analyzes block reference graphs to produce cognitive maps
//!   with clusters, density metrics, frontiers, gaps, and influence scores.
//! - **SerendipityEngine**: Discovers unexpected connections between knowledge blocks
//!   using structural, temporal, and semantic similarity.
//! - **AgentMemory**: Persistent memory system for AI agents using the existing
//!   BlockRepository infrastructure.
//! - **CounterfactualExplorer**: Explores "what if" scenarios and alternative branches.
//! - **KnowledgeEvolutionTracker**: Tracks how knowledge and beliefs evolve over time.
//!
//! All modules depend on `quilt-domain` for types and repository traits, and use
//! a trait-based `AIClient` for LLM backends.

pub mod agent_memory;
pub mod ai_client;
pub mod argument_cartographer;
pub mod cognitive_mirror;
pub mod counterfactual_explorer;
pub mod knowledge_evolution;
pub mod mental_model_gardener;
pub mod morning_briefing;
pub mod scheduler;
pub mod serendipity;
pub mod tree_rag;

#[cfg(any(feature = "ollama", feature = "openai"))]
pub mod ai_providers;

// Re-exports for convenience
pub use agent_memory::AgentMemory;
pub use ai_client::{AIClient, AIClientError, AIConfig, AIProvider, MockAIClient};
pub use argument_cartographer::{ArgumentCartographer, ArgumentEdge, ArgumentGraph, ArgumentNode};
pub use cognitive_mirror::CognitiveMirror;
pub use counterfactual_explorer::{
    CounterfactualBranch, CounterfactualExplorer, CounterfactualTree,
};
pub use knowledge_evolution::{BeliefChange, KnowledgeEvolutionTracker, KnowledgeTimeline};
pub use mental_model_gardener::{
    Belief, BeliefSnapshot, Contradiction, DeepeningSuggestion, MentalModel, MentalModelGardener,
};
pub use morning_briefing::{
    BriefingStatsDto, CognitivePulseDto, DecayAlertDto, DefaultMorningBriefingServices,
    KnowledgeEvolutionDto, MorningBriefing, MorningBriefingDto, MorningBriefingServices,
    SerendipityHighlightDto,
};
pub use serendipity::SerendipityEngine;
pub use tree_rag::{
    AssembledSection, Citation, FormatCache, GeneratedReport, MldocAst, MldocAstNode, MldocContent,
    ReportRequest, ReportScope, TopicCluster, TreeIndex, TreeNode, TreeRagConfig, TreeRagEngine,
    TreeRagStatus,
};

// Re-export scheduler types when scheduler module is added
pub use scheduler::TaskScheduler;

#[cfg(feature = "ollama")]
pub use ai_providers::{OllamaClient, OllamaConfig};
#[cfg(feature = "openai")]
pub use ai_providers::{OpenAIClient, OpenAIConfig};

// create_ai_client is always available when any provider feature is enabled
#[cfg(any(feature = "ollama", feature = "openai"))]
pub use ai_providers::create_ai_client;
