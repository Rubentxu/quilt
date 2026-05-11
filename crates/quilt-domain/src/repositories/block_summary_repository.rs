//! BlockSummaryRepository trait - persistence for block summaries

use crate::entities::BlockSummary;
use crate::errors::DomainError;
use crate::value_objects::Uuid;
use async_trait::async_trait;

/// Repository for BlockSummary persistence.
///
/// Block summaries are LLM-generated descriptions of block content
/// used by the TreeRAG engine for document generation.
#[async_trait]
pub trait BlockSummaryRepository: Send + Sync {
    /// Get a summary by block ID
    async fn get(&self, block_id: Uuid) -> Result<Option<BlockSummary>, DomainError>;

    /// Get summaries for multiple blocks at once
    async fn get_batch(&self, block_ids: &[Uuid]) -> Result<Vec<BlockSummary>, DomainError>;

    /// Insert or update a summary
    async fn upsert(&self, summary: &BlockSummary) -> Result<(), DomainError>;

    /// Delete a summary
    async fn delete(&self, block_id: Uuid) -> Result<(), DomainError>;

    /// List block IDs whose summaries are older than the given timestamp
    async fn list_stale(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Uuid>, DomainError>;

    /// Count total summaries
    async fn count(&self) -> Result<usize, DomainError>;

    /// Count summaries generated since a timestamp
    async fn count_since(
        &self,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, DomainError>;
}
