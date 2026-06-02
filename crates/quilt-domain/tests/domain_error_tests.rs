//! Integration tests for DomainError.
//!
//! Covers: Display for every variant, Error trait impl,
//! and Debug output.

use quilt_domain::errors::DomainError;
use quilt_domain::value_objects::Uuid;

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
