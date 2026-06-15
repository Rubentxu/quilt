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

pub mod analytics;
pub mod builtin;
pub mod definition;
pub mod entry;
pub mod merge;
pub mod relation;
pub mod resolver;
pub mod schema;
pub mod types;
pub mod validator;

pub use analytics::{
    AnalyticsParams, PropertyAnalytics, PropertyCoOccurrence, PropertyTrend, TrendDirection,
};
pub use builtin::{get_all_builtin_properties, get_builtin_property};
pub use definition::PropertyDefinition;
pub use entry::{DefaultPropertyEntry, HasTimestamp, HasValue, Mergeable, PropertyEntry};
pub use merge::merge_properties;
pub use relation::{PropertyRelation, RelationType};
pub use resolver::PropertyKeyResolver;
pub use schema::{AutoDetectParams, PropertySchema};
pub use types::{
    Cardinality, ClosedValue, DerivedSource, MergePolicy, PropertyMutability, PropertyStatus,
    PropertyType, PropertyVisibility, ViewContext,
};
pub use validator::PropertyValidator;
