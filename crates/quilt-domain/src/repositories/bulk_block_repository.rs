//! BulkBlockRepository — trait for operations that need all blocks at once.
//!
//! Follows ISP: only 2 callers need `get_all()`, but 10 implementations were
//! forced to stub it. This separates the concern into its own trait.
//!
//! Callers:
//! - [`BlockUseCases::get_all_blocks`](crate::use_cases::BlockUseCases)
//! - Graph algorithms in `quilt-mcp` that build the full reference graph

use crate::entities::Block;
use crate::errors::DomainError;
use async_trait::async_trait;

/// Trait for repositories that support bulk retrieval of all blocks.
///
/// Used by graph algorithms (shortest path, components, centrality) that
/// require the full block reference graph. Most repositories only need
/// targeted queries via [`BlockRepository`](super::BlockRepository).
#[async_trait]
pub trait BulkBlockRepository: Send + Sync {
    /// Get all blocks in the repository with their outgoing references.
    ///
    /// Used by graph algorithms that need the full block reference graph
    /// (shortest path, components, centrality).
    async fn get_all(&self) -> Result<Vec<Block>, DomainError>;
}