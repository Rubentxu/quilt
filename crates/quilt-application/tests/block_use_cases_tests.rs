//! Integration tests for BlockUseCases — exercises the use case layer
//! with in-memory repository implementations.
//!
//! Covers: create_with_page, delete, link, get_tree, get_backlinks,
//! list_by_property, create_task, and page auto-creation behavior.

use std::collections::HashMap;

use quilt_application::use_cases::{BlockUseCases, BlockUseCasesImpl};
use quilt_domain::value_objects::{PropertyValue, TaskMarker};
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};

// ── Helpers ──────────────────────────────────────────────────

fn setup() -> BlockUseCasesImpl<InMemoryBlockRepo, InMemoryPageRepo> {
    let block_repo = InMemoryBlockRepo::new();
    let page_repo = InMemoryPageRepo::new();
    BlockUseCasesImpl::new(block_repo, page_repo)
}

// ── create_with_page ────────────────────────────────────────

#[tokio::test]
async fn test_create_with_page_creates_page_if_missing() {
    let use_cases = setup();

    let block = use_cases
        .create_with_page("new-page", "Hello", None, None, HashMap::new())
        .await
        .unwrap();

    assert_eq!(block.content, "Hello");
    assert!(!block.id.to_string().is_empty());
    assert!(!block.page_id.to_string().is_empty());
}

#[tokio::test]
async fn test_create_with_page_with_marker() {
    let use_cases = setup();

    let block = use_cases
        .create_with_page(
            "tasks",
            "Buy milk",
            None,
            Some(TaskMarker::Todo),
            HashMap::new(),
        )
        .await
        .unwrap();

    assert_eq!(block.marker, Some(TaskMarker::Todo));
}

#[tokio::test]
async fn test_create_with_page_with_parent() {
    let use_cases = setup();

    // First create a parent block
    let parent = use_cases
        .create_with_page("parent-page", "Parent", None, None, HashMap::new())
        .await
        .unwrap();

    // Create child block with parent_id
    let child = use_cases
        .create_with_page(
            "parent-page",
            "Child",
            Some(parent.id),
            None,
            HashMap::new(),
        )
        .await
        .unwrap();

    assert_eq!(child.parent_id, Some(parent.id));
}

#[tokio::test]
async fn test_create_with_page_with_properties() {
    let use_cases = setup();
    let mut props = HashMap::new();
    props.insert(
        "created_by".to_string(),
        PropertyValue::String("agent::test".into()),
    );

    let block = use_cases
        .create_with_page("meta", "Content", None, None, props)
        .await
        .unwrap();

    assert_eq!(
        block.properties.get("created_by"),
        Some(&PropertyValue::String("agent::test".into()))
    );
}

// ── delete ──────────────────────────────────────────────────

#[tokio::test]
async fn test_delete_existing_block() {
    let use_cases = setup();

    let block = use_cases
        .create_with_page("temp", "To delete", None, None, HashMap::new())
        .await
        .unwrap();

    // Delete should succeed
    let result = use_cases.delete(block.id).await;
    assert!(result.is_ok());
}

// ── link ────────────────────────────────────────────────────

#[tokio::test]
async fn test_link_two_blocks() {
    let use_cases = setup();

    let source = use_cases
        .create_with_page("link-page", "Source", None, None, HashMap::new())
        .await
        .unwrap();
    let target = use_cases
        .create_with_page("link-page", "Target", None, None, HashMap::new())
        .await
        .unwrap();

    let result = use_cases.link(source.id, target.id).await;
    assert!(result.is_ok());

    // Verify the link exists by getting the source block
    let tree = use_cases.get_tree(source.id).await.unwrap();
    assert!(tree.root.refs.contains(&target.id));
}

// ── get_tree ────────────────────────────────────────────────

#[tokio::test]
async fn test_get_tree_returns_block_with_children() {
    let use_cases = setup();

    let parent = use_cases
        .create_with_page("tree-page", "Parent", None, None, HashMap::new())
        .await
        .unwrap();
    let _child1 = use_cases
        .create_with_page(
            "tree-page",
            "Child 1",
            Some(parent.id),
            None,
            HashMap::new(),
        )
        .await
        .unwrap();
    let _child2 = use_cases
        .create_with_page(
            "tree-page",
            "Child 2",
            Some(parent.id),
            None,
            HashMap::new(),
        )
        .await
        .unwrap();

    let tree = use_cases.get_tree(parent.id).await.unwrap();
    assert_eq!(tree.root.id, parent.id);
    assert!(
        tree.children.len() >= 2,
        "expected >= 2 children, got {}",
        tree.children.len()
    );
}

// ── get_backlinks ───────────────────────────────────────────

#[tokio::test]
async fn test_get_backlinks_returns_linked_blocks() {
    let use_cases = setup();

    let source = use_cases
        .create_with_page("bl-page", "Source", None, None, HashMap::new())
        .await
        .unwrap();
    let target = use_cases
        .create_with_page("bl-page", "Target", None, None, HashMap::new())
        .await
        .unwrap();

    use_cases.link(source.id, target.id).await.unwrap();

    let backlinks = use_cases.get_backlinks(target.id).await.unwrap();
    assert!(!backlinks.is_empty());
    assert!(backlinks.iter().any(|b| b.id == source.id));
}

// ── list_by_property ────────────────────────────────────────

#[tokio::test]
async fn test_list_by_property_finds_matching_blocks() {
    let use_cases = setup();
    let mut props = HashMap::new();
    props.insert(
        "created_by".to_string(),
        PropertyValue::String("agent::claude".into()),
    );

    let _block1 = use_cases
        .create_with_page("agent-page", "Block by claude", None, None, props.clone())
        .await
        .unwrap();
    let _block2 = use_cases
        .create_with_page("agent-page", "Another by claude", None, None, props)
        .await
        .unwrap();

    let results = use_cases
        .list_by_property("created_by", "agent::claude", 10)
        .await
        .unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
async fn test_list_by_property_respects_limit() {
    let use_cases = setup();
    let mut props = HashMap::new();
    props.insert(
        "created_by".to_string(),
        PropertyValue::String("user::test".into()),
    );

    for i in 0..5 {
        use_cases
            .create_with_page(
                "limit-page",
                &format!("Block {}", i),
                None,
                None,
                props.clone(),
            )
            .await
            .unwrap();
    }

    let results = use_cases
        .list_by_property("created_by", "user::test", 2)
        .await
        .unwrap();
    assert!(results.len() <= 2);
}

// ── create_task ─────────────────────────────────────────────

#[tokio::test]
async fn test_create_task_defaults_to_todo_marker() {
    let use_cases = setup();

    let task = use_cases
        .create_task("tasks", "Write tests", None, None)
        .await
        .unwrap();

    assert_eq!(task.marker, Some(TaskMarker::Todo));
}

// BUG FIXED (2026-06-02): create_task now always inserts before update.
#[tokio::test]
async fn test_create_task_with_priority() {
    let use_cases = setup();

    let task = use_cases
        .create_task("tasks", "Urgent", None, Some("A"))
        .await
        .unwrap();

    assert_eq!(
        task.priority,
        Some(quilt_domain::value_objects::Priority::A)
    );
    assert_eq!(task.marker, Some(TaskMarker::Todo));
}

// ── Page auto-creation ──────────────────────────────────────

#[tokio::test]
async fn test_creating_blocks_on_same_page_reuses_page() {
    let use_cases = setup();

    let b1 = use_cases
        .create_with_page("reuse-page", "First", None, None, HashMap::new())
        .await
        .unwrap();
    let b2 = use_cases
        .create_with_page("reuse-page", "Second", None, None, HashMap::new())
        .await
        .unwrap();

    // Both blocks should belong to the same page
    assert_eq!(b1.page_id, b2.page_id);
}
