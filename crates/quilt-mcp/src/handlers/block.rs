//! Block tool handler
//!
//! Owns: quilt_create_block, quilt_delete_block, quilt_link_blocks,
//!       quilt_get_block_tree, quilt_get_backlinks, quilt_create_task

use crate::handlers::ToolHandler;
use crate::serialization::block_to_json;
use crate::tools::Tool;
use crate::use_cases::{BlockTree, BlockUseCases};
use async_trait::async_trait;
use quilt_application::{parse_properties, TaskMarker, Uuid};
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Parse a UUID from JSON args.
fn parse_uuid(args: &Value, key: &str) -> Result<Uuid, String> {
    let s = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing '{}' parameter", key))?;
    Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))
}

/// Parse an optional UUID from JSON args.
fn parse_optional_uuid(args: &Value, key: &str) -> Result<Option<Uuid>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(Some(
            Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))?,
        )),
        _ => Ok(None),
    }
}

/// Parse an optional task marker from JSON args.
fn parse_optional_marker(args: &Value, key: &str) -> Result<Option<TaskMarker>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some("now") => Ok(Some(TaskMarker::Now)),
        Some("later") => Ok(Some(TaskMarker::Later)),
        Some("todo") => Ok(Some(TaskMarker::Todo)),
        Some("done") => Ok(Some(TaskMarker::Done)),
        Some("cancelled") => Ok(Some(TaskMarker::Cancelled)),
        Some(s) if s.is_empty() => Ok(None),
        None => Ok(None),
        Some(other) => Err(format!("Invalid marker: {}", other)),
    }
}

/// Convert BlockTree to JSON value.
fn block_tree_to_json(tree: &BlockTree) -> serde_json::Value {
    serde_json::json!({
        "block": block_to_json(&tree.root),
        "children": tree.children.iter().map(block_to_json).collect::<Vec<_>>(),
        "children_count": tree.children.len(),
    })
}

/// Block tool handler.
pub struct BlockToolHandler {
    block_use_cases: Arc<dyn BlockUseCases>,
}

impl BlockToolHandler {
    pub fn new(block_use_cases: Arc<dyn BlockUseCases>) -> Self {
        Self { block_use_cases }
    }
}

#[async_trait]
impl ToolHandler for BlockToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_create_block".to_string(),
                description: "Create a new block on a page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "content": { "type": "string", "description": "Block content (markdown)" },
                        "parent_id": { "type": "string", "description": "Parent block UUID (optional)" },
                        "marker": { "type": "string", "description": "Task marker: now, later, todo, done, cancelled (optional)" },
                        "properties": { "type": "object", "description": "Block properties (optional)" }
                    },
                    "required": ["page_name", "content"]
                }),
            },
            Tool {
                name: "quilt_delete_block".to_string(),
                description: "Delete a block (soft-delete to recycle bin)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Block UUID" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "quilt_link_blocks".to_string(),
                description: "Link one block to another (create a reference)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": { "type": "string", "description": "Source block UUID" },
                        "target_id": { "type": "string", "description": "Target block UUID" }
                    },
                    "required": ["source_id", "target_id"]
                }),
            },
            Tool {
                name: "quilt_get_block_tree".to_string(),
                description: "Get a block with all its children recursively".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Block UUID (root)" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "quilt_get_backlinks".to_string(),
                description: "Get all backlinks pointing to a block".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Target block UUID" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "quilt_create_task".to_string(),
                description: "Create a task with optional deadline".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "content": { "type": "string", "description": "Task content" },
                        "deadline": { "type": "string", "description": "Deadline date YYYY-MM-DD (optional)" },
                        "priority": { "type": "string", "description": "Priority: a, b, or c (optional)" }
                    },
                    "required": ["page_name", "content"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_create_block" => {
                let page_name = args
                    .get("page_name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'page_name'")?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content'")?;
                let parent_id = parse_optional_uuid(args, "parent_id")?;
                let marker = parse_optional_marker(args, "marker")?;
                let properties = args
                    .get("properties")
                    .and_then(|v| v.as_object())
                    .map(parse_properties)
                    .unwrap_or_default();

                let block = self
                    .block_use_cases
                    .create_with_page(page_name, content, parent_id, marker, properties)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "id": block.id.to_string(),
                    "page_id": block.page_id.to_string(),
                    "page_name": page_name,
                    "content": content,
                    "parent_id": parent_id.map(|id| id.to_string()),
                    "marker": marker.map(|m| format!("{:?}", m)),
                    "properties": serde_json::Map::from_iter(
                        block.properties.iter()
                            .map(|(k, v)| (k.clone(), v.to_json()))
                    ),
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            "quilt_delete_block" => {
                let block_id = parse_uuid(args, "block_id")?;
                self.block_use_cases
                    .delete(block_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "status": "deleted",
                    "block_id": block_id.to_string(),
                })
                .to_string())
            }

            "quilt_link_blocks" => {
                let source_id = parse_uuid(args, "source_id")?;
                let target_id = parse_uuid(args, "target_id")?;
                self.block_use_cases
                    .link(source_id, target_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "status": "linked",
                    "source_id": source_id.to_string(),
                    "target_id": target_id.to_string(),
                })
                .to_string())
            }

            "quilt_get_block_tree" => {
                let block_id = parse_uuid(args, "block_id")?;
                let tree = self
                    .block_use_cases
                    .get_tree(block_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&block_tree_to_json(&tree))
                    .unwrap_or_else(|e| e.to_string()))
            }

            "quilt_get_backlinks" => {
                let block_id = parse_uuid(args, "block_id")?;
                let backlinks = self
                    .block_use_cases
                    .get_backlinks(block_id)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "block_id": block_id.to_string(),
                    "backlinks": backlinks.iter().map(block_to_json).collect::<Vec<_>>(),
                    "count": backlinks.len(),
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            "quilt_create_task" => {
                let page_name = args
                    .get("page_name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'page_name'")?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content'")?;
                let deadline = args
                    .get("deadline")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
                let priority = args.get("priority").and_then(|v| v.as_str());

                let block = self
                    .block_use_cases
                    .create_task(page_name, content, deadline, priority)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "id": block.id.to_string(),
                    "page_name": page_name,
                    "content": content,
                    "marker": "TODO",
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}
