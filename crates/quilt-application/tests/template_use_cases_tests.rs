//! Integration tests for TemplateUseCases (ADR-0007).
//!
//! Covers: list_templates (filters by template/ prefix, sorts by name,
//! reads card-shape/icon/cssclass from the first metadata block),
//! get_template_schema (returns full block tree + properties or None
//! for unknown templates), end-to-end with the in-memory repos.

use std::collections::HashMap;

use quilt_application::use_cases::{TemplateUseCases, TemplateUseCasesImpl};
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue};
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};

// ── Helpers ──────────────────────────────────────────────────

fn setup() -> TemplateUseCasesImpl<InMemoryPageRepo, InMemoryBlockRepo> {
    TemplateUseCasesImpl::new(InMemoryPageRepo::new(), InMemoryBlockRepo::new())
}

fn make_page(name: &str) -> Page {
    Page::new(PageCreate {
        name: name.to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: std::collections::HashMap::new(),
    })
    .unwrap()
}

fn make_block_with_properties(
    page_id: quilt_domain::value_objects::Uuid,
    content: &str,
    properties: Vec<(&str, PropertyValue)>,
) -> Block {
    let mut props = HashMap::new();
    for (k, v) in properties {
        props.insert(k.to_string(), v);
    }
    Block::new(BlockCreate {
        page_id,
        content: content.to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: props,
    })
    .unwrap()
}

async fn insert_page(repo: &InMemoryPageRepo, page: &Page) {
    repo.insert(page).await.unwrap();
}

async fn insert_block(repo: &InMemoryBlockRepo, block: &Block) {
    repo.insert(block).await.unwrap();
}

// ── list_templates ──────────────────────────────────────────

#[tokio::test]
async fn test_list_templates_empty() {
    let use_cases = setup();
    let templates = use_cases.list_templates().await.unwrap();
    assert!(templates.is_empty());
}

#[tokio::test]
async fn test_list_templates_filters_by_template_prefix() {
    let page_repo = InMemoryPageRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), InMemoryBlockRepo::new());

    // Mix of regular pages and template pages
    insert_page(&page_repo, &make_page("regular-page")).await;
    insert_page(&page_repo, &make_page("journal-2026-06-01")).await;
    insert_page(&page_repo, &make_page("template/reference")).await;
    insert_page(&page_repo, &make_page("template/documentation")).await;
    insert_page(&page_repo, &make_page("template/nested/meeting")).await;
    insert_page(&page_repo, &make_page("templated")).await; // not a template
    insert_page(&page_repo, &make_page("template")).await; // bare "template"

    let templates = use_cases.list_templates().await.unwrap();
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();

    assert_eq!(templates.len(), 4);
    assert!(names.contains(&"reference"));
    assert!(names.contains(&"documentation"));
    assert!(names.contains(&"nested/meeting"));
    assert!(names.contains(&"template")); // bare "template" is also valid
    assert!(!names.contains(&"regular-page"));
    assert!(!names.contains(&"templated"));
    assert!(!names.contains(&"journal-2026-06-01"));
}

#[tokio::test]
async fn test_list_templates_sorted_by_name() {
    let page_repo = InMemoryPageRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), InMemoryBlockRepo::new());

    // Insert in non-alphabetical order. Page names are stored
    // lowercase by the domain layer, so we expect the same.
    insert_page(&page_repo, &make_page("template/zeta")).await;
    insert_page(&page_repo, &make_page("template/alpha")).await;
    insert_page(&page_repo, &make_page("template/beta")).await;

    let templates = use_cases.list_templates().await.unwrap();
    let names: Vec<&str> = templates.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta", "zeta"]);
}

#[tokio::test]
async fn test_list_templates_reads_card_metadata_from_first_block() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone());

    // Create the template page
    let page = make_page("template/meeting-notes");
    insert_page(&page_repo, &page).await;

    // Add a block with the metadata
    let metadata_block = make_block_with_properties(
        page.id,
        "",
        vec![
            ("card-shape", PropertyValue::String("reference".to_string())),
            ("icon", PropertyValue::String("📋".to_string())),
            (
                "cssclass",
                PropertyValue::String("card-meeting".to_string()),
            ),
        ],
    );
    insert_block(&block_repo, &metadata_block).await;

    let templates = use_cases.list_templates().await.unwrap();
    assert_eq!(templates.len(), 1);
    let t = &templates[0];
    assert_eq!(t.name, "meeting-notes");
    assert_eq!(t.full_name, "template/meeting-notes");
    assert_eq!(t.card_shape, "reference");
    assert_eq!(t.icon.as_deref(), Some("📋"));
    assert_eq!(t.cssclass.as_deref(), Some("card-meeting"));
    assert_eq!(t.block_count, 1);
    assert_eq!(t.metadata_block_ids, vec![metadata_block.id]);
}

#[tokio::test]
async fn test_list_templates_defaults_to_inline_when_no_card_shape() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone());

    let page = make_page("template/missing-shape");
    insert_page(&page_repo, &page).await;

    // Block with no card-shape property
    let block = make_block_with_properties(
        page.id,
        "just a regular block",
        vec![("cssclass", PropertyValue::String("card-plain".to_string()))],
    );
    insert_block(&block_repo, &block).await;

    let templates = use_cases.list_templates().await.unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].card_shape, "inline");
    assert_eq!(templates[0].cssclass.as_deref(), Some("card-plain"));
}

// ── get_template_schema ─────────────────────────────────────

#[tokio::test]
async fn test_get_template_schema_returns_none_for_unknown_template() {
    let use_cases = setup();
    let schema = use_cases.get_template_schema("nonexistent").await.unwrap();
    assert!(schema.is_none());
}

#[tokio::test]
async fn test_get_template_schema_returns_full_metadata() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone());

    let page = make_page("template/contact");
    insert_page(&page_repo, &page).await;

    // Two blocks: one with metadata, one with a "user-facing" property
    let meta = make_block_with_properties(
        page.id,
        "",
        vec![
            ("card-shape", PropertyValue::String("reference".to_string())),
            ("icon", PropertyValue::String("👤".to_string())),
        ],
    );
    let example = make_block_with_properties(
        page.id,
        "Alice Doe",
        vec![
            ("name", PropertyValue::String("Alice Doe".to_string())),
            ("priority", PropertyValue::Integer(1)),
        ],
    );
    insert_block(&block_repo, &meta).await;
    insert_block(&block_repo, &example).await;

    let schema = use_cases
        .get_template_schema("contact")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(schema.name, "contact");
    assert_eq!(schema.full_name, "template/contact");
    assert_eq!(schema.card_shape, "reference");
    assert_eq!(schema.icon.as_deref(), Some("👤"));
    assert_eq!(schema.blocks.len(), 2);

    // Properties should include the user-facing ones but not
    // the card-shape/icon (those are surfaced as top-level fields)
    let keys: Vec<&str> = schema.properties.iter().map(|p| p.key.as_str()).collect();
    assert!(keys.contains(&"name"));
    assert!(keys.contains(&"priority"));
    assert!(!keys.contains(&"card-shape"));
    assert!(!keys.contains(&"icon"));

    // Type hints preserved
    let priority_prop = schema
        .properties
        .iter()
        .find(|p| p.key == "priority")
        .unwrap();
    assert_eq!(priority_prop.r#type, "integer");
    assert_eq!(priority_prop.value, "1");
}

#[tokio::test]
async fn test_get_template_schema_reserves_block_level_keys() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();
    let use_cases = TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone());

    let page = make_page("template/with-reserved");
    insert_page(&page_repo, &page).await;
    let block = make_block_with_properties(
        page.id,
        "content",
        vec![
            (
                "template",
                PropertyValue::String("other-template".to_string()),
            ),
            ("type", PropertyValue::String("reference".to_string())),
            ("collapsed", PropertyValue::Boolean(true)),
            ("author", PropertyValue::String("claude".to_string())),
        ],
    );
    insert_block(&block_repo, &block).await;

    let schema = use_cases
        .get_template_schema("with-reserved")
        .await
        .unwrap()
        .unwrap();
    let keys: Vec<&str> = schema.properties.iter().map(|p| p.key.as_str()).collect();
    // template, type, collapsed are reserved block-level keys —
    // not part of the template contract
    assert!(!keys.contains(&"template"));
    assert!(!keys.contains(&"type"));
    assert!(!keys.contains(&"collapsed"));
    // author is surfaced
    assert!(keys.contains(&"author"));
}
