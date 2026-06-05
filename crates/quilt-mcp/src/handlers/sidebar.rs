//! Sidebar tool handler (mcp-sidebar-state capability).
//!
//! Owns: `quilt_get_sidebar_state`.
//!
//! Surfaces the sidebar's ephemeral UI state to MCP agents. The
//! `templates[]` field is sourced live from `TemplateUseCases::list()`.
//! The other fields (`collapsed`, `active_section`, `recents[]`) are
//! V1 client-side constants — the server has no source of truth for
//! them in V1 (collapsed persistence and server-side recents are
//! deferred to `quilt-fase2-server-favorites`).
//!
//! Why a tool (not a resource)? Per design D3: sidebar state is
//! volatile UI state, not a domain entity. Resources are reserved
//! for CRUD-shaped objects (pages, blocks, templates). The tool
//! pattern keeps the contract clean — read-only, fire-and-forget,
//! no caching semantics.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use async_trait::async_trait;
use quilt_application::use_cases::TemplateUseCases;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Sidebar tool handler.
pub struct SidebarToolHandler {
    template_use_cases: Arc<dyn TemplateUseCases>,
}

impl SidebarToolHandler {
    /// Create a new sidebar tool handler.
    pub fn new(template_use_cases: Arc<dyn TemplateUseCases>) -> Self {
        Self { template_use_cases }
    }

    /// Build the sidebar state JSON payload.
    ///
    /// `collapsed`, `active_section`, and `recents[]` are V1 client-side
    /// constants (D4): the server has no source of truth for them.
    /// `templates[]` is the live mirror of `TemplateUseCases::list()`.
    /// Each entry uses the same flattened shape as
    /// `quilt_list_templates` so MCP agents see one canonical template
    /// DTO across both tools.
    #[instrument(skip(self))]
    async fn get_sidebar_state(&self) -> Result<String, String> {
        let templates = self
            .template_use_cases
            .list_templates()
            .await
            .map_err(|e| e.to_string())?;

        // Mirror the shape of `quilt_list_templates` so the agent
        // can use either tool interchangeably.
        let template_entries: Vec<serde_json::Value> = templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "full_name": t.full_name,
                    "block_count": t.block_count,
                    "card_shape": t.card_shape,
                    "icon": t.icon,
                    "cssclass": t.cssclass,
                    "metadata_block_ids": t
                        .metadata_block_ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>(),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "collapsed": false,
            "active_section": "templates",
            "templates": template_entries,
            "recents": serde_json::Value::Array(vec![]),
        }))
        .unwrap_or_else(|e| e.to_string()))
    }
}

#[async_trait]
impl ToolHandler for SidebarToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_get_sidebar_state".to_string(),
            description: concat!(
                "Get the current sidebar state for MCP agents. Returns the sidebar's ",
                "ephemeral UI state: `collapsed` (V1: always false), `active_section` ",
                "(V1: always \"templates\" — the only section whose state the server ",
                "can derive), `templates[]` (live mirror of `quilt_list_templates`), and ",
                "`recents[]` (V1: always empty — server-side recents are deferred to ",
                "quilt-fase2-server-favorites). Use this when an agent needs to ",
                "discover what templates exist before recommending a create-from-template ",
                "flow to the user."
            )
            .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        }]
    }

    #[instrument(skip(self, _args))]
    async fn execute(&self, name: &str, _args: &Value) -> Result<String, String> {
        match name {
            "quilt_get_sidebar_state" => self.get_sidebar_state().await,
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // G1 (ADR-0008): sparse evidence — universal fallback. We do
    // NOT override `tool_evidence` here, so the server injects
    // `Evidence::universal_fallback(name)` after every successful
    // call. This keeps the contract minimal: `quilt_get_sidebar_state`
    // does not touch blocks, does not resolve a single page, and
    // has no query AST — there is nothing rich to attach.
    // (Implementation note: the trait default returns `None` which
    //  triggers the universal fallback. Nothing to write here in V1.)
    fn tool_evidence(
        &self,
        _name: &str,
        _args: &Value,
        _result: &Value,
    ) -> Option<Evidence> {
        None
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_application::errors::ApplicationError;
    use quilt_application::use_cases::{
        TemplateSchema, TemplateSummary, TemplateUseCases,
    };
    use quilt_application::templates::schema_pack::SchemaPack;

    /// Minimal in-memory `TemplateUseCases` mock. Returns the templates
    /// stored in `list_result` for `list_templates()`. The other two
    /// methods are unused by `SidebarToolHandler` but the trait requires
    /// them — we return harmless `Ok(None)` defaults.
    struct MockTemplateUseCases {
        list_result: Vec<TemplateSummary>,
    }

    impl MockTemplateUseCases {
        fn new(list_result: Vec<TemplateSummary>) -> Self {
            Self { list_result }
        }
    }

    #[async_trait]
    impl TemplateUseCases for MockTemplateUseCases {
        async fn list_templates(&self) -> Result<Vec<TemplateSummary>, ApplicationError> {
            Ok(self.list_result.clone())
        }

        async fn get_template_schema(
            &self,
            _template_name: &str,
        ) -> Result<Option<TemplateSchema>, ApplicationError> {
            Ok(None)
        }

        async fn get_schema_pack(
            &self,
            _template_name: &str,
        ) -> Result<Option<SchemaPack>, ApplicationError> {
            Ok(None)
        }
    }

    fn make_handler_with_templates(
        templates: Vec<TemplateSummary>,
    ) -> SidebarToolHandler {
        let use_cases: Arc<dyn TemplateUseCases> =
            Arc::new(MockTemplateUseCases::new(templates));
        SidebarToolHandler::new(use_cases)
    }

    fn make_handler_empty() -> SidebarToolHandler {
        make_handler_with_templates(vec![])
    }

    fn make_summary(name: &str, full_name: &str, block_count: usize) -> TemplateSummary {
        TemplateSummary {
            name: name.to_string(),
            full_name: full_name.to_string(),
            block_count,
            card_shape: "inline".to_string(),
            icon: None,
            cssclass: None,
            metadata_block_ids: vec![],
        }
    }

    // T-RED-1: tools() registers exactly one tool, named
    // `quilt_get_sidebar_state`, with an empty args schema.
    #[test]
    fn test_tools_registers_quilt_get_sidebar_state() {
        let handler = make_handler_empty();
        let tools = handler.tools();
        assert_eq!(tools.len(), 1, "expected exactly 1 tool");
        assert_eq!(tools[0].name, "quilt_get_sidebar_state");
        // Args schema is an empty object — no params required.
        let schema = &tools[0].input_schema;
        assert_eq!(schema["type"], "object");
        assert!(
            schema.get("properties").map(|p| p.as_object().map(|o| o.is_empty()).unwrap_or(false)).unwrap_or(true),
            "input_schema.properties must be empty/absent"
        );
    }

    // T-RED-2: execute() with empty templates returns the V1
    // constants exactly: collapsed=false, active_section="templates",
    // templates=[], recents=[].
    #[tokio::test]
    async fn test_execute_returns_v1_constants_with_empty_templates() {
        let handler = make_handler_empty();
        let result = handler
            .execute("quilt_get_sidebar_state", &serde_json::json!({}))
            .await
            .expect("execute must succeed");
        let v: serde_json::Value =
            serde_json::from_str(&result).expect("response must be valid JSON");
        assert_eq!(v["collapsed"], false, "V1 collapsed must be false");
        assert_eq!(
            v["active_section"], "templates",
            "V1 active_section must be the static string \"templates\""
        );
        assert!(
            v["templates"].is_array(),
            "templates must be a JSON array"
        );
        assert_eq!(
            v["templates"].as_array().unwrap().len(),
            0,
            "templates must be empty when no templates exist"
        );
        assert!(
            v["recents"].is_array(),
            "recents must be a JSON array"
        );
        assert_eq!(
            v["recents"].as_array().unwrap().len(),
            0,
            "recents must be [] in V1 (server-side recents deferred)"
        );
    }

    // T-RED-3: execute() forwards the use-case result to the JSON
    // `templates` array, mirroring the TemplateSummary shape (no
    // fields added, none dropped per design D4).
    #[tokio::test]
    async fn test_execute_returns_templates_from_use_case() {
        let handler = make_handler_with_templates(vec![
            make_summary("reference", "template/reference", 3),
            make_summary("meeting-notes", "template/meeting-notes", 5),
        ]);
        let result = handler
            .execute("quilt_get_sidebar_state", &serde_json::json!({}))
            .await
            .expect("execute must succeed");
        let v: serde_json::Value =
            serde_json::from_str(&result).expect("response must be valid JSON");
        let templates = v["templates"]
            .as_array()
            .expect("templates must be a JSON array");
        assert_eq!(templates.len(), 2);

        // First entry — mirrors the TemplateSummary shape.
        assert_eq!(templates[0]["name"], "reference");
        assert_eq!(templates[0]["full_name"], "template/reference");
        assert_eq!(templates[0]["block_count"], 3);
        assert_eq!(templates[0]["card_shape"], "inline");
        assert!(templates[0]["icon"].is_null());
        assert!(templates[0]["cssclass"].is_null());
        assert!(templates[0]["metadata_block_ids"].is_array());
        assert_eq!(templates[0]["metadata_block_ids"].as_array().unwrap().len(), 0);

        // Second entry.
        assert_eq!(templates[1]["name"], "meeting-notes");
        assert_eq!(templates[1]["block_count"], 5);
    }

    // T-RED-4: tool_evidence returns None (universal fallback per G1).
    // This is the sparse-evidence contract for ephemeral UI state.
    #[test]
    fn test_tool_evidence_returns_none_for_universal_fallback() {
        let handler = make_handler_empty();
        let args = serde_json::json!({});
        let result = serde_json::json!({
            "collapsed": false,
            "active_section": "templates",
            "templates": [],
            "recents": []
        });
        let ev = handler.tool_evidence("quilt_get_sidebar_state", &args, &result);
        assert!(
            ev.is_none(),
            "tool_evidence must return None so the server injects the sparse universal fallback"
        );
    }

    // T-RED-5 (extra): unknown tool name returns Err. Defensive — the
    // server's `execute_tool` already short-circuits to "Unknown tool"
    // for tools not in any handler's `tools()`, but the handler must
    // still reject names it does not own.
    #[tokio::test]
    async fn test_execute_unknown_tool_returns_err() {
        let handler = make_handler_empty();
        let result = handler
            .execute("quilt_nope", &serde_json::json!({}))
            .await;
        assert!(result.is_err(), "unknown tool must return Err");
        assert!(result.unwrap_err().contains("Unknown tool"));
    }
}
