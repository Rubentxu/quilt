//! Classes module - entity class system with inheritance
//!
//! This module provides the class system for Quilt, including:
//! - Class entity with inheritance support
//! - builtin_classes (Root, Tag, Page, Journal, Task, Query, Property)
//! - ClassValidator for runtime validation

pub mod types;
pub mod validator;

pub use types::{builtin_classes, Class};
pub use validator::ClassValidator;
