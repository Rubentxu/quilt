//! Properties module - typed property system
//!
//! This module provides the typed property system for Quilt, including:
//! - PropertyType enum for type safety
//! - PropertyDefinition for schema definition
//! - Builtin properties (status, priority, deadline, scheduled, url)
//! - PropertyValidator for runtime validation

pub mod builtin;
pub mod definition;
pub mod types;
pub mod validator;

pub use builtin::{get_all_builtin_properties, get_builtin_property};
pub use definition::PropertyDefinition;
pub use types::{Cardinality, ClosedValue, PropertyType, ViewContext};
pub use validator::PropertyValidator;
