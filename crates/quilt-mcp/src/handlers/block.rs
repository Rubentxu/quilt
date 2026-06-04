//! Block tool handler
//!
//! Owns: quilt_create_block, quilt_delete_block, quilt_link_blocks,
//!       quilt_get_block_tree, quilt_get_backlinks, quilt_create_task

use crate::handlers::ToolHandler;
use crate::protocol::{Evidence, SourceAuthority};
use crate::serialization::block_to_json;
use crate::tools::Tool;
use crate::use_cases::{BlockTree, BlockUseCases};
use async_trait::async_trait;
use quilt_application::{TaskMarker, Uuid, parse_properties};
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
            Tool {
                name: "quilt_list_blocks_by_author".to_string(),
                description: "List blocks created by a specific author (e.g. 'agent::claude' or 'user::alice'). Powers the /created-by filter and the agent-activity panel.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "author": { "type": "string", "description": "Author identifier (e.g. 'agent::claude', 'user::alice')" },
                        "limit": { "type": "integer", "description": "Max blocks to return (default 50)" }
                    },
                    "required": ["author"]
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

            "quilt_list_blocks_by_author" => {
                let author = args
                    .get("author")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'author'")?;
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(50);

                let blocks = self
                    .block_use_cases
                    .list_by_property("created_by", author, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                let result: Vec<serde_json::Value> = blocks
                    .iter()
                    .map(|b| {
                        serde_json::json!({
                            "id": b.id.to_string(),
                            "page_id": b.page_id.to_string(),
                            "content": b.content,
                            "marker": b.marker.as_ref().map(|m| format!("{:?}", m)),
                            "properties": b.properties.iter()
                                .map(|(k, v)| (k.clone(), v.to_json()))
                                .collect::<serde_json::Map<_, _>>(),
                            "created_at": b.created_at.to_rfc3339(),
                            "updated_at": b.updated_at.to_rfc3339(),
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "author": author,
                    "count": result.len(),
                    "blocks": result,
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // T-11, T-17: per-handler evidence override.
    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_create_block" | "quilt_create_task" => {
                if let Some(uuid) = result
                    .get("id")
                    .and_then(|v| v.as_str())
                    .and_then(Uuid::parse_str)
                {
                    ev.block_ids.push(uuid.into());
                }
            }
            "quilt_delete_block" => {
                if let Some(uuid) = result
                    .get("block_id")
                    .and_then(|v| v.as_str())
                    .and_then(Uuid::parse_str)
                {
                    ev.block_ids.push(uuid.into());
                }
            }
            "quilt_link_blocks" => {
                for k in ["source_id", "target_id"] {
                    if let Some(uuid) = result
                        .get(k)
                        .and_then(|v| v.as_str())
                        .and_then(Uuid::parse_str)
                    {
                        ev.block_ids.push(uuid.into());
                    }
                }
            }
            "quilt_get_block_tree" | "quilt_get_backlinks" => {
                if let Some(uuid) = result
                    .get("block")
                    .and_then(|b| b.get("id"))
                    .and_then(|v| v.as_str())
                    .and_then(Uuid::parse_str)
                {
                    ev.block_ids.push(uuid.into());
                }
                for key in ["backlinks", "children"] {
                    if let Some(arr) = result.get(key).and_then(|v| v.as_array()) {
                        for b in arr {
                            if let Some(uuid) = b
                                .get("id")
                                .and_then(|v| v.as_str())
                                .and_then(Uuid::parse_str)
                            {
                                ev.block_ids.push(uuid.into());
                            }
                        }
                    }
                }
            }
            "quilt_list_blocks_by_author" => {
                if let Some(blocks) = result.get("blocks").and_then(|v| v.as_array()) {
                    for b in blocks {
                        if let Some(uuid) = b
                            .get("id")
                            .and_then(|v| v.as_str())
                            .and_then(Uuid::parse_str)
                        {
                            ev.block_ids.push(uuid.into());
                        }
                    }
                    if let Some(first) = blocks.first() {
                        ev.source_authority = derive_authority(first);
                    }
                }
            }
            _ => return None,
        }
        Some(ev)
    }
}

fn derive_authority(block: &Value) -> Option<SourceAuthority> {
    let created_by = block
        .get("properties")
        .and_then(|p| p.get("created_by"))
        .and_then(|v| v.as_str());
    match created_by {
        Some(s) if s.starts_with("user::") => Some(SourceAuthority::Manual),
        Some(_) => Some(SourceAuthority::AutoExtracted),
        None => Some(SourceAuthority::AutoExtracted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h() -> BlockToolHandler {
        BlockToolHandler::new(Arc::new(NoopBlockUseCases))
    }

    /// Minimal stub of `BlockUseCases` that always errors. Only the
    /// shape of the trait matters here — `tool_evidence` is a pure
    /// function that does not call any use_case method.
    struct NoopBlockUseCases;

    #[async_trait]
    impl quilt_application::use_cases::BlockUseCases for NoopBlockUseCases {
        async fn create_with_page(
            &self,
            _page_name: &str,
            _content: &str,
            _parent_id: Option<quilt_application::Uuid>,
            _marker: Option<quilt_application::TaskMarker>,
            _properties: std::collections::HashMap<
                String,
                quilt_domain::value_objects::PropertyValue,
            >,
        ) -> Result<quilt_domain::entities::Block, quilt_application::ApplicationError> {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn create_task(
            &self,
            _page_name: &str,
            _content: &str,
            _deadline: Option<chrono::NaiveDate>,
            _priority: Option<&str>,
        ) -> Result<quilt_domain::entities::Block, quilt_application::ApplicationError> {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn delete(
            &self,
            _id: quilt_application::Uuid,
        ) -> Result<(), quilt_application::ApplicationError> {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn link(
            &self,
            _src: quilt_application::Uuid,
            _tgt: quilt_application::Uuid,
        ) -> Result<(), quilt_application::ApplicationError> {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn get_tree(
            &self,
            _id: quilt_application::Uuid,
        ) -> Result<quilt_application::use_cases::BlockTree, quilt_application::ApplicationError>
        {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn get_backlinks(
            &self,
            _id: quilt_application::Uuid,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_application::ApplicationError>
        {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
        async fn list_by_property(
            &self,
            _key: &str,
            _value: &str,
            _limit: usize,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_application::ApplicationError>
        {
            Err(quilt_application::ApplicationError::Validation(
                "noop".into(),
            ))
        }
    }

    #[test]
    fn test_tool_evidence_create_block_has_block_id() {
        let block_id = "11111111-2222-3333-4444-555555555555";
        let result = serde_json::json!({ "id": block_id });
        let ev = h()
            .tool_evidence("quilt_create_block", &serde_json::json!({}), &result)
            .unwrap();
        assert_eq!(ev.block_ids.len(), 1);
        assert_eq!(ev.block_ids[0].to_string(), block_id);
        assert_eq!(ev.tool_name, "quilt_create_block");
    }

    #[test]
    fn test_tool_evidence_delete_block_has_block_id() {
        let block_id = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
        let result = serde_json::json!({ "status": "deleted", "block_id": block_id });
        let ev = h()
            .tool_evidence("quilt_delete_block", &serde_json::json!({}), &result)
            .unwrap();
        assert_eq!(ev.block_ids[0].to_string(), block_id);
    }

    #[test]
    fn test_tool_evidence_link_blocks_has_both_ids() {
        let src = "11111111-2222-3333-4444-555555555555";
        let tgt = "66666666-7777-8888-9999-000000000000";
        let result = serde_json::json!({ "status": "linked", "source_id": src, "target_id": tgt });
        let ev = h()
            .tool_evidence("quilt_link_blocks", &serde_json::json!({}), &result)
            .unwrap();
        assert_eq!(ev.block_ids.len(), 2);
        assert!(ev.block_ids.iter().any(|u| u.to_string() == src));
        assert!(ev.block_ids.iter().any(|u| u.to_string() == tgt));
    }

    #[test]
    fn test_tool_evidence_list_by_author_extracts_all_ids() {
        let result = serde_json::json!({
            "author": "user::alice",
            "count": 2,
            "blocks": [
                { "id": "11111111-2222-3333-4444-555555555555" },
                { "id": "66666666-7777-8888-9999-000000000000" },
            ],
        });
        let ev = h()
            .tool_evidence(
                "quilt_list_blocks_by_author",
                &serde_json::json!({}),
                &result,
            )
            .unwrap();
        assert_eq!(ev.block_ids.len(), 2);
    }

    #[test]
    fn test_tool_evidence_list_by_author_authority_user_prefix() {
        let result = serde_json::json!({
            "author": "user::alice",
            "count": 1,
            "blocks": [{
                "id": "11111111-2222-3333-4444-555555555555",
                "properties": { "created_by": "user::alice" },
            }],
        });
        let ev = h()
            .tool_evidence(
                "quilt_list_blocks_by_author",
                &serde_json::json!({}),
                &result,
            )
            .unwrap();
        assert_eq!(ev.source_authority, Some(SourceAuthority::Manual));
    }

    #[test]
    fn test_tool_evidence_list_by_author_authority_agent_prefix() {
        let result = serde_json::json!({
            "author": "agent::claude",
            "count": 1,
            "blocks": [{
                "id": "11111111-2222-3333-4444-555555555555",
                "properties": { "created_by": "agent::claude" },
            }],
        });
        let ev = h()
            .tool_evidence(
                "quilt_list_blocks_by_author",
                &serde_json::json!({}),
                &result,
            )
            .unwrap();
        assert_eq!(ev.source_authority, Some(SourceAuthority::AutoExtracted));
    }

    #[test]
    fn test_tool_evidence_unknown_tool_returns_none() {
        let ev = h().tool_evidence(
            "quilt_search",
            &serde_json::json!({}),
            &serde_json::json!({}),
        );
        assert!(ev.is_none());
    }
}
