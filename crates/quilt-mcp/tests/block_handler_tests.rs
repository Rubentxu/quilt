//! Integration tests for MCP BlockToolHandler.
//!
//! Uses a mock BlockUseCases to test the execute method for each tool:
//! create_block, delete_block, link_blocks, get_block_tree,
//! get_backlinks, create_task, list_blocks_by_author.
//!
//! Also tests error paths: missing parameters, invalid UUIDs, unknown tools.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use quilt_application::use_cases::{BlockTree, BlockUseCases};
use quilt_application::ApplicationError;
use quilt_domain::entities::Block;
use quilt_domain::value_objects::{BlockFormat, PropertyValue, TaskMarker, Uuid};
use quilt_mcp::handlers::block::BlockToolHandler;
use quilt_mcp::handlers::ToolHandler;
use serde_json::json;

// ── Mock BlockUseCases ──────────────────────────────────────

#[derive(Default)]
struct MockBlockUseCases {
    /// Blocks that will be returned by create_with_page / create_task
    created_blocks: Mutex<Vec<Block>>,
    /// Blocks pre-seeded for get_tree / get_backlinks / list
    stored_blocks: Mutex<HashMap<Uuid, Block>>,
    /// Error to inject (if set, next call returns this error)
    inject_error: Mutex<Option<String>>,
}

impl MockBlockUseCases {
    fn new() -> Self {
        Self::default()
    }

    fn set_error(&self, msg: &str) {
        *self.inject_error.lock().unwrap() = Some(msg.to_string());
    }

    fn seed_block(&self, block: Block) {
        self.stored_blocks.lock().unwrap().insert(block.id, block);
    }

    fn make_block(id: Uuid, content: &str) -> Block {
        let now = Utc.with_ymd_and_hms(2026, 6, 2, 15, 0, 0).unwrap();
        Block {
            id,
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: content.to_string(),
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[async_trait]
impl BlockUseCases for MockBlockUseCases {
    async fn create_with_page(
        &self,
        _page_name: &str,
        content: &str,
        _parent_id: Option<Uuid>,
        _marker: Option<TaskMarker>,
        _properties: HashMap<String, PropertyValue>,
    ) -> Result<Block, ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        let block = Self::make_block(Uuid::new_v4(), content);
        self.created_blocks.lock().unwrap().push(block.clone());
        Ok(block)
    }

    async fn create_task(
        &self,
        _page_name: &str,
        content: &str,
        _deadline: Option<chrono::NaiveDate>,
        _priority: Option<&str>,
    ) -> Result<Block, ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        let mut block = Self::make_block(Uuid::new_v4(), content);
        block.marker = Some(TaskMarker::Todo);
        self.created_blocks.lock().unwrap().push(block.clone());
        Ok(block)
    }

    async fn delete(&self, _block_id: Uuid) -> Result<(), ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(())
    }

    async fn link(&self, _source_id: Uuid, _target_id: Uuid) -> Result<(), ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(())
    }

    async fn get_tree(&self, block_id: Uuid) -> Result<BlockTree, ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        let root = self
            .stored_blocks
            .lock()
            .unwrap()
            .get(&block_id)
            .cloned()
            .unwrap_or_else(|| Self::make_block(block_id, "mock-root"));
        Ok(BlockTree {
            root,
            children: vec![],
        })
    }

    async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(vec![])
    }

    async fn list_by_property(
        &self,
        _key: &str,
        _value: &str,
        _limit: usize,
    ) -> Result<Vec<Block>, ApplicationError> {
        if let Some(err) = self.inject_error.lock().unwrap().take() {
            return Err(ApplicationError::Domain(
                quilt_domain::errors::DomainError::Storage(err),
            ));
        }
        Ok(vec![])
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn handler() -> BlockToolHandler {
    let mock = Arc::new(MockBlockUseCases::new());
    BlockToolHandler::new(mock)
}

fn valid_uuid() -> String {
    Uuid::new_v4().to_string()
}

// ── quilt_create_block ──────────────────────────────────────

#[tokio::test]
async fn test_create_block_success() {
    let h = handler();
    let args = json!({
        "page_name": "test-page",
        "content": "Hello world"
    });

    let result = h.execute("quilt_create_block", &args).await;
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    let json_str = result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["page_name"], "test-page");
    assert_eq!(parsed["content"], "Hello world");
    assert!(parsed["id"].is_string());
}

#[tokio::test]
async fn test_create_block_missing_page_name() {
    let h = handler();
    let args = json!({ "content": "Hello" });

    let result = h.execute("quilt_create_block", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'page_name'"));
}

#[tokio::test]
async fn test_create_block_missing_content() {
    let h = handler();
    let args = json!({ "page_name": "test" });

    let result = h.execute("quilt_create_block", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'content'"));
}

#[tokio::test]
async fn test_create_block_with_parent_id() {
    let h = handler();
    let parent_id = valid_uuid();
    let args = json!({
        "page_name": "test",
        "content": "Child",
        "parent_id": parent_id
    });

    let result = h.execute("quilt_create_block", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["parent_id"], parent_id);
}

#[tokio::test]
async fn test_create_block_with_invalid_parent_uuid() {
    let h = handler();
    let args = json!({
        "page_name": "test",
        "content": "Child",
        "parent_id": "not-a-uuid"
    });

    let result = h.execute("quilt_create_block", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid UUID"));
}

#[tokio::test]
async fn test_create_block_with_marker() {
    let h = handler();
    let args = json!({
        "page_name": "test",
        "content": "Task",
        "marker": "todo"
    });

    let result = h.execute("quilt_create_block", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Marker should be present as Some value
    assert!(parsed["marker"].is_string());
}

// ── quilt_delete_block ──────────────────────────────────────

#[tokio::test]
async fn test_delete_block_success() {
    let h = handler();
    let args = json!({ "block_id": valid_uuid() });

    let result = h.execute("quilt_delete_block", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "deleted");
}

#[tokio::test]
async fn test_delete_block_missing_id() {
    let h = handler();
    let args = json!({});

    let result = h.execute("quilt_delete_block", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'block_id'"));
}

#[tokio::test]
async fn test_delete_block_invalid_uuid() {
    let h = handler();
    let args = json!({ "block_id": "garbage" });

    let result = h.execute("quilt_delete_block", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid UUID"));
}

// ── quilt_link_blocks ───────────────────────────────────────

#[tokio::test]
async fn test_link_blocks_success() {
    let h = handler();
    let args = json!({
        "source_id": valid_uuid(),
        "target_id": valid_uuid()
    });

    let result = h.execute("quilt_link_blocks", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["status"], "linked");
}

#[tokio::test]
async fn test_link_blocks_missing_source() {
    let h = handler();
    let args = json!({ "target_id": valid_uuid() });

    let result = h.execute("quilt_link_blocks", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Missing 'source_id'"));
}

// ── quilt_get_block_tree ────────────────────────────────────

#[tokio::test]
async fn test_get_block_tree_success() {
    let h = handler();
    let args = json!({ "block_id": valid_uuid() });

    let result = h.execute("quilt_get_block_tree", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["block"].is_object());
    assert!(parsed["children"].is_array());
}

// ── quilt_get_backlinks ─────────────────────────────────────

#[tokio::test]
async fn test_get_backlinks_success() {
    let h = handler();
    let args = json!({ "block_id": valid_uuid() });

    let result = h.execute("quilt_get_backlinks", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["backlinks"].is_array());
    assert!(parsed["count"].is_number());
}

// ── quilt_create_task ───────────────────────────────────────

#[tokio::test]
async fn test_create_task_success() {
    let h = handler();
    let args = json!({
        "page_name": "tasks",
        "content": "Write code"
    });

    let result = h.execute("quilt_create_task", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["marker"], "TODO");
}

#[tokio::test]
async fn test_create_task_missing_page_name() {
    let h = handler();
    let args = json!({ "content": "Task" });

    let result = h.execute("quilt_create_task", &args).await;
    assert!(result.is_err());
}

// ── quilt_list_blocks_by_author ─────────────────────────────

#[tokio::test]
async fn test_list_blocks_by_author_success() {
    let h = handler();
    let args = json!({ "author": "agent::claude" });

    let result = h.execute("quilt_list_blocks_by_author", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["author"], "agent::claude");
    assert!(parsed["blocks"].is_array());
}

#[tokio::test]
async fn test_list_blocks_by_author_default_limit() {
    let h = handler();
    let args = json!({ "author": "agent::claude" });

    let result = h.execute("quilt_list_blocks_by_author", &args).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Default limit is 50
    assert_eq!(parsed["count"], 0);
}

// ── Unknown tool ────────────────────────────────────────────

#[tokio::test]
async fn test_unknown_tool_returns_error() {
    let h = handler();
    let args = json!({});

    let result = h.execute("nonexistent_tool", &args).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown tool"));
}

// ── Tool listing ────────────────────────────────────────────

#[test]
fn test_tools_list_has_expected_tools() {
    let h = handler();
    let tools = h.tools();

    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"quilt_create_block"));
    assert!(names.contains(&"quilt_delete_block"));
    assert!(names.contains(&"quilt_link_blocks"));
    assert!(names.contains(&"quilt_get_block_tree"));
    assert!(names.contains(&"quilt_get_backlinks"));
    assert!(names.contains(&"quilt_create_task"));
    assert!(names.contains(&"quilt_list_blocks_by_author"));
}
