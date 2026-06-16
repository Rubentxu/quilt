//! Integration tests for MCP serialization helpers.
//!
//! Covers: block_to_json with all field combinations,
//! edge cases (None parent, None marker, None priority, collapsed),
//! and malformed/edge data (empty content, special characters).

use chrono::{TimeZone, Utc};
use quilt_domain::entities::Block;
use quilt_domain::value_objects::{BlockFormat, BlockType, Priority, TaskMarker, Uuid};
use quilt_mcp::serialization::block_to_json;
use std::collections::HashMap;

// ── Helpers ──────────────────────────────────────────────────

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 2, 15, 0, 0).unwrap()
}

fn make_block(
    id: Uuid,
    page_id: Uuid,
    parent_id: Option<Uuid>,
    content: &str,
    marker: Option<TaskMarker>,
    priority: Option<Priority>,
    collapsed: bool,
) -> Block {
    let now = fixed_time();
    Block {
        id,
        page_id,
        parent_id,
        order: 1.5,
        level: 2,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        marker,
        priority,
        content: content.to_string(),
        properties: HashMap::new(),
        refs: Vec::new(),
        tags: Vec::new(),
        scheduled: None,
        deadline: None,
        start_time: None,
        repeated: None,
        logbook: None,
        collapsed,
        created_at: now,
        updated_at: now,
        ..Default::default()
    }
}

// ── Basic structure ──────────────────────────────────────────

#[test]
fn test_block_to_json_basic() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Hello", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["id"], serde_json::json!(id.to_string()));
    assert_eq!(json["page_id"], serde_json::json!(page_id.to_string()));
    assert_eq!(json["content"], serde_json::json!("Hello"));
    assert_eq!(json["order"], serde_json::json!(1.5));
    assert_eq!(json["level"], serde_json::json!(2));
    assert_eq!(json["collapsed"], serde_json::json!(false));
}

#[test]
fn test_block_to_json_with_parent() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let parent_id = Uuid::new_v4();
    let block = make_block(id, page_id, Some(parent_id), "Child", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["parent_id"], serde_json::json!(parent_id.to_string()));
}

#[test]
fn test_block_to_json_without_parent() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Root", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["parent_id"], serde_json::Value::Null);
}

// ── Marker and Priority ──────────────────────────────────────

#[test]
fn test_block_to_json_with_todo_marker() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(
        id,
        page_id,
        None,
        "Task",
        Some(TaskMarker::Todo),
        None,
        false,
    );
    let json = block_to_json(&block);

    let marker_str = json["marker"].as_str().unwrap();
    assert!(marker_str.contains("TODO"), "marker was: {}", marker_str);
}

#[test]
fn test_block_to_json_with_done_marker() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(
        id,
        page_id,
        None,
        "Done",
        Some(TaskMarker::Done),
        None,
        false,
    );
    let json = block_to_json(&block);

    let marker_str = json["marker"].as_str().unwrap();
    assert!(marker_str.contains("DONE"), "marker was: {}", marker_str);
}

#[test]
fn test_block_to_json_without_marker() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "No marker", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["marker"], serde_json::Value::Null);
}

#[test]
fn test_block_to_json_with_priority_a() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Urgent", None, Some(Priority::A), false);
    let json = block_to_json(&block);

    let priority_str = json["priority"].as_str().unwrap();
    assert!(priority_str.contains('A'), "priority was: {}", priority_str);
}

#[test]
fn test_block_to_json_with_priority_c() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Low", None, Some(Priority::C), false);
    let json = block_to_json(&block);

    let priority_str = json["priority"].as_str().unwrap();
    assert!(priority_str.contains('C'), "priority was: {}", priority_str);
}

#[test]
fn test_block_to_json_without_priority() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "No priority", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["priority"], serde_json::Value::Null);
}

// ── Collapsed state ──────────────────────────────────────────

#[test]
fn test_block_to_json_collapsed_true() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Collapsed", None, None, true);
    let json = block_to_json(&block);

    assert_eq!(json["collapsed"], serde_json::json!(true));
}

// ── Timestamps ───────────────────────────────────────────────

#[test]
fn test_block_to_json_timestamps_are_rfc3339() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let now = fixed_time();
    let block = make_block(id, page_id, None, "Time", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["created_at"], serde_json::json!(now.to_rfc3339()));
    assert_eq!(json["updated_at"], serde_json::json!(now.to_rfc3339()));
}

// ── Edge cases ───────────────────────────────────────────────

#[test]
fn test_block_to_json_empty_content() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["content"], serde_json::json!(""));
}

#[test]
fn test_block_to_json_special_characters() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(
        id,
        page_id,
        None,
        "Hello \"world\"\nNew line\tTab",
        None,
        None,
        false,
    );
    let json = block_to_json(&block);

    let content = json["content"].as_str().unwrap();
    assert!(content.contains("Hello"));
    assert!(content.contains("world"));
    assert!(content.contains("New line"));
}

#[test]
fn test_block_to_json_unicode_content() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "日本語 ñ á é", None, None, false);
    let json = block_to_json(&block);

    assert_eq!(json["content"], serde_json::json!("日本語 ñ á é"));
}

#[test]
fn test_block_to_json_has_all_expected_keys() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Check keys", None, None, false);
    let json = block_to_json(&block);

    let expected_keys = [
        "id",
        "page_id",
        "parent_id",
        "order",
        "level",
        "content",
        "marker",
        "priority",
        "collapsed",
        "created_at",
        "updated_at",
    ];
    for key in &expected_keys {
        assert!(
            json.as_object().unwrap().contains_key(*key),
            "missing key: {}",
            key
        );
    }
}

#[test]
fn test_block_to_json_ids_are_strings_not_objects() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "IDs", None, None, false);
    let json = block_to_json(&block);

    assert!(
        json["id"].is_string(),
        "id should be string, got {:?}",
        json["id"]
    );
    assert!(json["page_id"].is_string(), "page_id should be string");
}

#[test]
fn test_block_to_json_order_is_number() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Order", None, None, false);
    let json = block_to_json(&block);

    assert!(json["order"].is_number());
    assert_eq!(json["order"], serde_json::json!(1.5));
}

#[test]
fn test_block_to_json_level_is_number() {
    let id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let block = make_block(id, page_id, None, "Level", None, None, false);
    let json = block_to_json(&block);

    assert!(json["level"].is_number());
    assert_eq!(json["level"], serde_json::json!(2));
}
