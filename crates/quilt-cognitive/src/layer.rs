//! Cognitive Layer — High-Level Cognitive Operations
//!
//! Provides a domain-level interface for cognitive operations, building on
//! `CognitiveServices` which exposes individual engine getters. This layer
//! exposes actions like `analyze_mirror`, `find_serendipity`, and `store_memory`.
//!
//! # Example
//!
//! ```text
//! use quilt_cognitive::layer::{CognitiveLayer, RealCognitiveLayer};
//! use std::sync::Arc;
//!
//! async fn example(layer: Arc<dyn CognitiveLayer>) {
//!     let block_uuid = quilt_domain::value_objects::Uuid::new_v4();
//!     let result = layer.analyze_mirror(block_uuid).await;
//! }
//! ```

use crate::agent_memory::{AgentMemory, MemoryEntry};
use crate::cognitive_mirror::{CognitiveError, CognitiveMap};
use crate::registry::CognitiveServices;
use crate::serendipity::{SerendipityConnection, SerendipityQuery};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;

/// Context for serendipity search.
#[derive(Debug, Clone)]
pub struct SerendipityContext {
    /// Block UUID to find connections around.
    pub block_uuid: Uuid,
    /// Maximum number of results to return.
    pub limit: usize,
    /// Minimum confidence threshold [0, 1].
    pub min_confidence: f32,
}

impl Default for SerendipityContext {
    fn default() -> Self {
        Self {
            block_uuid: Uuid::nil(),
            limit: 20,
            min_confidence: 0.3,
        }
    }
}

/// A discovered serendipitous link between blocks.
#[derive(Debug, Clone)]
pub struct SerendipityLink {
    pub source_block: Uuid,
    pub target_block: Uuid,
    pub bridge_concept: Option<String>,
    pub confidence: f32,
    pub connection_type: crate::serendipity::ConnectionType,
}

impl From<SerendipityConnection> for SerendipityLink {
    fn from(conn: SerendipityConnection) -> Self {
        Self {
            source_block: conn.idea_a,
            target_block: conn.idea_b,
            bridge_concept: conn.bridge_concept,
            confidence: conn.confidence,
            connection_type: conn.connection_type,
        }
    }
}

/// Result of cognitive mirror analysis.
#[derive(Debug, Clone)]
pub struct CognitiveMirrorResult {
    pub page_id: Uuid,
    pub cognitive_map: CognitiveMap,
    pub analysis_timestamp: DateTime<Utc>,
}

/// High-level cognitive operations trait.
///
/// This trait provides a simplified interface for cognitive operations,
/// building on the individual engines exposed via `CognitiveServices`.
///
/// Implementations:
/// - `RealCognitiveLayer`: Production implementation delegating to engines
/// - `MockCognitiveLayer`: Testing implementation with hardcoded responses
#[async_trait]
pub trait CognitiveLayer: Send + Sync {
    /// Analyze the cognitive map for the page containing the given block.
    async fn analyze_mirror(&self, block_uuid: Uuid) -> Result<CognitiveMirrorResult, CognitiveError>;

    /// Find serendipitous connections around the given block.
    async fn find_serendipity(&self, context: &SerendipityContext) -> Result<Vec<SerendipityLink>, CognitiveError>;

    /// Store a memory entry for an agent.
    async fn store_memory(&self, entry: MemoryEntry) -> Result<(), CognitiveError>;
}

/// Production implementation of `CognitiveLayer` that delegates to engines
/// via `CognitiveServices` and holds `AgentMemory` directly for store operations.
#[derive(Clone)]
pub struct RealCognitiveLayer {
    cognitive_services: Arc<dyn CognitiveServices>,
    agent_memory: Option<Arc<AgentMemory>>,
}

impl RealCognitiveLayer {
    pub fn new(cognitive_services: Arc<dyn CognitiveServices>) -> Self {
        Self {
            cognitive_services,
            agent_memory: None,
        }
    }

    /// Create a new `RealCognitiveLayer` with an agent memory instance.
    pub fn with_agent_memory(mut self, agent_memory: Arc<AgentMemory>) -> Self {
        self.agent_memory = Some(agent_memory);
        self
    }
}

#[async_trait]
impl CognitiveLayer for RealCognitiveLayer {
    #[tracing::instrument(skip(self))]
    async fn analyze_mirror(&self, block_uuid: Uuid) -> Result<CognitiveMirrorResult, CognitiveError> {
        let mirror = self
            .cognitive_services
            .cognitive_mirror()
            .ok_or(CognitiveError::EngineNotConfigured("CognitiveMirror"))?;

        // The block_uuid is actually a page_id in the current implementation
        // TODO: If block_uuid refers to a block (not page), we need to look up the page_id
        let page_id = block_uuid;

        let cognitive_map = mirror.analyze(page_id).await?;

        Ok(CognitiveMirrorResult {
            page_id,
            cognitive_map,
            analysis_timestamp: Utc::now(),
        })
    }

    #[tracing::instrument(skip(self))]
    async fn find_serendipity(&self, context: &SerendipityContext) -> Result<Vec<SerendipityLink>, CognitiveError> {
        let serendipity_engine = self
            .cognitive_services
            .serendipity_engine()
            .ok_or(CognitiveError::EngineNotConfigured("SerendipityEngine"))?;

        let query = SerendipityQuery {
            topic: None,
            limit: context.limit,
            offset: 0,
            min_confidence: context.min_confidence,
            temporal_window_days: None,
            page_id: Some(context.block_uuid),
        };

        serendipity_engine
            .find_connections(query)
            .await
            .map(|connections| connections.into_iter().map(SerendipityLink::from).collect())
            .map_err(|e| CognitiveError::BlockNotFound(Uuid::nil())) // Convert to CognitiveError
    }

    #[tracing::instrument(skip(self, entry))]
    async fn store_memory(&self, entry: MemoryEntry) -> Result<(), CognitiveError> {
        let agent_memory = self
            .agent_memory
            .as_ref()
            .ok_or(CognitiveError::EngineNotConfigured("AgentMemory"))?;

        // AgentMemory.store is an async method that persists the entry
        agent_memory
            .store(entry)
            .await
            .map_err(|_| CognitiveError::EngineNotConfigured("AgentMemory"))?;
        Ok(())
    }
}

/// Mock implementation of `CognitiveLayer` for testing.
#[derive(Default)]
pub struct MockCognitiveLayer {
    pub analyze_result: Option<CognitiveMirrorResult>,
    pub analyze_error: Option<String>,
    pub serendipity_result: Vec<SerendipityLink>,
    pub serendipity_error: Option<String>,
    pub store_memory_error: Option<String>,
}

impl MockCognitiveLayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the result for `analyze_mirror`.
    pub fn with_analyze_result(mut self, result: CognitiveMirrorResult) -> Self {
        self.analyze_result = Some(result);
        self
    }

    /// Set the error for `analyze_mirror`.
    pub fn with_analyze_error(mut self, error: String) -> Self {
        self.analyze_error = Some(error);
        self
    }

    /// Set the result for `find_serendipity`.
    pub fn with_serendipity_result(mut self, results: Vec<SerendipityLink>) -> Self {
        self.serendipity_result = results;
        self
    }

    /// Set the error for `find_serendipity`.
    pub fn with_serendipity_error(mut self, error: String) -> Self {
        self.serendipity_error = Some(error);
        self
    }

    /// Set the error for `store_memory`.
    pub fn with_store_memory_error(mut self, error: String) -> Self {
        self.store_memory_error = Some(error);
        self
    }
}

#[async_trait]
impl CognitiveLayer for MockCognitiveLayer {
    async fn analyze_mirror(&self, _block_uuid: Uuid) -> Result<CognitiveMirrorResult, CognitiveError> {
        if let Some(ref error) = self.analyze_error {
            return Err(CognitiveError::BlockNotFound(Uuid::nil()));
        }
        self.analyze_result
            .clone()
            .ok_or(CognitiveError::EngineNotConfigured("MockCognitiveLayer"))
    }

    async fn find_serendipity(&self, _context: &SerendipityContext) -> Result<Vec<SerendipityLink>, CognitiveError> {
        if let Some(ref _error) = self.serendipity_error {
            return Err(CognitiveError::BlockNotFound(Uuid::nil()));
        }
        Ok(self.serendipity_result.clone())
    }

    async fn store_memory(&self, _entry: MemoryEntry) -> Result<(), CognitiveError> {
        if let Some(ref _error) = self.store_memory_error {
            return Err(CognitiveError::EngineNotConfigured("AgentMemory"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_memory::MemoryEntry;
    use crate::cognitive_mirror::CognitiveMap;
    use crate::serendipity::ConnectionType;

    #[tokio::test]
    async fn test_mock_analyze_mirror_success() {
        let result = CognitiveMirrorResult {
            page_id: Uuid::new_v4(),
            cognitive_map: CognitiveMap::default(),
            analysis_timestamp: Utc::now(),
        };

        let mock = MockCognitiveLayer::new().with_analyze_result(result.clone());
        let layer = &mock as &dyn CognitiveLayer;

        let got = layer.analyze_mirror(Uuid::new_v4()).await.unwrap();
        assert_eq!(got.page_id, result.page_id);
    }

    #[tokio::test]
    async fn test_mock_analyze_mirror_error() {
        let mock = MockCognitiveLayer::new().with_analyze_error("BlockNotFound".to_string());
        let layer = &mock as &dyn CognitiveLayer;

        let err = layer.analyze_mirror(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, CognitiveError::BlockNotFound(_)));
    }

    #[tokio::test]
    async fn test_mock_serendipity_success() {
        let link = SerendipityLink {
            source_block: Uuid::new_v4(),
            target_block: Uuid::new_v4(),
            bridge_concept: Some("concept".to_string()),
            confidence: 0.8,
            connection_type: ConnectionType::Structural,
        };

        let mock = MockCognitiveLayer::new().with_serendipity_result(vec![link.clone()]);
        let layer = &mock as &dyn CognitiveLayer;

        let context = SerendipityContext::default();
        let got = layer.find_serendipity(&context).await.unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].confidence, 0.8);
    }

    #[tokio::test]
    async fn test_mock_store_memory_success() {
        let mock = MockCognitiveLayer::new();
        let layer = &mock as &dyn CognitiveLayer;

        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            context: "test".to_string(),
            content: "test memory".to_string(),
            importance: 0.8,
            decay_rate: 0.05,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        };

        let result = layer.store_memory(entry).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_store_memory_error() {
        let mock = MockCognitiveLayer::new().with_store_memory_error(
            "AgentMemory not configured".to_string(),
        );
        let layer = &mock as &dyn CognitiveLayer;

        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            context: "test".to_string(),
            content: "test memory".to_string(),
            importance: 0.8,
            decay_rate: 0.05,
            created_at: Utc::now(),
            last_accessed: Utc::now(),
        };

        let err = layer.store_memory(entry).await.unwrap_err();
        assert!(matches!(err, CognitiveError::EngineNotConfigured(_)));
    }
}
