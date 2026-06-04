//! Properties module - typed property system
//!
//! This module provides the typed property system for Quilt, including:
//! - PropertyType enum for type safety
//! - PropertyDefinition for schema definition
//! - Builtin properties (status, priority, deadline, scheduled, url)
//! - PropertyValidator for runtime validation
//! - PropertyEntry trait hierarchy for OCP-compliant metadata extension
//!
//! ## Module structure
//!
//! - [`entry`]: `HasValue` / `HasTimestamp` / `Mergeable` / `PropertyEntry` traits
//!   and the `DefaultPropertyEntry<V>` concrete impl. Block uses bare
//!   `PropertyValue`; Page uses `DefaultPropertyEntry<PropertyValue>` for LWW merge.
//! - [`merge`]: `merge_properties<E: PropertyEntry>` pure function (added in T-B.5).
//! - [`resolver`]: `PropertyKeyResolver<P: PropertyRepository>` for case-insensitive
//!   key lookup (added in T-B.6).
//! - [`builtin`]: predefined system properties (status, priority, ...).
//! - [`definition`]: `PropertyDefinition` schema type.
//! - [`types`]: `PropertyType`, `Cardinality`, `ViewContext`, `ClosedValue`.
//! - [`validator`]: `PropertyValidator<P>` for type/cardinality/closed-set checks.

pub mod builtin;
pub mod definition;
pub mod entry;
pub mod types;
pub mod validator;

pub use builtin::{get_all_builtin_properties, get_builtin_property};
pub use definition::PropertyDefinition;
pub use entry::{
    DefaultPropertyEntry, HasTimestamp, HasValue, Mergeable, PropertyEntry,
};
pub use types::{Cardinality, ClosedValue, PropertyType, ViewContext};
pub use validator::PropertyValidator;
