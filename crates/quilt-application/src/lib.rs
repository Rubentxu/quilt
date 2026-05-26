//! Quilt Application Layer
//!
//! Use cases and orchestration logic.
//!
//! # Use Cases
//!
//! The [`use_cases`] module provides higher-level use case traits that
//! encapsulate common workflows. These are used by presentation layers
//! (MCP, REST) to interact with the domain.
//!
//! # Key Types
//!
//! - [`ApplicationError`]: Unified error type for all application-level failures

pub mod bootstrap;
pub mod errors;

// Use cases module - higher-level operations for presentation layers
pub mod use_cases;

// Re-exports
pub use bootstrap::AppServices;
pub use errors::ApplicationError;

// Use case traits (re-exported for convenience)
pub use use_cases::{
    BlockTree, BlockUseCases, GraphSnapshot, JournalSummary, PageSummary, PageUseCases,
    PageWithBlocks, QueryPlan, ResourceUseCases, SearchResult, SearchUseCases, TagSummary,
};

// Domain types re-exported for use by presentation layers (MCP, REST)
// This allows quilt-mcp to use domain types without a direct quilt-domain dependency
pub use quilt_domain::entities::Block;
pub use quilt_domain::value_objects::{parse_properties, JournalDay, TaskMarker, Uuid};
