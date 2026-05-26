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
//! - `errors/` - Domain-specific error types

pub mod entities;
pub mod errors;
pub mod repositories;
pub mod services;
pub mod value_objects;

// Re-exports for convenience
pub use entities::{Asset, Block, File, Journal, Page, Tag};
pub use errors::DomainError;
pub use value_objects::{
    Align, AssetType, BlockFormat, JournalDay, Priority, PropertyValue, TaskMarker, Uuid,
};
