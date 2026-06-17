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
    /// Invalid source path (e.g., absolute path instead of relative)
    InvalidPageSourcePath(String),
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

    // Canonicalization / patch errors
    /// Merge conflict: patch value differs from existing value under RejectOnConflict policy
    MergeConflict {
        /// The property key that conflicted
        key: String,
        /// The value already present in the block
        existing: crate::value_objects::PropertyValue,
        /// The value the patch attempted to write
        attempted: crate::value_objects::PropertyValue,
    },
    /// Attempted to explicitly patch a property whose definition is immutable
    ImmutableProperty(String),
    /// Patch attempted to write a forbidden key (`content`, `text`, `children`)
    ForbiddenPatchKey(String),
    /// Unknown preset: no preset with this id exists in the registry
    UnknownPreset(crate::canonicalization::PresetId),
    /// A required preset argument was not provided
    MissingPresetArg {
        /// The preset that requires the argument
        preset: crate::canonicalization::PresetId,
        /// The kind of missing argument
        kind: crate::canonicalization::PresetArgKind,
    },
    /// Invalid preset id format
    InvalidPresetId(String),
    /// Duplicate preset argument kinds in a single preset invocation
    DuplicatePresetArgKind(crate::canonicalization::PresetArgKind),
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
    /// Invariant violation in domain logic
    InvariantViolation(&'static str),
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
            DomainError::InvalidPageSourcePath(msg) => {
                write!(f, "Invalid source path: {}", msg)
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
            DomainError::MergeConflict { key, .. } => {
                write!(f, "Merge conflict on property '{}'", key)
            }
            DomainError::ImmutableProperty(key) => {
                write!(f, "Property is immutable: {}", key)
            }
            DomainError::ForbiddenPatchKey(key) => {
                write!(f, "Forbidden patch key: {}", key)
            }
            DomainError::UnknownPreset(preset) => {
                write!(f, "unknown preset: {}", preset)
            }
            DomainError::MissingPresetArg { preset, kind } => {
                let kind_str = match kind {
                    crate::canonicalization::PresetArgKind::Date => "Date",
                    crate::canonicalization::PresetArgKind::Url => "Url",
                    crate::canonicalization::PresetArgKind::Text => "Text",
                };
                write!(f, "preset {} requires a {} argument", preset, kind_str)
            }
            DomainError::InvalidPresetId(s) => {
                write!(f, "invalid preset id: {}", s)
            }
            DomainError::DuplicatePresetArgKind(kind) => {
                let kind_str = match kind {
                    crate::canonicalization::PresetArgKind::Date => "Date",
                    crate::canonicalization::PresetArgKind::Url => "Url",
                    crate::canonicalization::PresetArgKind::Text => "Text",
                };
                write!(f, "duplicate preset arg kind: {}", kind_str)
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
            DomainError::InvariantViolation(msg) => {
                write!(f, "Invariant violation: {}", msg)
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
    use crate::canonicalization::{PresetArgKind, PresetId};

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

    // ── InvariantViolation ────────────────────────────────────────

    #[test]
    fn test_display_invariant_violation() {
        let err = DomainError::InvariantViolation("derived property must be immutable");
        let msg = format!("{}", err);
        assert!(msg.contains("Invariant violation: derived property must be immutable"));
    }

    #[test]
    fn test_implements_std_error_invariant_violation() {
        // InvariantViolation must still implement std::error::Error (additive variant)
        let err = DomainError::InvariantViolation("test");
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_debug_contains_invariant_violation_name() {
        let err = DomainError::InvariantViolation("test");
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvariantViolation"));
    }

    // ── MergeConflict ──────────────────────────────────────────────

    #[test]
    fn test_display_merge_conflict() {
        use crate::value_objects::PropertyValue;
        let err = DomainError::MergeConflict {
            key: "status".into(),
            existing: PropertyValue::string("done"),
            attempted: PropertyValue::string("todo"),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Merge conflict"));
        assert!(msg.contains("status"));
    }

    #[test]
    fn test_merge_conflict_equality() {
        use crate::value_objects::PropertyValue;
        let err1 = DomainError::MergeConflict {
            key: "status".into(),
            existing: PropertyValue::string("done"),
            attempted: PropertyValue::string("todo"),
        };
        let err2 = DomainError::MergeConflict {
            key: "status".into(),
            existing: PropertyValue::string("done"),
            attempted: PropertyValue::string("todo"),
        };
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_debug_contains_merge_conflict_name() {
        use crate::value_objects::PropertyValue;
        let err = DomainError::MergeConflict {
            key: "status".into(),
            existing: PropertyValue::string("done"),
            attempted: PropertyValue::string("todo"),
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("MergeConflict"));
    }

    // ── ImmutableProperty ──────────────────────────────────────────

    #[test]
    fn test_display_immutable_property() {
        let err = DomainError::ImmutableProperty("heading-level".into());
        let msg = format!("{}", err);
        assert_eq!(msg, "Property is immutable: heading-level");
    }

    #[test]
    fn test_debug_contains_immutable_property_name() {
        let err = DomainError::ImmutableProperty("heading-level".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ImmutableProperty"));
    }

    // ── ForbiddenPatchKey ─────────────────────────────────────────

    #[test]
    fn test_display_forbidden_patch_key() {
        let err = DomainError::ForbiddenPatchKey("content".into());
        let msg = format!("{}", err);
        assert_eq!(msg, "Forbidden patch key: content");
    }

    #[test]
    fn test_debug_contains_forbidden_patch_key_name() {
        let err = DomainError::ForbiddenPatchKey("content".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("ForbiddenPatchKey"));
    }

    // ── UnknownPreset ─────────────────────────────────────────────

    #[test]
    fn test_display_unknown_preset() {
        use crate::canonicalization::PresetId;
        let id = PresetId::new("/TODO").unwrap();
        let err = DomainError::UnknownPreset(id);
        let msg = format!("{}", err);
        assert_eq!(msg, "unknown preset: /TODO");
    }

    #[test]
    fn test_debug_contains_unknown_preset() {
        use crate::canonicalization::PresetId;
        let id = PresetId::new("/Video").unwrap();
        let err = DomainError::UnknownPreset(id);
        let debug = format!("{:?}", err);
        assert!(debug.contains("UnknownPreset"));
    }

    // ── MissingPresetArg ──────────────────────────────────────────

    #[test]
    fn test_display_missing_preset_arg() {
        use crate::canonicalization::{PresetArgKind, PresetId};
        let preset = PresetId::new("/Scheduled").unwrap();
        let err = DomainError::MissingPresetArg {
            preset,
            kind: PresetArgKind::Date,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("/Scheduled"));
        assert!(msg.contains("Date"));
    }

    #[test]
    fn test_debug_contains_missing_preset_arg() {
        use crate::canonicalization::{PresetArgKind, PresetId};
        let preset = PresetId::new("/Deadline").unwrap();
        let err = DomainError::MissingPresetArg {
            preset,
            kind: PresetArgKind::Url,
        };
        let debug = format!("{:?}", err);
        assert!(debug.contains("MissingPresetArg"));
    }

    // ── InvalidPresetId ──────────────────────────────────────────

    #[test]
    fn test_display_invalid_preset_id() {
        let err = DomainError::InvalidPresetId("TODO".into());
        let msg = format!("{}", err);
        assert_eq!(msg, "invalid preset id: TODO");
    }

    #[test]
    fn test_debug_contains_invalid_preset_id() {
        let err = DomainError::InvalidPresetId("TO DO".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidPresetId"));
    }

    // ── DuplicatePresetArgKind ────────────────────────────────────

    #[test]
    fn test_display_duplicate_preset_arg_kind() {
        let err = DomainError::DuplicatePresetArgKind(PresetArgKind::Text);
        let msg = format!("{}", err);
        assert_eq!(msg, "duplicate preset arg kind: Text");
    }

    #[test]
    fn test_debug_contains_duplicate_preset_arg_kind() {
        let err = DomainError::DuplicatePresetArgKind(PresetArgKind::Date);
        let debug = format!("{:?}", err);
        assert!(debug.contains("DuplicatePresetArgKind"));
    }
}
