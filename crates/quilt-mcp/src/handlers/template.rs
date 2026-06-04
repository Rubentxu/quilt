//! Template tool handler (ADR-0007).
//!
//! Owns: `quilt_list_templates`, `quilt_get_template_schema`.
//!
//! Surfaces `template/*` pages and their card-shape/icon/cssclass
//! metadata to MCP agents. Agents use these tools to discover what
//! templates are available, what each template's card shape is, and
//! what properties a block with `template:: <name>` should have.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::TemplateUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Template tool handler.
pub struct TemplateToolHandler {
    template_use_cases: Arc<dyn TemplateUseCases>,
}

impl TemplateToolHandler {
    pub fn new(template_use_cases: Arc<dyn TemplateUseCases>) -> Self {
        Self { template_use_cases }
    }
}

#[async_trait]
impl ToolHandler for TemplateToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_list_templates".to_string(),
                description: concat!(
                    "List all template pages in the graph (pages whose name starts with `template/`). ",
                    "Returns each template's short name, full page name, block count, and card ",
                    "metadata (card-shape, icon, cssclass). Use this before `quilt_create_block` ",
                    "to know what templates exist and how their cards will render."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "quilt_get_template_schema".to_string(),
                description: concat!(
                    "Get the full schema for one template by its short name. Returns the template's ",
                    "card-shape/icon/cssclass, the full block tree the template defines, and the ",
                    "union of properties the template declares. Use this to know exactly what ",
                    "properties to set on a block when you apply `template:: <name>` to it."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Template short name (the part after `template/`). Example: 'reference', 'documentation', 'meeting-notes'."
                        }
                    },
                    "required": ["name"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_list_templates" => {
                let templates = self
                    .template_use_cases
                    .list_templates()
                    .await
                    .map_err(|e| e.to_string())?;

                let summaries: Vec<serde_json::Value> = templates
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "full_name": t.full_name,
                            "block_count": t.block_count,
                            "card_shape": t.card_shape,
                            "icon": t.icon,
                            "cssclass": t.cssclass,
                            "metadata_block_ids": t.metadata_block_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": summaries.len(),
                    "templates": summaries,
                }))
                .unwrap_or_else(|e| e.to_string()))
            }
            "quilt_get_template_schema" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'name'")?;

                let schema = self
                    .template_use_cases
                    .get_template_schema(name)
                    .await
                    .map_err(|e| e.to_string())?;

                match schema {
                    Some(s) => {
                        let properties: Vec<serde_json::Value> = s
                            .properties
                            .iter()
                            .map(|p| {
                                serde_json::json!({
                                    "key": p.key,
                                    "value": p.value,
                                    "type": p.r#type,
                                })
                            })
                            .collect();

                        Ok(serde_json::to_string_pretty(&serde_json::json!({
                            "name": s.name,
                            "full_name": s.full_name,
                            "card_shape": s.card_shape,
                            "icon": s.icon,
                            "cssclass": s.cssclass,
                            "block_count": s.blocks.len(),
                            "blocks": s.blocks.iter().map(crate::serialization::block_to_json).collect::<Vec<_>>(),
                            "properties": properties,
                        }))
                        .unwrap_or_else(|e| e.to_string()))
                    }
                    None => Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "error": "template_not_found",
                        "name": name,
                        "message": format!("No template page found with name `template/{}`.", name),
                    }))
                    .unwrap_or_else(|e| e.to_string())),
                }
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // T-14: get_template_schema returns sparse evidence (template name
    // in page_name). list_templates uses universal fallback.
    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_get_template_schema" => {
                if let Some(n) = result.get("name").and_then(|v| v.as_str()) {
                    ev.page_name = Some(n.to_string());
                }
                Some(ev)
            }
            _ => None,
        }
    }
}
