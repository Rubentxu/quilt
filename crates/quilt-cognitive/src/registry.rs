//! Cognitive Services Registry
//!
//! Provides a unified interface for all cognitive engines, enabling:
//! - Simplified dependency injection into McpServer
//! - Mock implementations for testing
//! - Dynamic composition of cognitive services

use crate::agent_memory::AgentMemoryEngine;
use crate::argument_cartographer::ArgumentCartographerEngine;
use crate::cognitive_mirror::CognitiveMirrorEngine;
use crate::counterfactual_explorer::CounterfactualExplorerEngine;
use crate::knowledge_evolution::KnowledgeEvolutionEngine;
use crate::mental_model_gardener::MentalModelGardenerEngine;
use crate::morning_briefing::MorningBriefingEngine;
use crate::scheduler::TaskSchedulerEngine;
use crate::serendipity::SerendipityEngineTrait;
use crate::tree_rag::TreeRagEngineTrait;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

/// Unified trait exposing all cognitive engine capabilities.
///
/// All getters return Option since individual engines may be optional.
/// This trait enables:
/// - Simplified dependency injection into McpServer (single field vs 10)
/// - Mock implementations for testing
/// - Dynamic composition of cognitive services
///
/// # Example
///
/// ```ignore
/// use quilt_cognitive::CognitiveServices;
///
/// fn process_cognitive(cs: &dyn CognitiveServices) {
///     if let Some(mirror) = cs.cognitive_mirror() {
///         // use mirror
///     }
/// }
/// ```
#[async_trait]
pub trait CognitiveServices: Send + Sync {
    /// Get the CognitiveMirror engine if available.
    fn cognitive_mirror(&self) -> Option<&dyn CognitiveMirrorEngine>;

    /// Get the SerendipityEngine if available.
    fn serendipity_engine(&self) -> Option<&dyn SerendipityEngineTrait>;

    /// Get the AgentMemory engine if available.
    fn agent_memory(&self) -> Option<&dyn AgentMemoryEngine>;

    /// Get the ArgumentCartographer engine if available.
    fn argument_cartographer(&self) -> Option<&dyn ArgumentCartographerEngine>;

    /// Get the MentalModelGardener engine if available.
    fn mental_model_gardener(&self) -> Option<&dyn MentalModelGardenerEngine>;

    /// Get the CounterfactualExplorer engine if available.
    fn counterfactual_explorer(&self) -> Option<&dyn CounterfactualExplorerEngine>;

    /// Get the KnowledgeEvolutionTracker engine if available.
    fn knowledge_evolution_tracker(&self) -> Option<&dyn KnowledgeEvolutionEngine>;

    /// Get the MorningBriefing engine if available.
    fn morning_briefing(&self) -> Option<&dyn MorningBriefingEngine>;

    /// Get the TreeRAG engine if available.
    fn tree_rag(&self) -> Option<&dyn TreeRagEngineTrait>;

    /// Get the TaskScheduler engine if available.
    fn task_scheduler(&self) -> Option<&dyn TaskSchedulerEngine>;
}

/// Default implementation holding all cognitive engines as optional fields.
///
/// Use the builder pattern to construct:
///
/// ```
/// use quilt_cognitive::registry::CognitiveServicesRegistry;
///
/// // All engines are optional - start with None
/// let registry = CognitiveServicesRegistry::builder()
///     .with_cognitive_mirror(None)
///     .with_serendipity_engine(None)
///     .build();
/// ```
#[derive(Clone)]
pub struct CognitiveServicesRegistry {
    cognitive_mirror: Option<Arc<dyn CognitiveMirrorEngine>>,
    serendipity_engine: Option<Arc<dyn SerendipityEngineTrait>>,
    agent_memory: Option<Arc<dyn AgentMemoryEngine>>,
    argument_cartographer: Option<Arc<dyn ArgumentCartographerEngine>>,
    mental_model_gardener: Option<Arc<dyn MentalModelGardenerEngine>>,
    counterfactual_explorer: Option<Arc<dyn CounterfactualExplorerEngine>>,
    knowledge_evolution_tracker: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    morning_briefing: Option<Arc<dyn MorningBriefingEngine>>,
    tree_rag: Option<Arc<dyn TreeRagEngineTrait>>,
    task_scheduler: Option<Arc<dyn TaskSchedulerEngine>>,
}

impl Debug for CognitiveServicesRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CognitiveServicesRegistry")
            .field("cognitive_mirror", &self.cognitive_mirror.is_some())
            .field("serendipity_engine", &self.serendipity_engine.is_some())
            .field("agent_memory", &self.agent_memory.is_some())
            .field("argument_cartographer", &self.argument_cartographer.is_some())
            .field("mental_model_gardener", &self.mental_model_gardener.is_some())
            .field("counterfactual_explorer", &self.counterfactual_explorer.is_some())
            .field("knowledge_evolution_tracker", &self.knowledge_evolution_tracker.is_some())
            .field("morning_briefing", &self.morning_briefing.is_some())
            .field("tree_rag", &self.tree_rag.is_some())
            .field("task_scheduler", &self.task_scheduler.is_some())
            .finish()
    }
}

#[async_trait]
impl CognitiveServices for CognitiveServicesRegistry {
    fn cognitive_mirror(&self) -> Option<&dyn CognitiveMirrorEngine> {
        self.cognitive_mirror.as_ref().map(|arc| arc.as_ref())
    }

    fn serendipity_engine(&self) -> Option<&dyn SerendipityEngineTrait> {
        self.serendipity_engine.as_ref().map(|arc| arc.as_ref())
    }

    fn agent_memory(&self) -> Option<&dyn AgentMemoryEngine> {
        self.agent_memory.as_ref().map(|arc| arc.as_ref())
    }

    fn argument_cartographer(&self) -> Option<&dyn ArgumentCartographerEngine> {
        self.argument_cartographer.as_ref().map(|arc| arc.as_ref())
    }

    fn mental_model_gardener(&self) -> Option<&dyn MentalModelGardenerEngine> {
        self.mental_model_gardener.as_ref().map(|arc| arc.as_ref())
    }

    fn counterfactual_explorer(&self) -> Option<&dyn CounterfactualExplorerEngine> {
        self.counterfactual_explorer.as_ref().map(|arc| arc.as_ref())
    }

    fn knowledge_evolution_tracker(&self) -> Option<&dyn KnowledgeEvolutionEngine> {
        self.knowledge_evolution_tracker.as_ref().map(|arc| arc.as_ref())
    }

    fn morning_briefing(&self) -> Option<&dyn MorningBriefingEngine> {
        self.morning_briefing.as_ref().map(|arc| arc.as_ref())
    }

    fn tree_rag(&self) -> Option<&dyn TreeRagEngineTrait> {
        self.tree_rag.as_ref().map(|arc| arc.as_ref())
    }

    fn task_scheduler(&self) -> Option<&dyn TaskSchedulerEngine> {
        self.task_scheduler.as_ref().map(|arc| arc.as_ref())
    }
}

impl CognitiveServicesRegistry {
    /// Create a new empty registry with no cognitive services.
    pub fn new() -> Self {
        Self {
            cognitive_mirror: None,
            serendipity_engine: None,
            agent_memory: None,
            argument_cartographer: None,
            mental_model_gardener: None,
            counterfactual_explorer: None,
            knowledge_evolution_tracker: None,
            morning_briefing: None,
            tree_rag: None,
            task_scheduler: None,
        }
    }

    /// Get a builder for CognitiveServicesRegistry.
    pub fn builder() -> CognitiveServicesRegistryBuilder {
        CognitiveServicesRegistryBuilder::new()
    }

    /// Set the CognitiveMirror engine.
    pub fn with_cognitive_mirror(
        mut self,
        engine: Option<Arc<dyn CognitiveMirrorEngine>>,
    ) -> Self {
        self.cognitive_mirror = engine;
        self
    }

    /// Set the SerendipityEngine.
    pub fn with_serendipity_engine(
        mut self,
        engine: Option<Arc<dyn SerendipityEngineTrait>>,
    ) -> Self {
        self.serendipity_engine = engine;
        self
    }

    /// Set the AgentMemory engine.
    pub fn with_agent_memory(mut self, engine: Option<Arc<dyn AgentMemoryEngine>>) -> Self {
        self.agent_memory = engine;
        self
    }

    /// Set the ArgumentCartographer engine.
    pub fn with_argument_cartographer(
        mut self,
        engine: Option<Arc<dyn ArgumentCartographerEngine>>,
    ) -> Self {
        self.argument_cartographer = engine;
        self
    }

    /// Set the MentalModelGardener engine.
    pub fn with_mental_model_gardener(
        mut self,
        engine: Option<Arc<dyn MentalModelGardenerEngine>>,
    ) -> Self {
        self.mental_model_gardener = engine;
        self
    }

    /// Set the CounterfactualExplorer engine.
    pub fn with_counterfactual_explorer(
        mut self,
        engine: Option<Arc<dyn CounterfactualExplorerEngine>>,
    ) -> Self {
        self.counterfactual_explorer = engine;
        self
    }

    /// Set the KnowledgeEvolutionTracker engine.
    pub fn with_knowledge_evolution_tracker(
        mut self,
        engine: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    ) -> Self {
        self.knowledge_evolution_tracker = engine;
        self
    }

    /// Set the MorningBriefing engine.
    pub fn with_morning_briefing(
        mut self,
        engine: Option<Arc<dyn MorningBriefingEngine>>,
    ) -> Self {
        self.morning_briefing = engine;
        self
    }

    /// Set the TreeRAG engine.
    pub fn with_tree_rag(mut self, engine: Option<Arc<dyn TreeRagEngineTrait>>) -> Self {
        self.tree_rag = engine;
        self
    }

    /// Set the TaskScheduler engine.
    pub fn with_task_scheduler(
        mut self,
        engine: Option<Arc<dyn TaskSchedulerEngine>>,
    ) -> Self {
        self.task_scheduler = engine;
        self
    }
}

impl Default for CognitiveServicesRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for CognitiveServicesRegistry.
#[derive(Default)]
pub struct CognitiveServicesRegistryBuilder {
    cognitive_mirror: Option<Arc<dyn CognitiveMirrorEngine>>,
    serendipity_engine: Option<Arc<dyn SerendipityEngineTrait>>,
    agent_memory: Option<Arc<dyn AgentMemoryEngine>>,
    argument_cartographer: Option<Arc<dyn ArgumentCartographerEngine>>,
    mental_model_gardener: Option<Arc<dyn MentalModelGardenerEngine>>,
    counterfactual_explorer: Option<Arc<dyn CounterfactualExplorerEngine>>,
    knowledge_evolution_tracker: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    morning_briefing: Option<Arc<dyn MorningBriefingEngine>>,
    tree_rag: Option<Arc<dyn TreeRagEngineTrait>>,
    task_scheduler: Option<Arc<dyn TaskSchedulerEngine>>,
}

impl CognitiveServicesRegistryBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the CognitiveMirror engine.
    pub fn with_cognitive_mirror(
        mut self,
        engine: Option<Arc<dyn CognitiveMirrorEngine>>,
    ) -> Self {
        self.cognitive_mirror = engine;
        self
    }

    /// Set the SerendipityEngine.
    pub fn with_serendipity_engine(
        mut self,
        engine: Option<Arc<dyn SerendipityEngineTrait>>,
    ) -> Self {
        self.serendipity_engine = engine;
        self
    }

    /// Set the AgentMemory engine.
    pub fn with_agent_memory(mut self, engine: Option<Arc<dyn AgentMemoryEngine>>) -> Self {
        self.agent_memory = engine;
        self
    }

    /// Set the ArgumentCartographer engine.
    pub fn with_argument_cartographer(
        mut self,
        engine: Option<Arc<dyn ArgumentCartographerEngine>>,
    ) -> Self {
        self.argument_cartographer = engine;
        self
    }

    /// Set the MentalModelGardener engine.
    pub fn with_mental_model_gardener(
        mut self,
        engine: Option<Arc<dyn MentalModelGardenerEngine>>,
    ) -> Self {
        self.mental_model_gardener = engine;
        self
    }

    /// Set the CounterfactualExplorer engine.
    pub fn with_counterfactual_explorer(
        mut self,
        engine: Option<Arc<dyn CounterfactualExplorerEngine>>,
    ) -> Self {
        self.counterfactual_explorer = engine;
        self
    }

    /// Set the KnowledgeEvolutionTracker engine.
    pub fn with_knowledge_evolution_tracker(
        mut self,
        engine: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    ) -> Self {
        self.knowledge_evolution_tracker = engine;
        self
    }

    /// Set the MorningBriefing engine.
    pub fn with_morning_briefing(
        mut self,
        engine: Option<Arc<dyn MorningBriefingEngine>>,
    ) -> Self {
        self.morning_briefing = engine;
        self
    }

    /// Set the TreeRAG engine.
    pub fn with_tree_rag(mut self, engine: Option<Arc<dyn TreeRagEngineTrait>>) -> Self {
        self.tree_rag = engine;
        self
    }

    /// Set the TaskScheduler engine.
    pub fn with_task_scheduler(
        mut self,
        engine: Option<Arc<dyn TaskSchedulerEngine>>,
    ) -> Self {
        self.task_scheduler = engine;
        self
    }

    /// Build the CognitiveServicesRegistry.
    pub fn build(self) -> CognitiveServicesRegistry {
        CognitiveServicesRegistry {
            cognitive_mirror: self.cognitive_mirror,
            serendipity_engine: self.serendipity_engine,
            agent_memory: self.agent_memory,
            argument_cartographer: self.argument_cartographer,
            mental_model_gardener: self.mental_model_gardener,
            counterfactual_explorer: self.counterfactual_explorer,
            knowledge_evolution_tracker: self.knowledge_evolution_tracker,
            morning_briefing: self.morning_briefing,
            tree_rag: self.tree_rag,
            task_scheduler: self.task_scheduler,
        }
    }
}

/// Mock implementation of CognitiveServices for testing.
///
/// All engines return None by default, but can be configured
/// with specific mock engines for targeted testing.
///
/// # Example
///
/// ```ignore
/// use quilt_cognitive::registry::{CognitiveServices, MockCognitiveServices};
/// use std::sync::Arc;
///
/// let mock = MockCognitiveServices::builder()
///     .with_cognitive_mirror(Some(Arc::new(mock_mirror)))
///     .build();
/// ```
#[derive(Default)]
pub struct MockCognitiveServices {
    cognitive_mirror: Option<Arc<dyn CognitiveMirrorEngine>>,
    serendipity_engine: Option<Arc<dyn SerendipityEngineTrait>>,
    agent_memory: Option<Arc<dyn AgentMemoryEngine>>,
    argument_cartographer: Option<Arc<dyn ArgumentCartographerEngine>>,
    mental_model_gardener: Option<Arc<dyn MentalModelGardenerEngine>>,
    counterfactual_explorer: Option<Arc<dyn CounterfactualExplorerEngine>>,
    knowledge_evolution_tracker: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    morning_briefing: Option<Arc<dyn MorningBriefingEngine>>,
    tree_rag: Option<Arc<dyn TreeRagEngineTrait>>,
    task_scheduler: Option<Arc<dyn TaskSchedulerEngine>>,
}

impl Debug for MockCognitiveServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockCognitiveServices")
            .field("cognitive_mirror", &self.cognitive_mirror.is_some())
            .field("serendipity_engine", &self.serendipity_engine.is_some())
            .field("agent_memory", &self.agent_memory.is_some())
            .field("argument_cartographer", &self.argument_cartographer.is_some())
            .field("mental_model_gardener", &self.mental_model_gardener.is_some())
            .field("counterfactual_explorer", &self.counterfactual_explorer.is_some())
            .field("knowledge_evolution_tracker", &self.knowledge_evolution_tracker.is_some())
            .field("morning_briefing", &self.morning_briefing.is_some())
            .field("tree_rag", &self.tree_rag.is_some())
            .field("task_scheduler", &self.task_scheduler.is_some())
            .finish()
    }
}

impl MockCognitiveServices {
    /// Create a new mock with no engines configured.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a builder for MockCognitiveServices.
    pub fn builder() -> MockCognitiveServicesBuilder {
        MockCognitiveServicesBuilder::new()
    }
}

impl Default for MockCognitiveServicesBuilder {
    fn default() -> Self {
        Self {
            cognitive_mirror: None,
            serendipity_engine: None,
            agent_memory: None,
            argument_cartographer: None,
            mental_model_gardener: None,
            counterfactual_explorer: None,
            knowledge_evolution_tracker: None,
            morning_briefing: None,
            tree_rag: None,
            task_scheduler: None,
        }
    }
}

#[async_trait]
impl CognitiveServices for MockCognitiveServices {
    fn cognitive_mirror(&self) -> Option<&dyn CognitiveMirrorEngine> {
        self.cognitive_mirror.as_ref().map(|arc| arc.as_ref())
    }

    fn serendipity_engine(&self) -> Option<&dyn SerendipityEngineTrait> {
        self.serendipity_engine.as_ref().map(|arc| arc.as_ref())
    }

    fn agent_memory(&self) -> Option<&dyn AgentMemoryEngine> {
        self.agent_memory.as_ref().map(|arc| arc.as_ref())
    }

    fn argument_cartographer(&self) -> Option<&dyn ArgumentCartographerEngine> {
        self.argument_cartographer.as_ref().map(|arc| arc.as_ref())
    }

    fn mental_model_gardener(&self) -> Option<&dyn MentalModelGardenerEngine> {
        self.mental_model_gardener.as_ref().map(|arc| arc.as_ref())
    }

    fn counterfactual_explorer(&self) -> Option<&dyn CounterfactualExplorerEngine> {
        self.counterfactual_explorer.as_ref().map(|arc| arc.as_ref())
    }

    fn knowledge_evolution_tracker(&self) -> Option<&dyn KnowledgeEvolutionEngine> {
        self.knowledge_evolution_tracker.as_ref().map(|arc| arc.as_ref())
    }

    fn morning_briefing(&self) -> Option<&dyn MorningBriefingEngine> {
        self.morning_briefing.as_ref().map(|arc| arc.as_ref())
    }

    fn tree_rag(&self) -> Option<&dyn TreeRagEngineTrait> {
        self.tree_rag.as_ref().map(|arc| arc.as_ref())
    }

    fn task_scheduler(&self) -> Option<&dyn TaskSchedulerEngine> {
        self.task_scheduler.as_ref().map(|arc| arc.as_ref())
    }
}

/// Builder for MockCognitiveServices.
pub struct MockCognitiveServicesBuilder {
    cognitive_mirror: Option<Arc<dyn CognitiveMirrorEngine>>,
    serendipity_engine: Option<Arc<dyn SerendipityEngineTrait>>,
    agent_memory: Option<Arc<dyn AgentMemoryEngine>>,
    argument_cartographer: Option<Arc<dyn ArgumentCartographerEngine>>,
    mental_model_gardener: Option<Arc<dyn MentalModelGardenerEngine>>,
    counterfactual_explorer: Option<Arc<dyn CounterfactualExplorerEngine>>,
    knowledge_evolution_tracker: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    morning_briefing: Option<Arc<dyn MorningBriefingEngine>>,
    tree_rag: Option<Arc<dyn TreeRagEngineTrait>>,
    task_scheduler: Option<Arc<dyn TaskSchedulerEngine>>,
}

impl MockCognitiveServicesBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the CognitiveMirror engine.
    pub fn with_cognitive_mirror(
        mut self,
        engine: Option<Arc<dyn CognitiveMirrorEngine>>,
    ) -> Self {
        self.cognitive_mirror = engine;
        self
    }

    /// Set the SerendipityEngine.
    pub fn with_serendipity_engine(
        mut self,
        engine: Option<Arc<dyn SerendipityEngineTrait>>,
    ) -> Self {
        self.serendipity_engine = engine;
        self
    }

    /// Set the AgentMemory engine.
    pub fn with_agent_memory(mut self, engine: Option<Arc<dyn AgentMemoryEngine>>) -> Self {
        self.agent_memory = engine;
        self
    }

    /// Set the ArgumentCartographer engine.
    pub fn with_argument_cartographer(
        mut self,
        engine: Option<Arc<dyn ArgumentCartographerEngine>>,
    ) -> Self {
        self.argument_cartographer = engine;
        self
    }

    /// Set the MentalModelGardener engine.
    pub fn with_mental_model_gardener(
        mut self,
        engine: Option<Arc<dyn MentalModelGardenerEngine>>,
    ) -> Self {
        self.mental_model_gardener = engine;
        self
    }

    /// Set the CounterfactualExplorer engine.
    pub fn with_counterfactual_explorer(
        mut self,
        engine: Option<Arc<dyn CounterfactualExplorerEngine>>,
    ) -> Self {
        self.counterfactual_explorer = engine;
        self
    }

    /// Set the KnowledgeEvolutionTracker engine.
    pub fn with_knowledge_evolution_tracker(
        mut self,
        engine: Option<Arc<dyn KnowledgeEvolutionEngine>>,
    ) -> Self {
        self.knowledge_evolution_tracker = engine;
        self
    }

    /// Set the MorningBriefing engine.
    pub fn with_morning_briefing(
        mut self,
        engine: Option<Arc<dyn MorningBriefingEngine>>,
    ) -> Self {
        self.morning_briefing = engine;
        self
    }

    /// Set the TreeRAG engine.
    pub fn with_tree_rag(mut self, engine: Option<Arc<dyn TreeRagEngineTrait>>) -> Self {
        self.tree_rag = engine;
        self
    }

    /// Set the TaskScheduler engine.
    pub fn with_task_scheduler(
        mut self,
        engine: Option<Arc<dyn TaskSchedulerEngine>>,
    ) -> Self {
        self.task_scheduler = engine;
        self
    }

    /// Build the MockCognitiveServices.
    pub fn build(self) -> MockCognitiveServices {
        MockCognitiveServices {
            cognitive_mirror: self.cognitive_mirror,
            serendipity_engine: self.serendipity_engine,
            agent_memory: self.agent_memory,
            argument_cartographer: self.argument_cartographer,
            mental_model_gardener: self.mental_model_gardener,
            counterfactual_explorer: self.counterfactual_explorer,
            knowledge_evolution_tracker: self.knowledge_evolution_tracker,
            morning_briefing: self.morning_briefing,
            tree_rag: self.tree_rag,
            task_scheduler: self.task_scheduler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use crate::cognitive_mirror::CognitiveMap;
    use crate::cognitive_mirror::CognitiveMirrorEngine;
    use quilt_domain::value_objects::Uuid;

    /// A mock CognitiveMirrorEngine for testing that always succeeds.
    #[derive(Default, Debug)]
    struct MockMirrorEngine;

    #[async_trait]
    impl CognitiveMirrorEngine for MockMirrorEngine {
        async fn analyze(
            &self,
            _page_id: Uuid,
        ) -> Result<CognitiveMap, crate::cognitive_mirror::CognitiveError> {
            Ok(CognitiveMap {
                clusters: vec![],
                density: std::collections::HashMap::new(),
                frontiers: vec![],
                gaps: vec![],
                influences: vec![],
            })
        }
    }

    #[test]
    fn test_registry_with_no_engines() {
        let registry = CognitiveServicesRegistry::new();
        assert!(registry.cognitive_mirror().is_none());
        assert!(registry.serendipity_engine().is_none());
    }

    #[test]
    fn test_registry_builder() {
        let mirror = Arc::new(MockMirrorEngine::default()) as Arc<dyn CognitiveMirrorEngine>;
        let registry = CognitiveServicesRegistry::builder()
            .with_cognitive_mirror(Some(mirror))
            .build();

        assert!(registry.cognitive_mirror().is_some());
        assert!(registry.serendipity_engine().is_none());
    }

    #[test]
    fn test_mock_with_engine() {
        let mirror = Arc::new(MockMirrorEngine::default()) as Arc<dyn CognitiveMirrorEngine>;
        let mock = MockCognitiveServices::builder()
            .with_cognitive_mirror(Some(mirror))
            .build();

        assert!(mock.cognitive_mirror().is_some());
        assert!(mock.serendipity_engine().is_none());
    }

    #[test]
    fn test_mock_with_all_engines() {
        // This test verifies all engine slots can be set
        let mirror: Option<Arc<dyn CognitiveMirrorEngine>> = None;
        let mock = MockCognitiveServices::builder()
            .with_cognitive_mirror(mirror.clone())
            .with_serendipity_engine(None)
            .with_agent_memory(None)
            .with_argument_cartographer(None)
            .with_mental_model_gardener(None)
            .with_counterfactual_explorer(None)
            .with_knowledge_evolution_tracker(None)
            .with_morning_briefing(None)
            .with_tree_rag(None)
            .with_task_scheduler(None)
            .build();

        // All should be None since we passed None
        assert!(mock.cognitive_mirror().is_none());
        assert!(mock.serendipity_engine().is_none());
    }
}
