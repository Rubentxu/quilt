//! MCP tool handler for annotation operations.
//!
//! Owns:
//! - `quilt_create_annotation`
//! - `quilt_list_annotations`
//! - `quilt_get_annotation`
//! - `quilt_update_annotation_status`
//! - `quilt_delete_annotation`
//! - `quilt_resolve_annotation` (convenience for `update_status` with status=resolved)

use crate::handlers::ToolHandler;
use crate::tools::Tool;
use async_trait::async_trait;
use quilt_application::{
    AnnotationDto, AnnotationScope, AnnotationStatus, AnnotationUseCases, Uuid,
};
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// MCP tool handler for annotations. Wraps [`AnnotationUseCases`].
pub struct AnnotationToolHandler {
    use_cases: Arc<dyn AnnotationUseCases>,
}

impl AnnotationToolHandler {
    /// Construct from the standard application use-case handle.
    pub fn new(use_cases: Arc<dyn AnnotationUseCases>) -> Self {
        Self { use_cases }
    }
}

#[async_trait]
impl ToolHandler for AnnotationToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_create_annotation".to_string(),
                description:
                    "Create a new annotation on a block. Use this to leave a comment or mark \
                     for an agent. Block-scope annotations need no offsets; inline annotations \
                     need both highlightStart and highlightEnd."
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Target block UUID" },
                        "scope": { "type": "string", "enum": ["block", "inline"], "description": "block or inline" },
                        "author_type": { "type": "string", "enum": ["human", "agent"], "description": "human or agent" },
                        "author_name": { "type": "string", "description": "Username or agent ID" },
                        "content": { "type": "string", "description": "Annotation text (markdown, non-empty)" },
                        "parent_annotation_id": { "type": "string", "description": "Optional parent annotation UUID for replies" },
                        "highlight_start": { "type": "integer", "description": "Byte offset (required when scope=inline)" },
                        "highlight_end": { "type": "integer", "description": "Byte offset (required when scope=inline)" }
                    },
                    "required": ["block_id", "scope", "author_type", "author_name", "content"]
                }),
            },
            Tool {
                name: "quilt_list_annotations".to_string(),
                description:
                    "List annotations, optionally filtered by block, status, scope, or author. \
                     Returns a JSON array."
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Filter by target block UUID" },
                        "status": { "type": "string", "enum": ["pending", "in_progress", "resolved", "dismissed"], "description": "Filter by status" },
                        "scope": { "type": "string", "enum": ["block", "inline"], "description": "Filter by scope" },
                        "author_name": { "type": "string", "description": "Filter by author name (exact match)" }
                    }
                }),
            },
            Tool {
                name: "quilt_get_annotation".to_string(),
                description: "Get a single annotation by id.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Annotation UUID" }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "quilt_update_annotation_status".to_string(),
                description:
                    "Update an annotation's lifecycle status. `resolved_by` is required when \
                     transitioning to `resolved`."
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Annotation UUID" },
                        "status": { "type": "string", "enum": ["pending", "in_progress", "resolved", "dismissed"], "description": "New status" },
                        "resolved_by": { "type": "string", "description": "Name of the resolver (required for status=resolved)" }
                    },
                    "required": ["id", "status"]
                }),
            },
            Tool {
                name: "quilt_delete_annotation".to_string(),
                description: "Delete an annotation. Idempotent — returns ok=true even if the id does not exist.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Annotation UUID" }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "quilt_resolve_annotation".to_string(),
                description: "Convenience: mark an annotation as resolved. Equivalent to `quilt_update_annotation_status` with status=resolved.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Annotation UUID" },
                        "resolved_by": { "type": "string", "description": "Name of the resolver" }
                    },
                    "required": ["id", "resolved_by"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_create_annotation" => self.create_annotation(args).await,
            "quilt_list_annotations" => self.list_annotations(args).await,
            "quilt_get_annotation" => self.get_annotation(args).await,
            "quilt_update_annotation_status" => self.update_status(args).await,
            "quilt_delete_annotation" => self.delete_annotation(args).await,
            "quilt_resolve_annotation" => self.resolve_annotation(args).await,
            other => Err(format!("Unknown annotation tool: {}", other)),
        }
    }
}

impl AnnotationToolHandler {
    async fn create_annotation(&self, args: &Value) -> Result<String, String> {
        let block_id = required_str(args, "block_id")?;
        let scope = required_str(args, "scope")?;
        let author_type = required_str(args, "author_type")?;
        let author_name = required_str(args, "author_name")?;
        let content = required_str(args, "content")?;
        let parent = optional_str(args, "parent_annotation_id");
        let highlight_start = optional_u32(args, "highlight_start");
        let highlight_end = optional_u32(args, "highlight_end");

        let annotation = self
            .use_cases
            .create_from_dto(
                block_id,
                scope,
                author_type,
                author_name,
                content,
                parent,
                highlight_start,
                highlight_end,
            )
            .await
            .map_err(|e| e.to_string())?;

        let dto = AnnotationDto::from(annotation);
        serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())
    }

    async fn list_annotations(&self, args: &Value) -> Result<String, String> {
        use quilt_application::AnnotationFilters;
        let mut filters = AnnotationFilters::new();
        if let Some(s) = optional_str(args, "block_id") {
            let uuid = Uuid::parse_str(s)
                .ok_or_else(|| format!("Invalid block UUID: {}", s))?;
            filters = filters.with_block_id(uuid);
        }
        if let Some(s) = optional_str(args, "status") {
            if AnnotationStatus::try_from_str(s).is_none() {
                return Err(format!(
                    "Invalid status: '{}'. Expected pending, in_progress, resolved, dismissed",
                    s
                ));
            }
            filters = filters.with_status(s);
        }
        if let Some(s) = optional_str(args, "scope") {
            let scope = AnnotationScope::try_from_str(s).ok_or_else(|| {
                format!("Invalid scope: '{}'. Expected block, inline", s)
            })?;
            filters = filters.with_scope(scope);
        }
        if let Some(s) = optional_str(args, "author_name") {
            filters = filters.with_author_name(s);
        }

        let annotations = self
            .use_cases
            .list_by_filters(&filters)
            .await
            .map_err(|e| e.to_string())?;

        let dtos: Vec<AnnotationDto> = annotations.into_iter().map(Into::into).collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "count": dtos.len(),
            "annotations": dtos,
        }))
        .map_err(|e| e.to_string())
    }

    async fn get_annotation(&self, args: &Value) -> Result<String, String> {
        let id = required_str(args, "id")?;
        let uuid = Uuid::parse_str(id).ok_or_else(|| format!("Invalid UUID: {}", id))?;
        let annotation = self
            .use_cases
            .get_by_id(uuid)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Annotation not found: {}", id))?;
        let dto = AnnotationDto::from(annotation);
        serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())
    }

    async fn update_status(&self, args: &Value) -> Result<String, String> {
        let id = required_str(args, "id")?;
        let uuid = Uuid::parse_str(id).ok_or_else(|| format!("Invalid UUID: {}", id))?;
        let status_str = required_str(args, "status")?;
        let status = AnnotationStatus::try_from_str(status_str).ok_or_else(|| {
            format!(
                "Invalid status: '{}'. Expected pending, in_progress, resolved, dismissed",
                status_str
            )
        })?;
        let resolved_by = optional_str(args, "resolved_by").map(|s| s.to_string());

        let annotation = self
            .use_cases
            .update_status(uuid, status, resolved_by)
            .await
            .map_err(|e| e.to_string())?;
        let dto = AnnotationDto::from(annotation);
        serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())
    }

    async fn delete_annotation(&self, args: &Value) -> Result<String, String> {
        let id = required_str(args, "id")?;
        let uuid = Uuid::parse_str(id).ok_or_else(|| format!("Invalid UUID: {}", id))?;
        self.use_cases.delete(uuid).await.map_err(|e| e.to_string())?;
        serde_json::to_string_pretty(&serde_json::json!({"ok": true, "id": id}))
            .map_err(|e| e.to_string())
    }

    async fn resolve_annotation(&self, args: &Value) -> Result<String, String> {
        let id = required_str(args, "id")?;
        let uuid = Uuid::parse_str(id).ok_or_else(|| format!("Invalid UUID: {}", id))?;
        let resolved_by = required_str(args, "resolved_by")?.to_string();
        let annotation = self
            .use_cases
            .resolve(uuid, resolved_by)
            .await
            .map_err(|e| e.to_string())?;
        let dto = AnnotationDto::from(annotation);
        serde_json::to_string_pretty(&dto).map_err(|e| e.to_string())
    }
}

// ── Local helpers (private to this module) ──────────────────────────

fn required_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing '{}' parameter", key))
}

fn optional_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

fn optional_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|n| n as u32)
}

// ── Tests ──────────────────────────────────────────────────────────
//
// JSON shape tests for the tool definitions. Full integration tests
// (tool execution end-to-end with the SQLite repo) live in the
// existing `server.rs` test suite (we add a few cases there too).

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_application::ApplicationError;
    use quilt_domain::entities::{Annotation, AnnotationCreate, AnnotationScope, AnnotationStatus, AuthorType};
    use quilt_domain::repositories::AnnotationFilters;
    use quilt_domain::value_objects::Uuid;

    /// Minimal no-op service — every method returns `None` or empty
    /// so the shape tests can dispatch without panicking.
    struct NullService;

    /// Helper: build a fresh test annotation.
    fn make_annotation() -> Annotation {
        Annotation::new(AnnotationCreate {
            block_id: Uuid::new_v4(),
            scope: AnnotationScope::Block,
            author_type: AuthorType::Human,
            author_name: "x".into(),
            content: "x".into(),
            parent_annotation_id: None,
            highlight_start: None,
            highlight_end: None,
        })
        .unwrap()
    }

    #[async_trait::async_trait]
    impl AnnotationUseCases for NullService {
        async fn create_from_dto(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: &str,
            _: Option<&str>,
            _: Option<u32>,
            _: Option<u32>,
        ) -> Result<Annotation, ApplicationError> {
            Ok(make_annotation())
        }
        async fn get_by_id(&self, _: Uuid) -> Result<Option<Annotation>, ApplicationError> {
            Ok(None)
        }
        async fn list_by_block(&self, _: Uuid) -> Result<Vec<Annotation>, ApplicationError> {
            Ok(vec![])
        }
        async fn list_by_filters(
            &self,
            _: &AnnotationFilters,
        ) -> Result<Vec<Annotation>, ApplicationError> {
            Ok(vec![])
        }
        async fn update_status(
            &self,
            _: Uuid,
            _: AnnotationStatus,
            _: Option<String>,
        ) -> Result<Annotation, ApplicationError> {
            Ok(make_annotation())
        }
        async fn resolve(&self, _: Uuid, _: String) -> Result<Annotation, ApplicationError> {
            Ok(make_annotation())
        }
        async fn delete(&self, _: Uuid) -> Result<(), ApplicationError> {
            Ok(())
        }
    }

    /// The handler must report all 6 annotation tools to the MCP
    /// server. Drift here would silently break the API surface.
    #[test]
    fn tools_lists_all_six() {
        let h = AnnotationToolHandler::new(Arc::new(NullService));
        let names: Vec<String> = h.tools().into_iter().map(|t| t.name).collect();
        assert_eq!(names.len(), 6);
        assert!(names.iter().any(|n| n == "quilt_create_annotation"));
        assert!(names.iter().any(|n| n == "quilt_list_annotations"));
        assert!(names.iter().any(|n| n == "quilt_get_annotation"));
        assert!(names.iter().any(|n| n == "quilt_update_annotation_status"));
        assert!(names.iter().any(|n| n == "quilt_delete_annotation"));
        assert!(names.iter().any(|n| n == "quilt_resolve_annotation"));
    }

    /// Unknown tool name must produce a clear error so the MCP
    /// server's "Unknown tool" fallback doesn't trigger and mask
    /// dispatcher bugs.
    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let h = AnnotationToolHandler::new(Arc::new(NullService));
        let res = h.execute("quilt_does_not_exist", &Value::Null).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Unknown annotation tool"));
    }

    /// Each tool's `input_schema` MUST list the required fields we
    /// document in the description. Missing required = MCP clients
    /// will silently send invalid calls.
    #[test]
    fn tool_input_schemas_declare_required_fields() {
        let h = AnnotationToolHandler::new(Arc::new(NullService));
        for tool in h.tools() {
            let required: Vec<String> = tool
                .input_schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            match tool.name.as_str() {
                "quilt_create_annotation" => {
                    for k in ["block_id", "scope", "author_type", "author_name", "content"] {
                        assert!(
                            required.contains(&k.to_string()),
                            "create annotation must require {k}, got: {required:?}"
                        );
                    }
                }
                "quilt_get_annotation" => {
                    assert!(required.contains(&"id".to_string()));
                }
                "quilt_update_annotation_status" => {
                    for k in ["id", "status"] {
                        assert!(required.contains(&k.to_string()));
                    }
                }
                "quilt_delete_annotation" => {
                    assert!(required.contains(&"id".to_string()));
                }
                "quilt_resolve_annotation" => {
                    for k in ["id", "resolved_by"] {
                        assert!(required.contains(&k.to_string()));
                    }
                }
                "quilt_list_annotations" => {
                    // List accepts all-optional filters — `required` is
                    // allowed to be empty.
                }
                other => panic!("Unexpected tool: {}", other),
            }
        }
    }
}
