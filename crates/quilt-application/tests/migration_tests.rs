//! Integration tests for the migration module
//!
//! Tests the Markdown parser and MigrationEngine with fixtures and property-based testing.

use async_trait::async_trait;
use quilt_application::migration::{
    Frontmatter, MigrationEngine, infer_property_value, parse_md_import,
};
use quilt_domain::errors::DomainError;
use quilt_domain::properties::definition::PropertyDefinition;
use quilt_domain::properties::types::ClosedValue;
use quilt_domain::repositories::{PageRepository, PropertyRepository};
use quilt_domain::value_objects::{PropertyValue, Uuid};
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// In-memory property repository for testing
#[derive(Default)]
struct InMemoryPropertyRepo {
    properties: HashMap<String, PropertyDefinition>,
}

#[async_trait]
impl PropertyRepository for InMemoryPropertyRepo {
    async fn get_by_id(&self, _id: Uuid) -> Result<Option<PropertyDefinition>, DomainError> {
        Ok(None)
    }

    async fn get_by_db_ident(
        &self,
        ident: &str,
    ) -> Result<Option<PropertyDefinition>, DomainError> {
        Ok(self.properties.get(ident).cloned())
    }

    async fn get_all(&self) -> Result<Vec<PropertyDefinition>, DomainError> {
        Ok(self.properties.values().cloned().collect())
    }

    async fn insert(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
        Ok(())
    }

    async fn update(&self, _def: &PropertyDefinition) -> Result<(), DomainError> {
        Ok(())
    }

    async fn get_closed_values(&self, _property_id: Uuid) -> Result<Vec<ClosedValue>, DomainError> {
        Ok(Vec::new())
    }

    async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_by_db_idents(
        &self,
        _idents: &[&str],
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        Ok(Vec::new())
    }
    async fn search(
        &self,
        _query: &str,
        _limit: usize,
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        Ok(Vec::new())
    }
    async fn list_by_usage(&self, _limit: usize) -> Result<Vec<PropertyDefinition>, DomainError> {
        Ok(Vec::new())
    }
    async fn get_co_occurrences(
        &self,
        _limit: usize,
    ) -> Result<Vec<quilt_domain::properties::analytics::PropertyCoOccurrence>, DomainError> {
        Ok(vec![])
    }
    async fn get_trends(
        &self,
        _period_days: u32,
        _limit: usize,
    ) -> Result<Vec<quilt_domain::properties::analytics::PropertyTrend>, DomainError> {
        Ok(vec![])
    }
    async fn count_distinct_properties(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn count_blocks_with_properties(&self) -> Result<u64, DomainError> {
        Ok(0)
    }
}

/// Helper to get path to a test fixture
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from("tests/fixtures/md_import").join(name)
}

/// Helper to load a fixture file
fn load_fixture(name: &str) -> String {
    std::fs::read_to_string(fixture_path(name)).expect("Failed to read fixture")
}

// ═══════════════════════════════════════════════════════════════════════════
// Property Line Parsing Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_simple_property_line() {
    let input = "title: My Page";
    let result = parse_md_import(input).expect("Should parse");
    assert!(result.0.properties.is_empty()); // No frontmatter
}

#[test]
fn test_parse_frontmatter_property_with_colon_in_value() {
    let input = r#"---
key: value: with: colons
---
"#;
    let (fm, _) = parse_md_import(input).expect("Should parse");
    assert_eq!(fm.properties.len(), 1);
    assert_eq!(fm.properties[0].key, "key");
    assert_eq!(fm.properties[0].value, "value: with: colons");
}

#[test]
fn test_parse_frontmatter_empty_value() {
    let input = r#"---
empty_key:
another: with value
---
"#;
    let (fm, _) = parse_md_import(input).expect("Should parse");
    assert_eq!(fm.properties.len(), 2);
    assert_eq!(fm.properties[0].key, "empty_key");
    assert_eq!(fm.properties[0].value, "");
    assert_eq!(fm.properties[1].key, "another");
    assert_eq!(fm.properties[1].value, "with value");
}

#[test]
fn test_parse_frontmatter_boolean_values() {
    let input = r#"---
enabled: true
disabled: false
also_true: TRUE
also_false: FALSE
---
"#;
    let (fm, _) = parse_md_import(input).expect("Should parse");
    assert_eq!(fm.properties.len(), 4);
    // Values remain strings at this level; type inference happens later
    assert_eq!(fm.properties[0].value, "true");
    assert_eq!(fm.properties[1].value, "false");
}

// ═══════════════════════════════════════════════════════════════════════════
// Nested Block Indentation Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_single_level_indent() {
    let input = "\
- Top level
  - Nested
";
    let (_, blocks) = parse_md_import(input).expect("Should parse");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].indent_level, 0);
}

#[test]
fn test_parse_double_level_indent() {
    let input = "\
- Top level
  - Level 1
    - Level 2
";
    let (_, blocks) = parse_md_import(input).expect("Should parse");
    assert_eq!(blocks.len(), 1);
}

#[test]
fn test_parse_multiple_siblings_at_same_level() {
    let input = "\
- Block 1
- Block 2
- Block 3
";
    let (_, blocks) = parse_md_import(input).expect("Should parse");
    assert_eq!(blocks.len(), 3);
}

#[test]
fn test_parse_deeply_nested_blocks() {
    let input = "\
- Root
  - Level 1
    - Level 2
      - Level 3
        - Level 4
";
    let (_, blocks) = parse_md_import(input).expect("Should parse");
    // Parser should handle this gracefully
    assert!(!blocks.is_empty());
}

#[test]
fn test_parse_block_content_with_leading_spaces() {
    let input = "\
- Block with leading spaces in content
  - Nested block
";
    let (_, blocks) = parse_md_import(input).expect("Should parse");
    assert!(!blocks.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════════
// Type Inference Edge Case Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_infer_integer_from_string() {
    assert!(matches!(
        infer_property_value("0"),
        PropertyValue::Integer(0)
    ));
    assert!(matches!(
        infer_property_value("-1"),
        PropertyValue::Integer(-1)
    ));
    assert!(matches!(
        infer_property_value("999999"),
        PropertyValue::Integer(999999)
    ));
}

#[test]
fn test_infer_float_from_string() {
    assert!(matches!(infer_property_value("0.0"), PropertyValue::Float(f) if f == 0.0));
    assert!(
        matches!(infer_property_value("-3.14"), PropertyValue::Float(f) if (f + 3.14).abs() < 0.001)
    );
    assert!(matches!(
        infer_property_value("1e10"),
        PropertyValue::Float(_)
    ));
}

#[test]
fn test_infer_boolean_from_string() {
    assert!(matches!(
        infer_property_value("true"),
        PropertyValue::Boolean(true)
    ));
    assert!(matches!(
        infer_property_value("false"),
        PropertyValue::Boolean(false)
    ));
    assert!(matches!(
        infer_property_value("TRUE"),
        PropertyValue::Boolean(true)
    ));
    assert!(matches!(
        infer_property_value("FALSE"),
        PropertyValue::Boolean(false)
    ));
    assert!(matches!(
        infer_property_value("True"),
        PropertyValue::Boolean(true)
    ));
    assert!(matches!(
        infer_property_value("False"),
        PropertyValue::Boolean(false)
    ));
}

#[test]
fn test_infer_string_when_not_numeric_or_boolean() {
    assert!(matches!(infer_property_value("hello"), PropertyValue::String(s) if s == "hello"));
    assert!(matches!(infer_property_value(""), PropertyValue::String(s) if s.is_empty()));
    assert!(matches!(
        infer_property_value("trueish"),
        PropertyValue::String(_)
    ));
    assert!(matches!(
        infer_property_value("falsey"),
        PropertyValue::String(_)
    ));
}

#[test]
fn test_infer_date_like_strings_as_dates() {
    // Date-like strings ARE automatically parsed as dates (F21.2 fix)
    assert!(matches!(
        infer_property_value("2024-01-15"),
        PropertyValue::Date(_)
    ));
    assert!(matches!(
        infer_property_value("2024-12-31"),
        PropertyValue::Date(_)
    ));
    // DateTime strings are also parsed as dates
    assert!(matches!(
        infer_property_value("2024-01-15T10:30:00"),
        PropertyValue::Date(_)
    ));
    // Invalid dates fall back to string
    assert!(matches!(
        infer_property_value("not-a-date"),
        PropertyValue::String(_)
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// Fixture-Driven Integration Tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parse_simple_fixture() {
    let content = load_fixture("simple.md");
    let (fm, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 2);
    assert_eq!(fm.properties[0].key, "title");
    assert_eq!(fm.properties[0].value, "Simple Test Page");
    assert!(!blocks.is_empty());
}

#[test]
fn test_parse_with_properties_fixture() {
    let content = load_fixture("with_properties.md");
    let (fm, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 4); // title, created, author, priority
    assert_eq!(fm.properties[0].key, "title");
    assert_eq!(fm.properties[0].value, "Page With Properties");

    // Should have blocks
    assert!(!blocks.is_empty());
}

#[test]
fn test_parse_nested_blocks_fixture() {
    let content = load_fixture("nested_blocks.md");
    let (_, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert!(!blocks.is_empty());
    // Check that we have at least 2 top-level blocks
    assert!(blocks.len() >= 2);
}

#[test]
fn test_parse_numeric_properties_fixture() {
    let content = load_fixture("numeric_properties.md");
    let (fm, _) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 4); // title, count, price, percentage
    // The count property
    let count_prop = fm.properties.iter().find(|p| p.key == "count");
    assert!(count_prop.is_some());
    assert_eq!(count_prop.unwrap().value, "42");
}

#[test]
fn test_parse_boolean_properties_fixture() {
    let content = load_fixture("boolean_properties.md");
    let (fm, _) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 5); // title, enabled, disabled, active, inactive

    let enabled = fm.properties.iter().find(|p| p.key == "enabled");
    assert_eq!(enabled.unwrap().value, "true");

    let disabled = fm.properties.iter().find(|p| p.key == "disabled");
    assert_eq!(disabled.unwrap().value, "false");
}

#[test]
fn test_parse_complex_nested_fixture() {
    let content = load_fixture("complex_nested.md");
    let (_, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert!(!blocks.is_empty());
    // Root level blocks
    assert!(blocks.len() >= 2);
}

#[test]
fn test_parse_multiline_content_fixture() {
    let content = load_fixture("multiline_content.md");
    let (fm, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 2); // title, description
    assert_eq!(fm.properties[0].key, "title");
    assert_eq!(fm.properties[1].key, "description");
    assert!(!blocks.is_empty());
}

#[test]
fn test_parse_no_frontmatter_fixture() {
    let content = load_fixture("no_frontmatter.md");
    let (fm, blocks) = parse_md_import(&content).expect("Should parse fixture");

    assert!(fm.properties.is_empty());
    assert!(!blocks.is_empty());
    // 4 blocks: "No frontmatter...", "Block one", "Block two" (with child), "Block three"
    assert_eq!(blocks.len(), 4);
}

#[test]
fn test_parse_date_properties_fixture() {
    let content = load_fixture("date_properties.md");
    let (fm, _) = parse_md_import(&content).expect("Should parse fixture");

    assert_eq!(fm.properties.len(), 3);
    let created = fm.properties.iter().find(|p| p.key == "created");
    assert_eq!(created.unwrap().value, "2024-03-15");
}

// ═══════════════════════════════════════════════════════════════════════════
// MigrationEngine Integration Tests
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_migration_engine_import_simple_file() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    let content = load_fixture("simple.md");
    let result = engine.import_file(&content, "Test Page").await;

    assert!(result.is_ok());
    let import_result = result.unwrap();
    assert_eq!(import_result.pages_created, 1);
    assert!(import_result.blocks_created > 0);
}

#[tokio::test]
async fn test_migration_engine_import_no_duplicate_page() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    let content = load_fixture("simple.md");

    // First import should succeed
    let result1 = engine.import_file(&content, "Test Page").await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap().pages_created, 1);

    // Second import should be skipped with warning
    let result2 = engine.import_file(&content, "Test Page").await;
    assert!(result2.is_ok());
    let import_result2 = result2.unwrap();
    assert_eq!(import_result2.pages_created, 0);
    assert!(!import_result2.warnings.is_empty());
}

#[tokio::test]
async fn test_migration_engine_import_with_properties() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    let content = load_fixture("with_properties.md");
    let result = engine.import_file(&content, "Properties Test").await;

    assert!(result.is_ok());
    let import_result = result.unwrap();
    assert_eq!(import_result.pages_created, 1);
    assert!(import_result.blocks_created > 0);

    // Verify page was created
    let page = page_repo.get_by_name("Properties Test").await.unwrap();
    assert!(page.is_some());
}

#[tokio::test]
async fn test_migration_engine_import_nested_blocks() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    let content = load_fixture("complex_nested.md");
    let result = engine.import_file(&content, "Nested Test").await;

    assert!(result.is_ok());
    let import_result = result.unwrap();
    assert_eq!(import_result.pages_created, 1);
    // Should create many blocks due to nesting
    assert!(import_result.blocks_created >= 5);
}

#[tokio::test]
async fn test_migration_engine_import_directory_empty_dir() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    // Use the root directory which likely has no .md files
    let temp_dir = std::env::temp_dir();
    let result = engine.import_directory(&temp_dir).await;

    assert!(result.is_ok());
    // Empty directory or directory with no .md files returns empty results
    let results = result.unwrap();
    // Just verify it doesn't error - results may be empty or have warnings
    let total_blocks: usize = results.iter().map(|r| r.blocks_created).sum();
    assert_eq!(total_blocks, 0);
}

#[tokio::test]
async fn test_migration_engine_import_directory_with_files() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let property_repo = Arc::new(InMemoryPropertyRepo::default());
    let engine = MigrationEngine::new(page_repo.clone(), block_repo.clone(), property_repo);

    // Use the fixtures directory
    let fixtures_dir = PathBuf::from("tests/fixtures/md_import");
    let result = engine.import_directory(&fixtures_dir).await;

    assert!(result.is_ok());
    let results = result.unwrap();
    // Should have imported multiple files
    assert!(results.len() >= 5);

    // Check that total pages created is correct
    let total_pages: usize = results.iter().map(|r| r.pages_created).sum();
    assert!(total_pages >= 5);
}

// ═══════════════════════════════════════════════════════════════════════════
// Property-Based Roundtrip Tests (via proptest if available)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_parser_does_not_panic_on_various_inputs() {
    // These inputs should not panic
    let inputs = [
        "",
        "- single block",
        "- block with :: in content",
        "  - indented block",
        "- a\n- b\n- c",
        "-\n  - nested under empty",
        "many spaces    before",
        "tabs\there",
        r#"---
key: value
---
"#,
        r#"---
multiple: [values, here]
---
"#,
    ];

    for input in inputs {
        let result = std::panic::catch_unwind(|| parse_md_import(input));
        assert!(result.is_ok(), "Parser panicked on input: {:?}", input);
    }
}
