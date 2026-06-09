//! Integration tests for the `ApplyTemplateWithContractUseCase` (Q030).
//!
//! Covers:
//! - happy path: required properties supplied, locked untouched →
//!   block updated with the union of values, contract respected.
//! - locked property mutation in the proposed values is rejected.
//! - missing required property is rejected.
//! - version mismatch is rejected.
//! - the contract's `template_id` is matched against the schema
//!   returned by `TemplateUseCases::get_template_schema`.

use std::collections::HashMap;
use std::sync::Arc;

use quilt_application::templates::contract::{
    ApplyTemplateWithContractError, ApplyTemplateWithContractUseCase,
    ApplyTemplateWithContractUseCaseImpl,
};
use quilt_application::use_cases::{TemplateUseCases, TemplateUseCasesImpl};
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate, TemplateContract, Version};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};

// ── Helpers ────────────────────────────────────────────────────────

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

fn make_block(page_id: Uuid, content: &str) -> Block {
    Block::new(BlockCreate {
        page_id,
        content: content.to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: HashMap::new(),
    })
    .unwrap()
}

fn make_template_block(page_id: Uuid, props: Vec<(&str, &str)>) -> Block {
    let mut block = make_block(page_id, "Reference template");
    for (k, v) in props {
        block
            .properties
            .insert(k.to_string(), PropertyValue::String(v.to_string()));
    }
    block
}

async fn setup_template_world(
    template_name: &str,
    template_block_props: Vec<(&str, &str)>,
) -> (
    Arc<InMemoryPageRepo>,
    Arc<InMemoryBlockRepo>,
    Uuid, // template page id
    Uuid, // target page id
    Uuid, // target block id
) {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();

    // 1. Template page
    let template_full = format!("template/{}", template_name);
    let template_page = make_page(&template_full);
    let template_page_id = template_page.id;
    page_repo.insert(&template_page).await.unwrap();

    let template_block = make_template_block(template_page_id, template_block_props);
    block_repo.insert(&template_block).await.unwrap();

    // 2. Target page
    let target_page = make_page("My Notes");
    let target_page_id = target_page.id;
    page_repo.insert(&target_page).await.unwrap();

    // 3. Target block (initially with no properties)
    let target_block = make_block(target_page_id, "Some content");
    let target_block_id = target_block.id;
    block_repo.insert(&target_block).await.unwrap();

    (
        page_repo,
        block_repo,
        template_page_id,
        target_page_id,
        target_block_id,
    )
}

fn template_use_cases(
    page_repo: Arc<InMemoryPageRepo>,
    block_repo: Arc<InMemoryBlockRepo>,
) -> Arc<TemplateUseCasesImpl<InMemoryPageRepo, InMemoryBlockRepo>> {
    Arc::new(TemplateUseCasesImpl::new(
        Arc::clone(&page_repo),
        Arc::clone(&block_repo),
    ))
}

// ── Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn apply_with_contract_happy_path() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world("reference", vec![("title", "Untitled"), ("status", "todo")]).await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    // Build a contract that mirrors the template.
    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .inline_layout("title")
        .panel_layout("status")
        .build()
        .expect("contract should build");

    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "My Reference".to_string());
    proposed.insert("status".to_string(), "in-progress".to_string());

    let result = use_case
        .apply(target_block_id, "reference", &contract, &proposed, None)
        .await;

    assert!(result.is_ok(), "happy path should succeed: {result:?}");
    let res = result.unwrap();
    assert!(res.applied.contains(&"title".to_string()));
    assert!(res.applied.contains(&"status".to_string()));
    assert!(res.preserved.is_empty());
    assert!(res.rejected.is_empty());
    assert_eq!(res.contract_version.as_u32(), 1);

    // Verify the block was updated
    let block = block_repo
        .get_by_id(target_block_id)
        .await
        .unwrap()
        .expect("block should exist");
    assert_eq!(
        block.properties.get("title").unwrap().as_display_string(),
        "My Reference"
    );
    assert_eq!(
        block.properties.get("status").unwrap().as_display_string(),
        "in-progress"
    );
}

#[tokio::test]
async fn apply_with_contract_missing_required_returns_error() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world("reference", vec![("title", "Untitled"), ("status", "todo")]).await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .inline_layout("title")
        .panel_layout("status")
        .build()
        .unwrap();

    // Missing "status"!
    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "My Reference".to_string());

    let result = use_case
        .apply(target_block_id, "reference", &contract, &proposed, None)
        .await;

    assert!(result.is_err(), "missing required must error");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ApplyTemplateWithContractError::MissingRequiredProperty(_)
        ),
        "expected MissingRequiredProperty, got {err:?}"
    );

    // Block must NOT have been mutated.
    let block = block_repo
        .get_by_id(target_block_id)
        .await
        .unwrap()
        .unwrap();
    assert!(block.properties.get("title").is_none());
    assert!(block.properties.get("status").is_none());
}

#[tokio::test]
async fn apply_with_contract_locked_property_must_match_template() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world(
            "reference",
            vec![
                ("title", "Untitled"),
                ("status", "todo"),
                ("template", "reference"), // template value
            ],
        )
        .await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    // Contract declares "template" as locked with value "reference".
    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .required_property("template")
        .inline_layout("title")
        .panel_layout("status")
        .locked_layout("template")
        .build()
        .unwrap();

    // User tries to change "template" — must be rejected.
    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "x".to_string());
    proposed.insert("status".to_string(), "y".to_string());
    proposed.insert("template".to_string(), "HACKED-VALUE".to_string());

    let result = use_case
        .apply(target_block_id, "reference", &contract, &proposed, None)
        .await;

    assert!(result.is_err(), "locked property mutation must be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            ApplyTemplateWithContractError::LockedPropertyChanged { .. }
                | ApplyTemplateWithContractError::LockedPropertyAdded(_)
        ),
        "expected a locked-related error, got {err:?}"
    );
}

#[tokio::test]
async fn apply_with_contract_locked_property_kept_at_template_value_succeeds() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world(
            "reference",
            vec![
                ("title", "Untitled"),
                ("status", "todo"),
                ("template", "reference"),
            ],
        )
        .await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .required_property("template")
        .inline_layout("title")
        .panel_layout("status")
        .locked_layout("template")
        .build()
        .unwrap();

    // User keeps "template" at the template's value.
    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "x".to_string());
    proposed.insert("status".to_string(), "y".to_string());
    proposed.insert("template".to_string(), "reference".to_string());

    let result = use_case
        .apply(target_block_id, "reference", &contract, &proposed, None)
        .await;
    assert!(
        result.is_ok(),
        "keeping locked value at template's value should succeed: {result:?}"
    );
}

#[tokio::test]
async fn apply_with_contract_version_mismatch_returns_error() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world("reference", vec![("title", "Untitled"), ("status", "todo")]).await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .inline_layout("title")
        .panel_layout("status")
        .build()
        .unwrap();

    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "x".to_string());
    proposed.insert("status".to_string(), "y".to_string());

    // Caller says they have v2 but the contract is v1.
    let result = use_case
        .apply(
            target_block_id,
            "reference",
            &contract,
            &proposed,
            Some(Version::new().bump()),
        )
        .await;

    assert!(result.is_err(), "version mismatch must error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApplyTemplateWithContractError::VersionMismatch { .. }),
        "expected VersionMismatch, got {err:?}"
    );
}

#[tokio::test]
async fn apply_with_contract_unknown_template_returns_error() {
    let page_repo = InMemoryPageRepo::new();
    let block_repo = InMemoryBlockRepo::new();

    // No template page exists for "nonexistent".
    let target_page = make_page("Some Page");
    let target_page_id = target_page.id;
    page_repo.insert(&target_page).await.unwrap();
    let target_block = make_block(target_page_id, "x");
    let target_block_id = target_block.id;
    block_repo.insert(&target_block).await.unwrap();

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(Uuid::new_v4())
        .required_property("title")
        .inline_layout("title")
        .build()
        .unwrap();

    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "x".to_string());

    let result = use_case
        .apply(target_block_id, "nonexistent", &contract, &proposed, None)
        .await;
    assert!(result.is_err(), "unknown template must error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApplyTemplateWithContractError::TemplateNotFound(_)),
        "expected TemplateNotFound, got {err:?}"
    );
}

#[tokio::test]
async fn apply_with_contract_unknown_block_returns_error() {
    let (page_repo, block_repo, tpl_page_id, _tgt_page_id, _target_block_id) =
        setup_template_world("reference", vec![("title", "Untitled")]).await;

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(tpl_page_id)
        .required_property("title")
        .inline_layout("title")
        .build()
        .unwrap();

    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "x".to_string());

    let nonexistent_block = Uuid::new_v4();
    let result = use_case
        .apply(nonexistent_block, "reference", &contract, &proposed, None)
        .await;
    assert!(result.is_err(), "unknown block must error");
    let err = result.unwrap_err();
    assert!(
        matches!(err, ApplyTemplateWithContractError::BlockNotFound(_)),
        "expected BlockNotFound, got {err:?}"
    );
}

#[tokio::test]
async fn apply_with_contract_preserves_existing_user_properties() {
    let (page_repo, block_repo, _tpl_page_id, _tgt_page_id, target_block_id) =
        setup_template_world("reference", vec![("title", "Untitled"), ("status", "todo")]).await;

    // Pre-set a user property on the target block.
    let mut target_block = block_repo
        .get_by_id(target_block_id)
        .await
        .unwrap()
        .unwrap();
    target_block.properties.insert(
        "my-custom".to_string(),
        PropertyValue::String("user-value".to_string()),
    );
    block_repo.update(&target_block).await.unwrap();

    let tpl_uc = template_use_cases(page_repo.clone(), block_repo.clone());
    let use_case = ApplyTemplateWithContractUseCaseImpl::new(tpl_uc.clone(), block_repo.clone());

    let contract = TemplateContract::builder()
        .template_id(_tpl_page_id)
        .required_property("title")
        .required_property("status")
        .inline_layout("title")
        .panel_layout("status")
        .build()
        .unwrap();

    let mut proposed = HashMap::new();
    proposed.insert("title".to_string(), "New".to_string());
    proposed.insert("status".to_string(), "done".to_string());

    use_case
        .apply(target_block_id, "reference", &contract, &proposed, None)
        .await
        .unwrap();

    let block = block_repo
        .get_by_id(target_block_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        block
            .properties
            .get("my-custom")
            .unwrap()
            .as_display_string(),
        "user-value",
        "user's custom property must be preserved"
    );
    assert_eq!(
        block.properties.get("title").unwrap().as_display_string(),
        "New"
    );
}
