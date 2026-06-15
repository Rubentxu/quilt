//! Schema module — portable property and class schema types
//!
//! Extracted from `quilt-domain` for WASM-target compilation.
//! Contains ONLY pure types and synchronous validation — no async, no repositories.
//!
//! # Pure types (extracted from quilt-domain)
//! - PropertyType, Cardinality, ViewContext enums
//! - ClosedValue, PropertyDefinition structs
//! - Class struct + builtin classes
//! - Sync validation (type/cardinality/closed-set checking)

pub mod classes;
pub mod properties;

pub use classes::Class;
pub use properties::{
    Cardinality, ClosedValue, DerivedSource, MergePolicy, PropertyDefinition, PropertyMutability,
    PropertyType, PropertyVisibility, ViewContext,
};
