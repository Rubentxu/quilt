//! GraphSpaceRepository trait - abstraction for graph space metadata persistence

use crate::entities::GraphSpace;
use crate::errors::DomainError;
use async_trait::async_trait;

/// Repository for graph space metadata persistence.
///
/// Implementations must ensure thread-safe access to the singleton settings row.
#[async_trait]
pub trait GraphSpaceRepository: Send + Sync {
    /// Get the current graph space metadata.
    ///
    /// If no settings exist, returns default metadata.
    async fn get_graph_space(&self) -> Result<GraphSpace, DomainError>;

    /// Update graph space metadata.
    ///
    /// This replaces ALL metadata fields. Partial updates should
    /// be done by getting current metadata, modifying, and saving.
    async fn update_graph_space(&self, graph_space: &GraphSpace) -> Result<(), DomainError>;
}
