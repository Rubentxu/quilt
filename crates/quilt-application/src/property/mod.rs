//! Property Intelligence — application services
//!
//! Orchestrates property definition CRUD, batch queries, and search
//! via the `PropertyRepository` trait.

pub mod service;

pub use service::{PropertyService, PropertyServiceTrait, PropertySuggestion};
