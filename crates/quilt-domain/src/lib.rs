//! Quilt Domain Layer
//!
//! This crate contains the pure domain model with ZERO external dependencies.
//! It defines entities, value objects, repository traits, and domain services.
//!
//! # Architecture
//!
//! - `entities/` - Domain entities (Block, Page, etc.)
//! - `value_objects/` - Immutable value types (JournalDay, TaskMarker, etc.)
//! - `repositories/` - Repository traits (abstractions, not implementations)
//! - `services/` - Domain services (outliner logic, etc.)
//! - `references/` - Reference model (Ref, RefType, RefIndex)
//! - `errors/` - Domain-specific error types

pub mod canonicalization;
pub mod content;
pub mod entities;
pub mod errors;
pub mod events;
pub mod projection;
pub mod properties;
pub mod references;
pub mod repositories;
pub mod services;
pub mod value_objects;

// Re-exports for convenience
pub use canonicalization::{
    CanonicalInput, CanonicalizationResult, Canonicalizer, PatchOutcome, ProjectionConflict,
    PropertyDefinitionRegistry, PropertyPatch, PropertyPatchProvenance, SourceKind,
};
pub use content::{BlockContent, BlockSegment, Mark};
pub use entities::{Asset, Block, File, Journal, Page, PropertyKey, Tag};
pub use errors::DomainError;
pub use projection::{
    Decoration, DecorationKind, DefaultProjection, LinkKind, LinkView, Projection,
    ProjectionContext, ProjectionContract, ProjectionContractId, ProjectionRegistry,
    ProjectionView, ProjectionViewBuilder, ProjectionViewDelta, PropertyPredicate,
    RegisteredProjection,
};
pub use references::{Ref, RefIndex, RefType};
pub use services::OrderCalculator;
pub use value_objects::{
    Align, AssetType, BlockFormat, BlockType, JournalDay, Priority, PropertyValue, TaskMarker, Uuid,
};
