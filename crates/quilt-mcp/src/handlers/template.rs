//! Template tool handler (ADR-0007 + Q030).
//!
//! Owns:
//! - `quilt_list_templates`
//! - `quilt_get_template_schema`
//! - `quilt_reapply_template`         (F20)
//! - `quilt_get_template_schema_pack` (F20)
//! - `quilt_get_template_contract`         (Q030)
//! - `quilt_list_templates_with_contracts` (Q030)
//! - `quilt_apply_template_with_contract`  (Q030)
//!
//! Surfaces `template/*` pages and their card-shape/icon/cssclass
//! metadata to MCP agents. Agents use these tools to discover what
//! templates are available, what each template's card shape is, and
//! what properties a block with `template:: <name>` should have.
//!
//! The contract tools (Q030) let an agent fetch a template's
//! declared `TemplateContract` (which properties it requires, which
//! are locked from user edits, the contract version) and apply
//! the template to a block while the contract is enforced.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::TemplateUseCases;
use async_trait::async_trait;
use quilt_application::templates::contract::{
    ApplyTemplateWithContractError, ApplyTemplateWithContractUseCase,
};
use quilt_application::templates::reapply::{ReapplyMode, ReapplyTemplateUseCase};
use quilt_application::{BlockDto, TemplateContract};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

// ── Local wire-format DTOs ────────────────────────────────────────
//
// These mirror the existing `serde_json::json!({ ... })` shapes used
// by the handler responses. The DTOs live alongside the handler so
// the wire format stays in one place and we avoid hand-rolling JSON
// in the response path. Domain DTOs (e.g. `TemplateSummary`,
// `TemplateSchema`, `ReapplyResult`) are reused when their serialized
// shape matches the wire format exactly; the local DTOs are only for
// cases where we need a derived field (`block_count`) or a different
// blocks serializer (`block_to_json` vs `Block`).

/// Wire shape for a single template entry in `quilt_list_templates`.
///
/// `metadata_block_ids` is stringified (matches the previous
/// `json!()` behavior). Owns its strings so the wrapper can collect
/// owned items without lifetime gymnastics.
#[derive(Serialize)]
struct TemplateSummaryWire {
    name: String,
    full_name: String,
    block_count: usize,
    card_shape: String,
    icon: Option<String>,
    cssclass: Option<String>,
    metadata_block_ids: Vec<String>,
}

/// Wire shape for the `quilt_list_templates` outer response.
#[derive(Serialize)]
struct TemplateListResponse {
    count: usize,
    templates: Vec<TemplateSummaryWire>,
}

/// Wire shape for one property entry in `quilt_get_template_schema`.
///
/// Mirrors the previous `json!({"key", "value", "type"})` shape
/// exactly — `property_type` is intentionally absent to keep the
/// response lean and stable.
#[derive(Serialize)]
struct TemplatePropertyWire {
    key: String,
    value: String,
    #[serde(rename = "type")]
    kind: String,
}

/// Wire shape for `quilt_get_template_schema` — same fields as
/// `TemplateSchema` plus a derived `block_count` and blocks rendered
/// via `block_to_json` (the domain DTO serializes `Block` directly
/// which yields a different shape).
#[derive(Serialize)]
struct TemplateSchemaResponse {
    name: String,
    full_name: String,
    card_shape: String,
    icon: Option<String>,
    cssclass: Option<String>,
    block_count: usize,
    blocks: Vec<Value>,
    properties: Vec<TemplatePropertyWire>,
}

/// Wire shape for the `quilt_get_template_schema` not-found error.
#[derive(Serialize)]
struct TemplateNotFound {
    error: &'static str,
    name: String,
    message: String,
}

/// Wire shape for `quilt_reapply_template` invalid-argument error.
#[derive(Serialize)]
struct InvalidArgument {
    is_error: bool,
    error: &'static str,
    message: String,
}

/// Wire shape for `quilt_get_template_schema_pack` (both `Some` and
/// `None` branches — the option serializes to `null` naturally).
#[derive(Serialize)]
struct SchemaPackResponse<'a> {
    name: &'a str,
    schema_pack: Option<&'a quilt_application::templates::schema_pack::SchemaPack>,
}

// ── Contract (Q030) wire shapes ─────────────────────────────────

/// Wire shape for `quilt_get_template_contract` — returns the
/// `TemplateContract` directly (the domain DTO already serializes
/// with the right shape: `template_id`, `required_properties`,
/// `layout`, `locked_properties`, `version`).
#[derive(Serialize)]
struct TemplateContractResponse<'a> {
    template_id: String,
    required_properties: Vec<String>,
    layout: &'a [quilt_application::TemplateLayout],
    locked_properties: Vec<String>,
    version: u32,
}

/// Wire shape for `quilt_list_templates_with_contracts` — one
/// entry per template with both summary and contract.
#[derive(Serialize)]
struct TemplateWithContractWire {
    name: String,
    full_name: String,
    card_shape: String,
    icon: Option<String>,
    cssclass: Option<String>,
    block_count: usize,
    contract: Value,
}

/// Wire shape for the `quilt_list_templates_with_contracts` outer
/// response.
#[derive(Serialize)]
struct TemplateListWithContractsResponse {
    count: usize,
    templates: Vec<TemplateWithContractWire>,
}

/// Wire shape for an error returned by a contract tool.
#[derive(Serialize)]
struct ContractErrorWire {
    error: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    proposed_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

// ── Template tool handler ─────────────────────────────────────────

/// Template tool handler.
pub struct TemplateToolHandler {
    template_use_cases: Arc<dyn TemplateUseCases>,
    reapply_use_cases: Arc<dyn ReapplyTemplateUseCase>,
    apply_with_contract_use_cases: Arc<dyn ApplyTemplateWithContractUseCase>,
}

impl TemplateToolHandler {
    pub fn new(
        template_use_cases: Arc<dyn TemplateUseCases>,
        reapply_use_cases: Arc<dyn ReapplyTemplateUseCase>,
        apply_with_contract_use_cases: Arc<dyn ApplyTemplateWithContractUseCase>,
    ) -> Self {
        Self {
            template_use_cases,
            reapply_use_cases,
            apply_with_contract_use_cases,
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
            // ── Contract tools (Q030) ─────────────────────────────────
            Tool {
                name: "quilt_get_template_contract".to_string(),
                description: concat!(
                    "Get the contract for a template (Q030). The contract declares which ",
                    "properties the template requires, which properties are locked from user ",
                    "edits, how each property is laid out (inline / panel / locked), and the ",
                    "contract version. Use this BEFORE `quilt_apply_template_with_contract` so ",
                    "you know what to supply and can pass `caller_version` to detect drift."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "template_id": {
                            "type": "string",
                            "description": "UUID of the template page (the same id you got from `quilt_list_templates` metadata_block_ids or from the template page itself)."
                        }
                    },
                    "required": ["template_id"]
                }),
            },
            Tool {
                name: "quilt_list_templates_with_contracts".to_string(),
                description: concat!(
                    "List all templates with their contracts attached. Convenience tool that ",
                    "fuses `quilt_list_templates` and `quilt_get_template_contract` so the agent ",
                    "can see every template and its required/locked property list in one call."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "quilt_apply_template_with_contract".to_string(),
                description: concat!(
                    "Apply a template to a block, enforcing the contract (Q030). Required ",
                    "properties must be supplied in `proposed`; locked properties must match ",
                    "the template's canonical value; if `caller_version` is given, it must ",
                    "match the contract's current version. Returns the list of applied keys."
                ).to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": {
                            "type": "string",
                            "description": "UUID of the block to apply the template to."
                        },
                        "template_id": {
                            "type": "string",
                            "description": "UUID of the template page (preferred)."
                        },
                        "template_name": {
                            "type": "string",
                            "description": "Short name of the template (e.g. 'reference'). Used as fallback when `template_id` is not supplied."
                        },
                        "proposed": {
                            "type": "object",
                            "description": "Property values to apply. Keys must include every required property from the contract; locked property values must match the template's canonical value.",
                            "additionalProperties": {"type": "string"}
                        },
                        "caller_version": {
                            "type": "integer",
                            "description": "Optional version of the contract the caller previously saw. If supplied and doesn't match the current contract, the call is rejected with `version_mismatch`."
                        }
                    },
                    "required": ["block_id", "proposed"]
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

                let summaries: Vec<TemplateSummaryWire> = templates
                    .iter()
                    .map(|t| TemplateSummaryWire {
                        name: t.name.clone(),
                        full_name: t.full_name.clone(),
                        block_count: t.block_count,
                        card_shape: t.card_shape.clone(),
                        icon: t.icon.clone(),
                        cssclass: t.cssclass.clone(),
                        metadata_block_ids: t
                            .metadata_block_ids
                            .iter()
                            .map(|id| id.to_string())
                            .collect(),
                    })
                    .collect();

                let response = TemplateListResponse {
                    count: summaries.len(),
                    templates: summaries,
                };
                Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
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
                        let properties: Vec<TemplatePropertyWire> = s
                            .properties
                            .iter()
                            .map(|p| TemplatePropertyWire {
                                key: p.key.clone(),
                                value: p.value.clone(),
                                kind: p.r#type.clone(),
                            })
                            .collect();

                        let blocks: Vec<Value> = s
                            .blocks
                            .iter()
                            .map(|b| serde_json::to_value(BlockDto::from(b.clone())).unwrap_or_default())
                            .collect();

                        let response = TemplateSchemaResponse {
                            name: s.name,
                            full_name: s.full_name,
                            card_shape: s.card_shape,
                            icon: s.icon,
                            cssclass: s.cssclass,
                            block_count: s.blocks.len(),
                            blocks,
                            properties,
                        };
                        Ok(serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|e| e.to_string()))
                    }
                    None => {
                        let response = TemplateNotFound {
                            error: "template_not_found",
                            name: name.to_string(),
                            message: format!(
                                "No template page found with name `template/{}`.",
                                name
                            ),
                        };
                        Ok(serde_json::to_string_pretty(&response)
                            .unwrap_or_else(|e| e.to_string()))
                    }
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
                    .map_err(|_| format!("Invalid block_id format: {}", block_id_str))?;

                let mode_str = args
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("preserve_manual");
                let mode = match mode_str {
                    "override_all" => ReapplyMode::OverrideAll,
                    "preserve_manual" => ReapplyMode::PreserveManual,
                    _ => {
                        let err = InvalidArgument {
                            is_error: true,
                            error: "InvalidArgument",
                            message: format!(
                                "Invalid mode '{}'. Must be 'override_all' or 'preserve_manual'.",
                                mode_str
                            ),
                        };
                        return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
                    }
                };

                let result = self
                    .reapply_use_cases
                    .reapply(template_name, block_uuid, mode)
                    .await
                    .map_err(|e| e.to_string())?;

                // ReapplyResult already derives Serialize with the exact
                // field shape we want — reuse the domain DTO directly.
                Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()))
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

                let response = SchemaPackResponse {
                    name,
                    schema_pack: pack.as_ref(),
                };
                Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
            }
            // ── Contract (Q030) handlers ────────────────────────────
            "quilt_get_template_contract" => {
                self.handle_get_template_contract(args).await
            }
            "quilt_list_templates_with_contracts" => {
                self.handle_list_templates_with_contracts().await
            }
            "quilt_apply_template_with_contract" => {
                self.handle_apply_template_with_contract(args).await
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // T-14 / T-27: Evidence tiers for template tools.
    // - quilt_list_templates: universal fallback
    // - quilt_get_template_schema: sparse (page_name in evidence)
    // - quilt_reapply_template: rich (block_ids in evidence)
    // - quilt_get_template_schema_pack: sparse (universal fallback — no override)
    // - quilt_get_template_contract: sparse (page_name = template_id)
    // - quilt_list_templates_with_contracts: universal fallback
    // - quilt_apply_template_with_contract: rich (block_ids in evidence)
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
                if let Some(block_id_str) = args.get("block_id").and_then(|v| v.as_str())
                    && let Some(uuid) = quilt_domain::value_objects::Uuid::parse_str(block_id_str).ok()
                {
                    ev.block_ids = vec![uuid.into()];
                }
                Some(ev)
            }
            "quilt_get_template_contract" => {
                let mut ev = Evidence::universal_fallback(name);
                if let Some(tid) = args.get("template_id").and_then(|v| v.as_str()) {
                    ev.page_name = Some(tid.to_string());
                }
                Some(ev)
            }
            "quilt_apply_template_with_contract" => {
                // Rich tier: block_ids in evidence
                let mut ev = Evidence::universal_fallback(name);
                if let Some(block_id_str) = args.get("block_id").and_then(|v| v.as_str())
                    && let Some(uuid) = quilt_domain::value_objects::Uuid::parse_str(block_id_str).ok()
                {
                    ev.block_ids = vec![uuid.into()];
                }
                Some(ev)
            }
            _ => None, // universal fallback applied by server
        }
    }
}

// ── Contract handlers (Q030) ──────────────────────────────────────

impl TemplateToolHandler {
    /// Common helper: build a `TemplateContract` from a template
    /// schema + its page id.
    ///
    /// The contract declares:
    /// - All non-reserved template properties as `required`.
    /// - Each required key gets a layout chosen by the schema's
    ///   existing card-shape hint when present, otherwise `inline`.
    /// - The reserved `template` key is declared as `Locked` only if
    ///   the template page actually carries a `template::` property
    ///   on its blocks (i.e. only if the schema's raw blocks have
    ///   it). This keeps the contract minimal: a template that
    ///   doesn't use the reserved key doesn't pin users to it.
    /// - Version starts at 1.
    async fn build_contract_for_template(
        &self,
        template_id: quilt_domain::value_objects::Uuid,
        schema: &quilt_application::use_cases::TemplateSchema,
    ) -> TemplateContract {
        let mut builder = TemplateContract::builder().template_id(template_id);

        // Helper: convert schema property key into a required entry
        // with inline layout (default).
        for prop in &schema.properties {
            builder = builder
                .required_property(&prop.key)
                .inline_layout(&prop.key);
        }

        // Conditionally add reserved "template" key as Locked if the
        // template page actually carries it on any block.
        let template_key_present = schema
            .blocks
            .iter()
            .any(|b| b.properties.contains_key("template"));
        if template_key_present {
            builder = builder
                .required_property("template")
                .locked_layout("template");
        }

        builder
            .build()
            .expect("contract for a known template should always build")
    }

    async fn handle_get_template_contract(&self, args: &Value) -> Result<String, String> {
        let template_id_str = args
            .get("template_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let err = ContractErrorWire {
                    error: "invalid_argument",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some("Missing 'template_id'".to_string()),
                };
                serde_json::to_string_pretty(&err).unwrap_or_default()
            })?;

        let template_id = quilt_domain::value_objects::Uuid::parse_str(template_id_str).map_err(|_| {
            let err = ContractErrorWire {
                error: "invalid_argument",
                property: None,
                template_value: None,
                proposed_value: None,
                expected: None,
                actual: None,
                message: Some(format!("Invalid template_id format: {}", template_id_str)),
            };
            serde_json::to_string_pretty(&err).unwrap_or_default()
        })?;

        // Resolve the page name from the id, then look up the schema.
        // The application layer has `get_template_schema(name)`; we
        // need the page name. We do this by listing all templates
        // and finding the one whose `metadata_block_ids` contains
        // the requested id — or, simpler, we look up the page by id.
        //
        // To avoid coupling this handler to the page repository, we
        // call `list_templates` and match by full name → id. Pages
        // in the contract world are identified by their `id`, not
        // their name. We therefore resolve via the schema: each
        // schema carries the page's block list whose first block's
        // page_id IS the template page id.
        //
        // Simplest: enumerate templates and look for one whose
        // template page id matches. We do this via list_templates
        // then a per-template schema fetch.
        let templates = self
            .template_use_cases
            .list_templates()
            .await
            .map_err(|e| e.to_string())?;

        let mut found_schema: Option<quilt_application::use_cases::TemplateSchema> = None;
        for t in &templates {
            // The template page's id can be inferred from any block
            // on the page; `t.metadata_block_ids` carries those ids.
            // We can also just use the page's blocks — the schema
            // returns them. But the page's id itself is what we need.
            //
            // We use the `get_by_id` block repository call below for
            // the match: if any metadata block's page_id matches
            // the requested template_id, this is the right template.
            // However, the application layer doesn't expose page
            // resolution by id directly. So we fetch each schema
            // and check if any block in it lives on the page with
            // the given id.
            //
            // For efficiency, only fetch the schema when the page
            // name (template/<name>) matches the candidate. We then
            // look at the schema's blocks and find the one whose
            // page_id matches the requested template_id.
            let schema = match self.template_use_cases.get_template_schema(&t.name).await {
                Ok(Some(s)) => s,
                _ => continue,
            };
            // The schema doesn't directly carry the page_id, but
            // its blocks do. We check the first block's page_id.
            if let Some(first_block) = schema.blocks.first()
                && first_block.page_id == template_id
            {
                found_schema = Some(schema);
                break;
            }
        }

        let schema = match found_schema {
            Some(s) => s,
            None => {
                let err = ContractErrorWire {
                    error: "template_not_found",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some(format!(
                        "No template page found with id {}.",
                        template_id
                    )),
                };
                return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
            }
        };

        let contract = self.build_contract_for_template(template_id, &schema).await;
        let response = TemplateContractResponse {
            template_id: contract.template_id().to_string(),
            required_properties: contract
                .required_properties()
                .iter()
                .map(|k| k.to_string())
                .collect(),
            layout: contract.layout(),
            locked_properties: contract
                .locked_properties()
                .iter()
                .map(|k| k.to_string())
                .collect(),
            version: contract.version().as_u32(),
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    async fn handle_list_templates_with_contracts(&self) -> Result<String, String> {
        let templates = self
            .template_use_cases
            .list_templates()
            .await
            .map_err(|e| e.to_string())?;

        let mut entries: Vec<TemplateWithContractWire> = Vec::new();
        for t in &templates {
            // Fetch schema + build contract for this template.
            let schema = match self.template_use_cases.get_template_schema(&t.name).await {
                Ok(Some(s)) => s,
                _ => continue,
            };
            let page_id = schema.blocks.first().map(|b| b.page_id).unwrap_or_else(|| {
                quilt_domain::value_objects::Uuid::nil()
            });
            let contract = self.build_contract_for_template(page_id, &schema).await;

            // Serialize the contract as a JSON value for inlining.
            let contract_json = serde_json::to_value(&contract).unwrap_or(serde_json::Value::Null);

            entries.push(TemplateWithContractWire {
                name: t.name.clone(),
                full_name: t.full_name.clone(),
                card_shape: t.card_shape.clone(),
                icon: t.icon.clone(),
                cssclass: t.cssclass.clone(),
                block_count: t.block_count,
                contract: contract_json,
            });
        }

        let response = TemplateListWithContractsResponse {
            count: entries.len(),
            templates: entries,
        };
        Ok(serde_json::to_string_pretty(&response).unwrap_or_else(|e| e.to_string()))
    }

    async fn handle_apply_template_with_contract(&self, args: &Value) -> Result<String, String> {
        // 1. Parse block_id
        let block_id_str = args
            .get("block_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                let err = ContractErrorWire {
                    error: "invalid_argument",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some("Missing 'block_id'".to_string()),
                };
                serde_json::to_string_pretty(&err).unwrap_or_default()
            })?;
        let block_uuid = quilt_domain::value_objects::Uuid::parse_str(block_id_str).map_err(|_| {
            let err = ContractErrorWire {
                error: "invalid_argument",
                property: None,
                template_value: None,
                proposed_value: None,
                expected: None,
                actual: None,
                message: Some(format!("Invalid block_id format: {}", block_id_str)),
            };
            serde_json::to_string_pretty(&err).unwrap_or_default()
        })?;

        // 2. Resolve template id (prefer template_id, fallback to template_name)
        let template_id = if let Some(tid_str) = args.get("template_id").and_then(|v| v.as_str()) {
            quilt_domain::value_objects::Uuid::parse_str(tid_str).map_err(|_| {
                let err = ContractErrorWire {
                    error: "invalid_argument",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some(format!("Invalid template_id format: {}", tid_str)),
                };
                serde_json::to_string_pretty(&err).unwrap_or_default()
            })?
        } else if let Some(tname) = args.get("template_name").and_then(|v| v.as_str()) {
            // Resolve via the schema's first block's page_id.
            let schema = self
                .template_use_cases
                .get_template_schema(tname)
                .await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| {
                    let err = ContractErrorWire {
                        error: "template_not_found",
                        property: None,
                        template_value: None,
                        proposed_value: None,
                        expected: None,
                        actual: None,
                        message: Some(format!("No template page found with name `template/{}`.", tname)),
                    };
                    serde_json::to_string_pretty(&err).unwrap_or_default()
                })?;
            schema.blocks.first().map(|b| b.page_id).ok_or_else(|| {
                let err = ContractErrorWire {
                    error: "template_not_found",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some(format!("Template `{}` has no blocks.", tname)),
                };
                serde_json::to_string_pretty(&err).unwrap_or_default()
            })?
        } else {
            let err = ContractErrorWire {
                error: "invalid_argument",
                property: None,
                template_value: None,
                proposed_value: None,
                expected: None,
                actual: None,
                message: Some("Missing 'template_id' or 'template_name'".to_string()),
            };
            return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
        };

        // 3. Resolve the schema and build the contract.
        let schema = self
            .template_use_cases
            .get_template_schema_by_id(template_id)
            .await
            .map_err(|e| e.to_string())?;
        // If `get_template_schema_by_id` is not supported by the trait
        // (or returns None), fall back to enumerating templates.
        let schema = match schema {
            Some(s) => s,
            None => {
                let templates = self
                    .template_use_cases
                    .list_templates()
                    .await
                    .map_err(|e| e.to_string())?;
                let mut found = None;
                for t in &templates {
                    let s = match self.template_use_cases.get_template_schema(&t.name).await {
                        Ok(Some(s)) => s,
                        _ => continue,
                    };
                    if let Some(first) = s.blocks.first()
                        && first.page_id == template_id
                    {
                        found = Some(s);
                        break;
                    }
                }
                match found {
                    Some(s) => s,
                    None => {
                        let err = ContractErrorWire {
                            error: "template_not_found",
                            property: None,
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: Some(format!(
                                "No template page found with id {}.",
                                template_id
                            )),
                        };
                        return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
                    }
                }
            }
        };

        let contract = self.build_contract_for_template(template_id, &schema).await;

        // 4. Parse proposed
        let proposed_map = match args.get("proposed").and_then(|v| v.as_object()) {
            Some(m) => m
                .iter()
                .map(|(k, v)| {
                    let s = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    (k.clone(), s)
                })
                .collect::<std::collections::HashMap<_, _>>(),
            None => {
                let err = ContractErrorWire {
                    error: "invalid_argument",
                    property: None,
                    template_value: None,
                    proposed_value: None,
                    expected: None,
                    actual: None,
                    message: Some("Missing 'proposed'".to_string()),
                };
                return Ok(serde_json::to_string_pretty(&err).unwrap_or_default());
            }
        };

        // 5. Optional caller version
        let caller_version = args
            .get("caller_version")
            .and_then(|v| v.as_u64())
            .map(|n| quilt_application::Version::from_u32(n as u32));

        // 6. Find template short name for the use case (the use case
        //    uses the name to look up the schema internally).
        let template_name = schema.name.clone();

        // 7. Run the use case
        let result = self
            .apply_with_contract_use_cases
            .apply(block_uuid, &template_name, &contract, &proposed_map, caller_version)
            .await;

        match result {
            Ok(res) => Ok(serde_json::to_string_pretty(&res).unwrap_or_else(|e| e.to_string())),
            Err(e) => {
                let (error_tag, wire) = match e {
                    ApplyTemplateWithContractError::TemplateNotFound(n) => (
                        "template_not_found",
                        ContractErrorWire {
                            error: "template_not_found",
                            property: None,
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: Some(n),
                        },
                    ),
                    ApplyTemplateWithContractError::BlockNotFound(b) => (
                        "block_not_found",
                        ContractErrorWire {
                            error: "block_not_found",
                            property: None,
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: Some(b),
                        },
                    ),
                    ApplyTemplateWithContractError::MissingRequiredProperty(p) => (
                        "missing_required_property",
                        ContractErrorWire {
                            error: "missing_required_property",
                            property: Some(p),
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: None,
                        },
                    ),
                    ApplyTemplateWithContractError::LockedPropertyChanged {
                        property,
                        template_value,
                        proposed_value,
                    } => (
                        "locked_property_changed",
                        ContractErrorWire {
                            error: "locked_property_changed",
                            property: Some(property),
                            template_value: Some(template_value),
                            proposed_value: Some(proposed_value),
                            expected: None,
                            actual: None,
                            message: None,
                        },
                    ),
                    ApplyTemplateWithContractError::LockedPropertyAdded(p) => (
                        "locked_property_added",
                        ContractErrorWire {
                            error: "locked_property_added",
                            property: Some(p),
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: None,
                        },
                    ),
                    ApplyTemplateWithContractError::VersionMismatch { expected, actual } => (
                        "version_mismatch",
                        ContractErrorWire {
                            error: "version_mismatch",
                            property: None,
                            template_value: None,
                            proposed_value: None,
                            expected: Some(expected),
                            actual: Some(actual),
                            message: None,
                        },
                    ),
                    ApplyTemplateWithContractError::Infrastructure(m) => (
                        "infrastructure",
                        ContractErrorWire {
                            error: "infrastructure",
                            property: None,
                            template_value: None,
                            proposed_value: None,
                            expected: None,
                            actual: None,
                            message: Some(m),
                        },
                    ),
                };
                tracing::warn!(error = error_tag, "apply_template_with_contract failed");
                Ok(serde_json::to_string_pretty(&wire).unwrap_or_default())
            }
        }
    }
}
