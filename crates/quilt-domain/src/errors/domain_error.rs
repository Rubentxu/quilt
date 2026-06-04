//! DomainError - error types for the domain layer

use crate::value_objects::Uuid;
use std::fmt;

/// DomainError represents errors that occur in the domain layer.
///
/// These are pure domain errors, not infrastructure errors like
/// database connection failures.
#[derive(Debug)]
pub enum DomainError {
    // Entity errors
    /// Block was not found
    BlockNotFound(Uuid),
    /// Page was not found
    PageNotFound(Uuid),
    /// File was not found
    FileNotFound(Uuid),

    // Validation errors
    /// Invalid page name (contains special characters, etc.)
    InvalidPageName(String),
    /// Invalid journal day format
    InvalidJournalDay(String),
    /// Page is not a journal (when journal operation was expected)
    InvalidPageType(String),
    /// Invalid timezone string
    InvalidTimezone(String),
    /// Invalid configuration value
    InvalidConfiguration(String),
    /// Property failed type / cardinality / closed-set validation
    PropertyValidationError { property: String, error: String },
    /// Property is marked read-only and cannot be set by the caller
    PropertyReadOnly(String),

    // Operation errors
    /// Circular reference detected (moving block to own descendant)
    CircularReference(Uuid),
    /// Cannot delete block with children
    BlockHasChildren,
    /// Entity already exists
    AlreadyExists(String),
    /// Entity not found (generic)
    NotFound(String),

    // Data errors
    /// Invalid or corrupt data
    InvalidData(String),

    // Repository errors (storage, defined here for convenience)
    /// Storage error (data couldn't be stored or retrieved)
    Storage(String),
    /// Database error (e.g. failed to look up a property via the repository)
    Database(String),
    /// Feature not yet implemented
    NotImplemented(&'static str),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::BlockNotFound(id) => {
                write!(f, "Block not found: {}", id)
            }
            DomainError::PageNotFound(id) => {
                write!(f, "Page not found: {}", id)
            }
            DomainError::FileNotFound(id) => {
                write!(f, "File not found: {}", id)
            }
            DomainError::InvalidPageName(name) => {
                write!(f, "Invalid page name: {}", name)
            }
            DomainError::InvalidJournalDay(day) => {
                write!(f, "Invalid journal day: {}", day)
            }
            DomainError::InvalidPageType(msg) => {
                write!(f, "Invalid page type: {}", msg)
            }
            DomainError::InvalidTimezone(tz) => {
                write!(f, "Invalid timezone: {}", tz)
            }
            DomainError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {}", msg)
            }
            DomainError::PropertyValidationError { property, error } => {
                write!(f, "Property validation failed for '{}': {}", property, error)
            }
            DomainError::PropertyReadOnly(key) => {
                write!(f, "Property is read-only: {}", key)
            }
            DomainError::CircularReference(id) => {
                write!(f, "Circular reference detected for block: {}", id)
            }
            DomainError::BlockHasChildren => {
                write!(f, "Cannot delete block with children")
            }
            DomainError::AlreadyExists(entity) => {
                write!(f, "{} already exists", entity)
            }
            DomainError::NotFound(entity) => {
                write!(f, "Not found: {}", entity)
            }
            DomainError::InvalidData(msg) => {
                write!(f, "Invalid data: {}", msg)
            }
            DomainError::Storage(msg) => {
                write!(f, "Storage error: {}", msg)
            }
            DomainError::Database(msg) => {
                write!(f, "Database error: {}", msg)
            }
            DomainError::NotImplemented(feature) => {
                write!(f, "Not implemented: {}", feature)
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// Result type alias for domain operations
#[allow(dead_code)]
pub type DomainResult<T> = Result<T, DomainError>;
