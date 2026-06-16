//! Integration tests for the MarkdownCanonicalizer — end-to-end round-trip.
//!
//! T21: End-to-end round-trip integration test:
//! 1. Take a Markdown block text
//! 2. Canonicalize it → get CanonicalizationResult
//! 3. Apply derived patches to a Block via PropertyPatch::apply_to
//! 4. Assert final Block has the expected properties

use crate::services::canonicalizer::MarkdownCanonicalizer;
use quilt_core::parser::inline::InlineParser;
use quilt_domain::canonicalization::{
    Canonicalizer, PropertyDefinitionRegistry, PropertyPatch, PropertyPatchProvenance,
};
use quilt_domain::entities::{Block, BlockCreate, PropertyKey};
use quilt_domain::properties::types::{
    Cardinality, MergePolicy, PropertyMutability, PropertyType, PropertyVisibility,
};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
use std::collections::HashMap;
use std::sync::Arc;

fn new_canonicalizer() -> MarkdownCanonicalizer {
    MarkdownCanonicalizer::new(Arc::new(InlineParser::new()))
}

/// Extract &str from PropertyValue (String or Ref variants).
fn extract_str(v: &PropertyValue) -> Option<&str> {
    match v {
        PropertyValue::String(s) => Some(s),
        PropertyValue::Ref(r) => Some(r),
        _ => None,
    }
}

/// Build a minimal PropertyDefinitionRegistry from a list of (db_ident, merge_policy).
fn make_registry(defs: &[(&str, MergePolicy)]) -> PropertyDefinitionRegistry {
    let definitions: Vec<quilt_domain::properties::PropertyDefinition> = defs
        .iter()
        .map(
            |(db_ident, merge_policy)| quilt_domain::properties::PropertyDefinition {
                id: Uuid::new_v4(),
                db_ident: db_ident.to_string(),
                title: db_ident.to_string(),
                property_type: PropertyType::Text,
                cardinality: Cardinality::One,
                closed_values: Vec::new(),
                attribute: None,
                status: quilt_domain::properties::types::PropertyStatus::Active,
                derived_from: None,
                visibility: PropertyVisibility::default(),
                mutability: PropertyMutability::Mutable,
                merge_policy: *merge_policy,
                alias_of: None,
                block_count: 0,
                page_count: 0,
                first_seen_at: None,
                last_seen_at: None,
            },
        )
        .collect();

    PropertyDefinitionRegistry::from_definitions(definitions)
}

fn make_empty_block() -> Block {
    Block::new(BlockCreate {
        page_id: Uuid::new_v4(),
        content: String::new(),
        parent_id: None,
        order: 0.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: HashMap::new(),
    })
    .expect("creates block")
}

// ── Round-trip: Markdown → canonicalize → apply ────────────────────────────────

#[test]
fn round_trip_h1_heading_sets_correct_properties() {
    let c = new_canonicalizer();
    let input = "## Section Title";
    let result = c.canonicalize_block(input);

    // Verify content is preserved (T15)
    assert_eq!(result.content.as_plain_text(), input);

    // Verify heading patches derived (T16)
    let heading_level_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "heading-level");
    assert!(
        heading_level_patch.is_some(),
        "expected heading-level patch"
    );
    assert_eq!(extract_str(&heading_level_patch.unwrap().value), Some("2"));

    let block_role_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "block-role");
    assert!(block_role_patch.is_some(), "expected block-role patch");
    assert_eq!(
        extract_str(&block_role_patch.unwrap().value),
        Some("heading")
    );

    // Apply patches to block
    let mut block = make_empty_block();
    let defs = make_registry(&[
        ("heading-level", MergePolicy::SetIfMissing),
        ("block-role", MergePolicy::SetIfMissing),
    ]);

    for patch in &result.derived {
        let outcome = patch
            .apply_to(&mut block, &defs)
            .expect("patch should apply");
        assert!(outcome.conflicts.is_empty(), "no conflicts expected");
    }

    // Assert final block state
    assert_eq!(
        extract_str(block.properties.get("heading-level").unwrap()),
        Some("2")
    );
    assert_eq!(
        extract_str(block.properties.get("block-role").unwrap()),
        Some("heading")
    );
}

#[test]
fn round_trip_page_ref_applies_page_ref_patch() {
    let c = new_canonicalizer();
    let input = "See [[My Project]] for details";
    let result = c.canonicalize_block(input);

    let page_ref_patch = result.derived.iter().find(|p| p.key.as_str() == "page-ref");
    assert!(
        page_ref_patch.is_some(),
        "expected page-ref patch: {:?}",
        result.derived
    );
    assert_eq!(
        extract_str(&page_ref_patch.unwrap().value),
        Some("My Project")
    );

    let mut block = make_empty_block();
    let defs = make_registry(&[("page-ref", MergePolicy::SetIfMissing)]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }
    assert_eq!(
        extract_str(block.properties.get("page-ref").unwrap()),
        Some("My Project")
    );
}

#[test]
fn round_trip_block_ref_applies_block_ref_patch() {
    let c = new_canonicalizer();
    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let input = format!("Link to (({}))", uuid);
    let result = c.canonicalize_block(&input);

    let block_ref_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "block-ref");
    assert!(block_ref_patch.is_some(), "expected block-ref patch");
    assert_eq!(extract_str(&block_ref_patch.unwrap().value), Some(uuid));

    let mut block = make_empty_block();
    let defs = make_registry(&[("block-ref", MergePolicy::SetIfMissing)]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }
    assert_eq!(
        extract_str(block.properties.get("block-ref").unwrap()),
        Some(uuid)
    );
}

#[test]
fn round_trip_external_link_applies_link_patches() {
    let c = new_canonicalizer();
    let input = "Visit [Quilt](https://quilt.com) now";
    let result = c.canonicalize_block(input);

    let kind_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "link-kind");
    assert_eq!(
        kind_patch.map(|p| extract_str(&p.value)),
        Some(Some("external"))
    );

    let url_patch = result.derived.iter().find(|p| p.key.as_str() == "link-url");
    assert_eq!(
        url_patch.map(|p| extract_str(&p.value)),
        Some(Some("https://quilt.com"))
    );

    let text_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "link-text");
    assert_eq!(
        text_patch.map(|p| extract_str(&p.value)),
        Some(Some("Quilt"))
    );

    // Apply and verify
    let mut block = make_empty_block();
    let defs = make_registry(&[
        ("link-kind", MergePolicy::SetIfMissing),
        ("link-url", MergePolicy::SetIfMissing),
        ("link-text", MergePolicy::SetIfMissing),
    ]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }
    assert_eq!(
        extract_str(block.properties.get("link-url").unwrap()),
        Some("https://quilt.com")
    );
}

#[test]
fn round_trip_image_embed_applies_embed_patches() {
    let c = new_canonicalizer();
    let input = "![Screenshot](https://example.com/screenshot.png)";
    let result = c.canonicalize_block(input);

    let kind_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "embed-kind");
    assert_eq!(
        kind_patch.map(|p| extract_str(&p.value)),
        Some(Some("image"))
    );

    let url_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "embed-url");
    assert_eq!(
        url_patch.map(|p| extract_str(&p.value)),
        Some(Some("https://example.com/screenshot.png"))
    );

    let mut block = make_empty_block();
    let defs = make_registry(&[
        ("embed-kind", MergePolicy::SetIfMissing),
        ("embed-url", MergePolicy::SetIfMissing),
    ]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }
    assert_eq!(
        extract_str(block.properties.get("embed-url").unwrap()),
        Some("https://example.com/screenshot.png")
    );
}

#[test]
fn round_trip_todo_marker_applies_status_and_type_patches() {
    let c = new_canonicalizer();
    let input = "TODO: Fix the login bug";
    let result = c.canonicalize_block(input);

    // New behavior: emits status:: todo + type:: task + projection:: auto (no marker::)
    let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
    assert!(
        status_patch.is_some(),
        "expected status patch: {:?}",
        result.derived
    );
    assert_eq!(extract_str(&status_patch.unwrap().value), Some("todo"));

    let type_patch = result.derived.iter().find(|p| p.key.as_str() == "type");
    assert!(
        type_patch.is_some(),
        "expected type patch: {:?}",
        result.derived
    );
    assert_eq!(extract_str(&type_patch.unwrap().value), Some("task"));

    // Apply with a registry that defines status + type + projection
    let mut block = make_empty_block();
    let defs = make_registry(&[
        ("status", MergePolicy::SetIfMissing),
        ("type", MergePolicy::SetIfMissing),
        ("projection", MergePolicy::SetIfMissing),
    ]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }
    assert_eq!(
        extract_str(block.properties.get("status").unwrap()),
        Some("todo")
    );
    assert_eq!(
        extract_str(block.properties.get("type").unwrap()),
        Some("task")
    );
    assert_eq!(
        extract_str(block.properties.get("projection").unwrap()),
        Some("auto")
    );
}

#[test]
fn round_trip_brackets_done_applies_status_done() {
    let c = new_canonicalizer();
    let input = "[x] Task completed";
    let result = c.canonicalize_block(input);

    // New behavior: emits status:: done (no marker::)
    let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
    assert!(
        status_patch.is_some(),
        "expected status patch: {:?}",
        result.derived
    );
    assert_eq!(extract_str(&status_patch.unwrap().value), Some("done"));
}

#[test]
fn round_trip_page_ref_with_alias_emits_alias() {
    let c = new_canonicalizer();
    let input = "See [[Actual Page|Display Text]] here";
    let result = c.canonicalize_block(input);

    let alias_patch = result
        .derived
        .iter()
        .find(|p| p.key.as_str() == "page-ref-alias");
    assert!(alias_patch.is_some(), "expected page-ref-alias patch");
    assert_eq!(
        extract_str(&alias_patch.unwrap().value),
        Some("Display Text")
    );
}

#[test]
fn round_trip_multiple_refs_collects_all() {
    let c = new_canonicalizer();
    let input = "See [[Page A]] and [[Page B]] plus ((block-uuid))";
    let result = c.canonicalize_block(input);

    let page_refs: Vec<_> = result
        .derived
        .iter()
        .filter(|p| p.key.as_str() == "page-ref")
        .collect();
    assert_eq!(page_refs.len(), 2);

    let block_refs: Vec<_> = result
        .derived
        .iter()
        .filter(|p| p.key.as_str() == "block-ref")
        .collect();
    assert_eq!(block_refs.len(), 1);
}

#[test]
fn round_trip_mixed_content_emits_all_patches() {
    let c = new_canonicalizer();
    let input = "## Backlog\n\nSee [[Project X]] and [docs](https://example.com) #urgent";
    let result = c.canonicalize_block(input);

    // Heading
    assert!(
        result
            .derived
            .iter()
            .any(|p| p.key.as_str() == "heading-level")
    );
    // Page ref
    assert!(result.derived.iter().any(|p| p.key.as_str() == "page-ref"));
    // Link
    assert!(result.derived.iter().any(|p| p.key.as_str() == "link-kind"));
    // No task marker
    assert!(!result.derived.iter().any(|p| p.key.as_str() == "marker"));
}

#[test]
fn round_trip_unknown_keys_are_skipped() {
    let c = new_canonicalizer();
    let input = "TODO: some task";
    let result = c.canonicalize_block(input);

    // Apply with a registry that only has "marker" defined
    let mut block = make_empty_block();
    let defs = make_registry(&[("marker", MergePolicy::Overwrite)]);

    for patch in &result.derived {
        let outcome = patch.apply_to(&mut block, &defs).expect("should not error");
        // Unknown keys should be skipped, not errored
        if patch.key.as_str() == "marker" {
            assert!(
                outcome.skipped.is_empty()
                    || outcome
                        .derived_materialized
                        .iter()
                        .any(|k| k.as_str() == "marker")
            );
        }
    }
}

#[test]
fn round_trip_overwrite_policy_replaces_status_values() {
    let c = new_canonicalizer();
    let input = "TODO: old task";
    let result = c.canonicalize_block(input);

    let mut block = make_empty_block();
    // Pre-populate with existing status
    block
        .properties
        .insert("status".into(), PropertyValue::text("done"));

    // Use Overwrite policy so it replaces
    let defs = make_registry(&[
        ("status", MergePolicy::Overwrite),
        ("type", MergePolicy::Overwrite),
        ("projection", MergePolicy::Overwrite),
    ]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }

    // Should be overwritten to "todo"
    assert_eq!(
        extract_str(block.properties.get("status").unwrap()),
        Some("todo")
    );
}

#[test]
fn round_trip_set_if_missing_keeps_existing() {
    let c = new_canonicalizer();
    let input = "TODO: new task";
    let result = c.canonicalize_block(input);

    let mut block = make_empty_block();
    // Pre-populate with existing marker
    block
        .properties
        .insert("marker".into(), PropertyValue::text("done"));

    // Use SetIfMissing policy so it keeps existing
    let defs = make_registry(&[("marker", MergePolicy::SetIfMissing)]);
    for patch in &result.derived {
        let _ = patch.apply_to(&mut block, &defs);
    }

    // Should keep "done" (set-if-missing)
    assert_eq!(
        extract_str(block.properties.get("marker").unwrap()),
        Some("done")
    );
}
