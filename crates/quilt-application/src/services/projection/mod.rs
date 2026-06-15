//! Projection services — V1 contract registry and adapters.
//!
//! This module provides the application-layer implementation of the
//! [`quilt_domain::projection`] domain layer.
//!
//! ## Structure
//!
//! - [`contracts`] — V1 contract definitions (predicate + adapter pairs)
//! - [`registry`] — [`StaticProjectionRegistry::v1()`] implementation of the
//!   [`quilt_domain::projection::registry::ProjectionRegistry`] port

pub mod contracts;
pub mod registry;

pub use registry::StaticProjectionRegistry;
