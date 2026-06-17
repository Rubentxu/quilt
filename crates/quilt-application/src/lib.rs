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

// Services module — application-level orchestration
pub mod services;

// Use cases module - higher-level operations for presentation layers
pub mod use_cases;

// Template extension types (G6 schema packs, F15 reapply)
pub mod templates;

// Query execution (F18)
pub mod query;

// Migration module (F21) - Quilt markdown import
pub mod migration;

// Property Intelligence (PI-3) — property service
pub mod property;

// Property Schema Templates (PI-7) — schema service
pub mod schema;

// Re-exports
pub use bootstrap::AppServices;
pub use errors::ApplicationError;

// Use case traits (re-exported for convenience)
pub use use_cases::{
    AnnotationDto, AnnotationUseCases, BlockDto, BlockTree, BlockUseCases, GraphSnapshot,
    JournalSummary, MigrationUseCases, PageSummary, PageUseCases, PageWithBlocks, QueryPlan,
    ResourceUseCases, SearchResult, SearchUseCases, TagSummary,
};

// Migration VOs (GS-9)
pub use migration::{
    CandidateStatus, IngestResult, IngestResultEntry, IngestionCandidate, IngestionPlan,
    PlanSummary, ReindexResult, ReindexResultEntry,
};

// Domain types re-exported for use by presentation layers (MCP, REST)
// This allows quilt-mcp to use domain types without a direct quilt-domain dependency
pub use quilt_domain::entities::Block;
pub use quilt_domain::entities::{
    AnnotationScope, AnnotationStatus, ContractError, PropertyKey, TemplateContract,
    TemplateContractBuilder, TemplateLayout, Version,
};
pub use quilt_domain::repositories::AnnotationFilters;
pub use quilt_domain::value_objects::{JournalDay, TaskMarker, Uuid, parse_properties};
