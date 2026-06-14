//! DomainError - error types for the domain layer

use crate::value_objects::Uuid;
use std::fmt;

/// DomainError represents errors that occur in the domain layer.
///
/// These are pure domain errors, not infrastructure errors like
/// database connection failures.
#[derive(Debug, PartialEq)]
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
    /// Parse error (failed to parse a string into a domain type)
    ParseError(String),

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
                write!(
                    f,
                    "Property validation failed for '{}': {}",
                    property, error
                )
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
            DomainError::ParseError(msg) => {
                write!(f, "Parse error: {}", msg)
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Display for each variant ──────────────────────────────────

    #[test]
    fn test_display_block_not_found() {
        let id = Uuid::new_v4();
        let err = DomainError::BlockNotFound(id);
        let msg = format!("{}", err);
        assert!(msg.contains("Block not found"));
        assert!(msg.contains(&id.to_string()));
    }

    #[test]
    fn test_display_page_not_found() {
        let id = Uuid::new_v4();
        let err = DomainError::PageNotFound(id);
        let msg = format!("{}", err);
        assert!(msg.contains("Page not found"));
        assert!(msg.contains(&id.to_string()));
    }

    #[test]
    fn test_display_file_not_found() {
        let id = Uuid::new_v4();
        let err = DomainError::FileNotFound(id);
        let msg = format!("{}", err);
        assert!(msg.contains("File not found"));
    }

    #[test]
    fn test_display_invalid_page_name() {
        let err = DomainError::InvalidPageName("bad//name".into());
        assert_eq!(format!("{}", err), "Invalid page name: bad//name");
    }

    #[test]
    fn test_display_invalid_journal_day() {
        let err = DomainError::InvalidJournalDay("2026-13-01".into());
        assert_eq!(format!("{}", err), "Invalid journal day: 2026-13-01");
    }

    #[test]
    fn test_display_invalid_page_type() {
        let err = DomainError::InvalidPageType("not a journal".into());
        assert_eq!(format!("{}", err), "Invalid page type: not a journal");
    }

    #[test]
    fn test_display_invalid_timezone() {
        let err = DomainError::InvalidTimezone("Mars/Zone".into());
        assert_eq!(format!("{}", err), "Invalid timezone: Mars/Zone");
    }

    #[test]
    fn test_display_invalid_configuration() {
        let err = DomainError::InvalidConfiguration("missing port".into());
        assert_eq!(format!("{}", err), "Invalid configuration: missing port");
    }

    #[test]
    fn test_display_circular_reference() {
        let id = Uuid::new_v4();
        let err = DomainError::CircularReference(id);
        let msg = format!("{}", err);
        assert!(msg.contains("Circular reference"));
    }

    #[test]
    fn test_display_block_has_children() {
        let err = DomainError::BlockHasChildren;
        assert_eq!(format!("{}", err), "Cannot delete block with children");
    }

    #[test]
    fn test_display_already_exists() {
        let err = DomainError::AlreadyExists("Page 'foo'".into());
        assert_eq!(format!("{}", err), "Page 'foo' already exists");
    }

    #[test]
    fn test_display_not_found() {
        let err = DomainError::NotFound("User settings".into());
        assert_eq!(format!("{}", err), "Not found: User settings");
    }

    #[test]
    fn test_display_invalid_data() {
        let err = DomainError::InvalidData("corrupt JSON".into());
        assert_eq!(format!("{}", err), "Invalid data: corrupt JSON");
    }

    #[test]
    fn test_display_storage() {
        let err = DomainError::Storage("disk full".into());
        assert_eq!(format!("{}", err), "Storage error: disk full");
    }

    #[test]
    fn test_display_not_implemented() {
        let err = DomainError::NotImplemented("export PDF");
        assert_eq!(format!("{}", err), "Not implemented: export PDF");
    }

    // ── std::error::Error trait ──────────────────────────────────

    #[test]
    fn test_implements_std_error() {
        let err = DomainError::NotFound("test".into());
        // This only compiles if DomainError implements std::error::Error
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_error_source_is_none() {
        let err = DomainError::InvalidData("test".into());
        assert!(std::error::Error::source(&err).is_none());
    }

    // ── Debug output ─────────────────────────────────────────────

    #[test]
    fn test_debug_contains_variant_name() {
        let err = DomainError::BlockHasChildren;
        let debug = format!("{:?}", err);
        assert!(debug.contains("BlockHasChildren"));
    }
}
