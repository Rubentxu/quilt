//! Quilt Application Layer
//!
//! Use cases and orchestration logic implementing the CQRS pattern.
//!
//! # Architecture
//!
//! This crate follows Command Query Responsibility Segregation (CQRS):
//! - **Commands** ([`commands`] module): Write operations that modify domain state
//! - **Queries** ([`queries`] module): Read operations that retrieve domain data
//! - **Query Service** ([`query_service`] module): High-level query DSL parsing and SQL generation
//!
//! # Key Types
//!
//! - [`BlockCommand`] / [`BlockQuery`]: Handlers for block operations
//! - [`PageCommand`] / [`PageQuery`]: Handlers for page operations
//! - [`SyncCommand`]: Handlers for sync operations
//! - [`ApplicationError`]: Unified error type for all application-level failures

pub mod commands;
pub mod errors;
#[cfg(feature = "tokio-runtime")]
pub mod event_bridge;
#[cfg(feature = "tokio-runtime")]
pub mod event_bus;
pub mod queries;
pub mod services;

pub mod query_service;

pub mod sync_commands;
pub mod sync_service;

// Re-exports
pub use commands::{BlockCommand, BlockCommandHandler, PageCommand, PageCommandHandler};
pub use errors::ApplicationError;
pub use queries::{BlockQuery, BlockQueryHandler, PageQuery, PageQueryHandler};
pub use sync_commands::{SyncCommand, SyncCommandHandler, SyncResult};
pub use sync_service::{ConflictInfo, SyncService, SyncServiceConfig, SyncServiceState};
