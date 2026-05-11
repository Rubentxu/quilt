//! Quilt Infrastructure Layer
//!
//! This crate contains concrete implementations of domain abstractions.
//!
//! # Modules
//!
//! - [`database`]: SQLite-based persistence using sqlx
//! - [`serialization`]: JSON serialization utilities
//! - [`logging`]: Tracing-based logging configuration
//!
//! # Architecture
//!
//! This crate implements the infrastructure concerns from the hexagonal architecture:
//! - Database connection pooling and migration management
//! - Repository pattern implementations for persistence
//! - Serialization of domain types to/from storage formats

// Stubbed out for now - full implementation pending
pub mod database;
pub mod logging;
pub mod serialization;
