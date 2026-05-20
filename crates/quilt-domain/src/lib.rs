//! Quilt Domain Layer
//!
//! This crate contains the pure domain model with MINIMAL external dependencies.
//! It defines entities, value objects, repository traits, and domain services.
//!
//! # Dependency Policy
//!
//! This crate maintains a MINIMAL dependency footprint:
//! - `async-trait` — required (Rust 2021 lacks native async traits)
//! - `chrono` — deferred (Timestamp migration to native chrono replacement planned for Phase 2)
//! - `serde`/`serde_json` — kept for `PropertyValue` only (genuine JSON serialization need)
//! - All other deps removed (no once_cell, etc.)
//!
//! # Architecture
//!
//! - `entities/` - Domain entities (Block, Page, etc.)
//! - `value_objects/` - Immutable value types (JournalDay, TaskMarker, etc.)
//! - `repositories/` - Repository traits (abstractions, not implementations)
//! - `services/` - Domain services (outliner logic, etc.)
//! - `errors/` - Domain-specific error types
//! - `events/` - Domain events
//! - `properties/` - Typed property system
//! - `classes/` - Entity class system with inheritance
//! - `types/` - Aggregate and composite types (DailySummary, etc.)

pub mod classes;
pub mod content;
pub mod entities;
pub mod errors;
pub mod events;
pub mod properties;
pub mod repositories;
pub mod services;
pub mod types;
pub mod value_objects;

// Re-exports for convenience
pub use classes::{builtin_classes, Class, ClassValidator};
pub use entities::{
    Asset, Block, BlockSummary, DeepLink, File, Journal, Page, ScheduledTask, Tag, TaskType,
    UserSettings,
};
pub use errors::DomainError;
pub use events::{AppEvent, FileChanged, FileEventType};
pub use properties::{
    Cardinality, ClosedValue, PropertyDefinition, PropertyType, PropertyValidator, ViewContext,
};
pub use repositories::{
    BlockSummaryRepository, ClassRepository, DeepLinkRepository, JournalRepository,
    PropertyRepository, ScheduledTaskRepository, SettingsRepository,
};
pub use services::TimezoneService;
pub use types::DailySummary;
pub use value_objects::{
    Align, AssetType, BlockFormat, JournalDay, Priority, PropertyValue, TaskMarker, Uuid,
};
pub use content::{BlockContent, BlockSegment, Mark};
