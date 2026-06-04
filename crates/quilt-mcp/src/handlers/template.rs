//! Template tool handler (ADR-0007).
//!
//! Owns: `quilt_list_templates`, `quilt_get_template_schema`.
//! F20 extension: `quilt_reapply_template`, `quilt_get_template_schema_pack`.
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
use quilt_application::templates::reapply::{ReapplyMode, ReapplyTemplateUseCase};
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Template tool handler.
pub struct TemplateToolHandler {
    template_use_cases: Arc<dyn TemplateUseCases>,
    reapply_use_cases: Arc<dyn ReapplyTemplateUseCase>,
}

impl TemplateToolHandler {
    pub fn new(
        template_use_cases: Arc<dyn TemplateUseCases>,
        reapply_use_cases: Arc<dyn ReapplyTemplateUseCase>,
    ) -> Self {
        Self {
            template_use_cases,
            reapply_use_cases,
        }
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
            Tool {
                name: "quilt_reapply_template".to_string(),
                description: concat!(
                    "Re-apply a template's properties to an existing block. Useful when the template ",
                    "has been updated and you want to propagate changes to blocks that were created ",
                    "from it. Supports two modes: 'override_all' (overwrites all properties) and ",
                    "'preserve_manual' (only updates properties that haven't been manually edited). ",
                    "Returns the lists of applied, preserved, and overwritten property keys."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_name": {
                            "type": "string",
                            "description": "Template short name (the part after `template/`). Example: 'reference'."
                        },
                        "block_id": {
                            "type": "string",
                            "description": "UUID of the block to reapply the template to."
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["override_all", "preserve_manual"],
                            "default": "preserve_manual",
                            "description": "Reapply mode: 'override_all' overwrites everything; 'preserve_manual' keeps manually-edited properties."
                        }
                    },
                    "required": ["template_name", "block_id"]
                }),
            },
            Tool {
                name: "quilt_get_template_schema_pack".to_string(),
                description: concat!(
                    "Get the schema pack (G6) for a template. The schema pack is stored as the ",
                    "`schema-pack::` JSON property on the template page and contains card_shape, ",
                    "icon, cssclass, link_verbs, default_properties, and display_hints. ",
                    "Returns null if the template has no schema-pack property."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Template short name (the part after `template/`). Example: 'reference'."
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
            "quilt_reapply_template" => {
                let template_name = args
                    .get("template_name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'template_name'")?;
                let block_id_str = args
                    .get("block_id")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'block_id'")?;
                let block_uuid = quilt_domain::value_objects::Uuid::parse_str(block_id_str)
                    .ok_or_else(|| format!("Invalid block_id format: {}", block_id_str))?;

                let mode_str = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("preserve_manual");
                let mode = match mode_str {
                    "override_all" => ReapplyMode::OverrideAll,
                    "preserve_manual" => ReapplyMode::PreserveManual,
                    _ => {
                        let err = serde_json::json!({
                            "is_error": true,
                            "error": "InvalidArgument",
                            "message": format!("Invalid mode '{}'. Must be 'override_all' or 'preserve_manual'.", mode_str),
                        });
                        return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
                    }
                };

                let result = self
                    .reapply_use_cases
                    .reapply(template_name, block_uuid, mode)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "applied": result.applied,
                    "preserved": result.preserved,
                    "overwritten": result.overwritten,
                }))
                .unwrap_or_else(|e| e.to_string()))
            }
            "quilt_get_template_schema_pack" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'name'")?;

                let pack = self
                    .template_use_cases
                    .get_schema_pack(name)
                    .await
                    .map_err(|e| e.to_string())?;

                match pack {
                    Some(p) => Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "name": name,
                        "schema_pack": p,
                    }))
                    .unwrap_or_else(|e| e.to_string())),
                    None => Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "name": name,
                        "schema_pack": serde_json::Value::Null,
                    }))
                    .unwrap_or_else(|e| e.to_string())),
                }
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // T-14 / T-27: Evidence tiers for template tools.
    // - quilt_list_templates: universal fallback
    // - quilt_get_template_schema: sparse (page_name in evidence)
    // - quilt_reapply_template: rich (block_ids in evidence)
    // - quilt_get_template_schema_pack: sparse (universal fallback — no override)
    fn tool_evidence(&self, name: &str, args: &Value, result: &Value) -> Option<Evidence> {
        match name {
            "quilt_get_template_schema" => {
                let mut ev = Evidence::universal_fallback(name);
                if let Some(n) = result.get("name").and_then(|v| v.as_str()) {
                    ev.page_name = Some(n.to_string());
                }
                Some(ev)
            }
            "quilt_reapply_template" => {
                // Rich tier: block_ids in evidence
                let mut ev = Evidence::universal_fallback(name);
                if let Some(block_id_str) = args.get("block_id").and_then(|v| v.as_str()) {
                    if let Some(uuid) = quilt_domain::value_objects::Uuid::parse_str(block_id_str) {
                        ev.block_ids = vec![uuid.into()];
                    }
                }
                Some(ev)
            }
            _ => None, // universal fallback applied by server
        }
    }
}
