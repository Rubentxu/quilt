//! Template use cases (ADR-0007).
//!
//! Discovers `template/*` pages, reads their `card-shape::`, `icon::`, and
//! `cssclass::` properties from the page's first block, and returns the
//! metadata to the presentation layer (MCP tools, REST endpoints, and
//! the frontend's template picker).
//!
//! The MCP agent uses these to know which templates exist and what
//! structure each one declares. The frontend uses them to render the
//! template picker with live card-shape previews.

use crate::errors::ApplicationError;
use crate::templates::schema_pack::SchemaPack;
use async_trait::async_trait;
use quilt_domain::entities::{Block, Page};
use quilt_domain::properties::types::PropertyType;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;

/// Use cases for template discovery and schema retrieval.
///
/// V1: read-only. Templates are created via the regular page/block
/// APIs (the user creates a `template/<name>` page and adds a block
/// with the card-shape/icon/cssclass properties). The use case
/// surfaces them as a typed DTO so MCP agents and the UI can list
/// them without knowing the convention.
#[async_trait]
pub trait TemplateUseCases: Send + Sync {
    /// List all template pages in the graph, with their card metadata.
    ///
    /// Returns one entry per `template/*` page. Pages without
    /// template/<name> are excluded. Entries are sorted by template
    /// name (case-insensitive) so the agent and UI see a stable
    /// order.
    async fn list_templates(&self) -> Result<Vec<TemplateSummary>, ApplicationError>;

    /// Get the full schema for a single template by its short name
    /// (the part after `template/`).
    ///
    /// Returns `Ok(None)` if the template page does not exist. The
    /// returned schema includes all blocks that live on the template
    /// page (the contract the agent must respect) and the union of
    /// properties declared across those blocks.
    async fn get_template_schema(
        &self,
        template_name: &str,
    ) -> Result<Option<TemplateSchema>, ApplicationError>;

    /// Get the schema pack (G6) for a single template by its short name.
    ///
    /// The schema pack is stored as the `schema-pack::` string property
    /// on the template page. Returns `Ok(None)` if the template page
    /// does not exist or has no schema-pack property.
    async fn get_schema_pack(
        &self,
        template_name: &str,
    ) -> Result<Option<SchemaPack>, ApplicationError>;
}

/// Summary of one template page — what the MCP tool returns to the
/// agent and what the frontend's template picker displays.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateSummary {
    /// Short name (e.g., "reference" for `template/reference`).
    pub name: String,
    /// Full page name including the `template/` prefix.
    pub full_name: String,
    /// Total blocks on the template page (including the metadata block).
    pub block_count: usize,
    /// The `card-shape::` value, defaulting to `inline` if missing.
    /// One of "reference" | "content" | "inline".
    pub card_shape: String,
    /// The `icon::` value, if declared.
    pub icon: Option<String>,
    /// The `cssclass::` value, if declared.
    pub cssclass: Option<String>,
    /// The block IDs that declare template metadata, in order.
    /// Useful for the agent when modifying the schema.
    pub metadata_block_ids: Vec<Uuid>,
}

/// Full schema of one template page. The agent uses this to know
/// what structure a block applied with `template:: <name>` should
/// have — which properties are expected, what the example block tree
/// looks like.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateSchema {
    /// Short name (e.g., "reference").
    pub name: String,
    /// Full page name.
    pub full_name: String,
    /// All blocks on the template page, in order. Includes the
    /// metadata block (the one with `card-shape::`, `icon::`, etc.)
    /// and any child blocks the user added as examples.
    pub blocks: Vec<Block>,
    /// Union of all properties declared across the template's
    /// blocks, in the order they appear.
    pub properties: Vec<TemplateProperty>,
    /// Same fields as TemplateSummary for convenience.
    pub card_shape: String,
    pub icon: Option<String>,
    pub cssclass: Option<String>,
}

/// One property of a template's contract: key + value + type hint.
#[derive(Debug, Clone, Serialize)]
pub struct TemplateProperty {
    pub key: String,
    /// Stringified value. Most template properties are simple
    /// strings; the type hint is provided alongside.
    pub value: String,
    /// "string" | "number" | "boolean" | "date" | "array" | "object"
    /// (the JSON type the value would deserialize as).
    pub r#type: String,
    /// Canonical property type from quilt-domain.
    /// None when type_hint doesn't map to a known PropertyType.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property_type: Option<PropertyType>,
}

/// Combines a property definition with display hints and default value.
/// Used to provide full schema information for template properties.
#[derive(Debug, Clone, Serialize)]
pub struct TemplatePropertySchema {
    /// The property definition from quilt-domain.
    pub property: quilt_domain::properties::PropertyDefinition,
    /// Display hints for rendering.
    pub display_hint: crate::templates::schema_pack::DisplayHint,
    /// Default value for this property, if any.
    pub default: Option<quilt_domain::value_objects::PropertyValue>,
}

// ── Implementation ───────────────────────────────────────────────

/// Generic implementation backed by the standard repositories.
pub struct TemplateUseCasesImpl<PR: PageRepository, BR: BlockRepository> {
    page_repo: Arc<PR>,
    block_repo: Arc<BR>,
}

impl<PR: PageRepository, BR: BlockRepository> TemplateUseCasesImpl<PR, BR> {
    pub fn new(page_repo: Arc<PR>, block_repo: Arc<BR>) -> Self {
        Self {
            page_repo,
            block_repo,
        }
    }
}

#[async_trait]
impl<PR: PageRepository + 'static, BR: BlockRepository + 'static> TemplateUseCases
    for TemplateUseCasesImpl<PR, BR>
{
    #[instrument(skip(self))]
    async fn list_templates(&self) -> Result<Vec<TemplateSummary>, ApplicationError> {
        let all_pages = self
            .page_repo
            .get_all()
            .await
            .map_err(ApplicationError::from)?;

        let mut summaries: Vec<TemplateSummary> = Vec::new();
        for page in all_pages {
            if !is_template_name(&page.name) {
                continue;
            }
            // Read the page's blocks to extract card metadata
            let blocks = self
                .block_repo
                .get_by_page(page.id)
                .await
                .map_err(ApplicationError::from)?;
            let summary = summarize_template(&page, &blocks);
            summaries.push(summary);
        }

        // Sort by short name (case-insensitive) for stable ordering
        summaries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(summaries)
    }

    #[instrument(skip(self))]
    async fn get_template_schema(
        &self,
        template_name: &str,
    ) -> Result<Option<TemplateSchema>, ApplicationError> {
        let full_name = format!("template/{}", template_name);
        let page = match self
            .page_repo
            .get_by_name(&full_name)
            .await
            .map_err(ApplicationError::from)?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(ApplicationError::from)?;
        let summary = summarize_template(&page, &blocks);
        let properties = collect_properties(&blocks);

        Ok(Some(TemplateSchema {
            name: summary.name,
            full_name: summary.full_name,
            blocks,
            properties,
            card_shape: summary.card_shape,
            icon: summary.icon,
            cssclass: summary.cssclass,
        }))
    }

    #[instrument(skip(self))]
    async fn get_schema_pack(
        &self,
        template_name: &str,
    ) -> Result<Option<SchemaPack>, ApplicationError> {
        let full_name = format!("template/{}", template_name);
        let page = match self
            .page_repo
            .get_by_name(&full_name)
            .await
            .map_err(ApplicationError::from)?
        {
            Some(p) => p,
            None => return Ok(None),
        };

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(ApplicationError::from)?;

        // Look for the schema-pack:: property on any block
        for block in &blocks {
            if let Some(prop) = block.properties.get(SCHEMA_PACK_KEY) {
                if let quilt_domain::value_objects::PropertyValue::String(json_str) = prop {
                    match SchemaPack::from_json(json_str) {
                        Ok(pack) => return Ok(Some(pack)),
                        Err(_) => return Ok(None),
                    }
                }
            }
        }

        Ok(None)
    }
}

/// Schema pack property key on a template page.
const SCHEMA_PACK_KEY: &str = "schema-pack";

// ── Helpers (private) ─────────────────────────────────────────────

/// Mirrors `Page::is_template_name` (crates/quilt-domain/src/entities/page.rs:160-167)
/// — duplicated here so this use case doesn't pull in the Page
/// constructor. Only checks the name format.
fn is_template_name(name: &str) -> bool {
    name == "template" || name.starts_with("template/") || name.starts_with("templates/")
}

/// Read card-shape/icon/cssclass from the first block on the
/// template page that has them. Convention: the user puts the
/// metadata on the FIRST block of the template page.
fn summarize_template(page: &Page, blocks: &[Block]) -> TemplateSummary {
    let short_name = strip_template_prefix(&page.name);
    let mut card_shape = "inline".to_string();
    let mut icon: Option<String> = None;
    let mut cssclass: Option<String> = None;
    let mut metadata_block_ids: Vec<Uuid> = Vec::new();

    for block in blocks {
        // A block is "metadata" if it carries at least one of the
        // card-shape / icon / cssclass properties. Dedupe by id
        // so a block that has all three counts only once.
        let shape = block.properties.get("card-shape").and_then(string_value);
        let ic = block.properties.get("icon").and_then(string_value);
        let cc = block.properties.get("cssclass").and_then(string_value);
        if shape.is_none() && ic.is_none() && cc.is_none() {
            continue;
        }
        if !metadata_block_ids.contains(&block.id) {
            metadata_block_ids.push(block.id);
        }
        if let Some(s) = shape {
            card_shape = s;
        }
        if let Some(i) = ic {
            icon = Some(i);
        }
        if let Some(c) = cc {
            cssclass = Some(c);
        }
    }

    TemplateSummary {
        name: short_name,
        full_name: page.name.clone(),
        block_count: blocks.len(),
        card_shape,
        icon,
        cssclass,
        metadata_block_ids,
    }
}

/// Union of all string-typed properties across the template's blocks.
/// The order matches the order of the blocks (preserving authoring
/// order for the agent).
fn collect_properties(blocks: &[Block]) -> Vec<TemplateProperty> {
    let mut out: Vec<TemplateProperty> = Vec::new();
    for block in blocks {
        for (key, value) in &block.properties {
            // Skip the card-shape/icon/cssclass that are already
            // surfaced as top-level fields.
            if matches!(key.as_str(), "card-shape" | "icon" | "cssclass") {
                continue;
            }
            // Skip reserved block-level keys that the agent should
            // not see in the template contract.
            if matches!(key.as_str(), "template" | "type" | "collapsed") {
                continue;
            }
            let (stringified, type_hint) = property_value_to_string(value);
            out.push(TemplateProperty {
                key: key.clone(),
                value: stringified,
                r#type: type_hint.clone(),
                property_type: property_type_to_domain(&type_hint),
            });
        }
    }
    out
}

fn strip_template_prefix(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("template/") {
        return rest.to_string();
    }
    if let Some(rest) = name.strip_prefix("templates/") {
        return rest.to_string();
    }
    if name == "template" || name == "templates" {
        return name.to_string();
    }
    name.to_string()
}

fn string_value(value: &quilt_domain::value_objects::PropertyValue) -> Option<String> {
    property_value_to_string(value).0.into()
}

fn property_value_to_string(
    value: &quilt_domain::value_objects::PropertyValue,
) -> (String, String) {
    use quilt_domain::value_objects::PropertyValue;
    let stringified = match value {
        PropertyValue::String(s) => s.clone(),
        PropertyValue::Boolean(b) => b.to_string(),
        PropertyValue::Integer(i) => i.to_string(),
        PropertyValue::Float(f) => f.to_string(),
        PropertyValue::Date(d) => d.to_rfc3339(),
        PropertyValue::Ref(s) => s.clone(),
        PropertyValue::Array(arr) => {
            let parts: Vec<String> = arr
                .iter()
                .map(property_value_to_string)
                .map(|(s, _)| s)
                .collect();
            format!("[{}]", parts.join(", "))
        }
    };
    let type_hint = value.type_name().to_string();
    (stringified, type_hint)
}

/// Maps a legacy type hint string (from `PropertyValue.type_name()`)
/// to a canonical `PropertyType`.
///
/// Returns `None` when the type hint doesn't map to any `PropertyType`
/// variant (e.g., "array" has no direct equivalent).
fn property_type_to_domain(type_hint: &str) -> Option<PropertyType> {
    match type_hint {
        "string" => Some(PropertyType::Text),
        "integer" | "float" => Some(PropertyType::Number),
        "date" => Some(PropertyType::Date),
        "boolean" => Some(PropertyType::Checkbox),
        "ref" => Some(PropertyType::Node),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::schema_pack::{DisplayFormat, DisplayHint};
    use quilt_domain::properties::types::PropertyType;

    #[test]
    fn strip_template_prefix_variants() {
        assert_eq!(strip_template_prefix("template/reference"), "reference");
        assert_eq!(
            strip_template_prefix("template/meeting-notes"),
            "meeting-notes"
        );
        assert_eq!(strip_template_prefix("template/nested/path"), "nested/path");
        assert_eq!(strip_template_prefix("templates/legacy"), "legacy");
        assert_eq!(strip_template_prefix("template"), "template");
        assert_eq!(strip_template_prefix("regular"), "regular");
    }

    #[test]
    fn is_template_name_variants() {
        assert!(is_template_name("template"));
        assert!(is_template_name("template/reference"));
        assert!(is_template_name("template/nested"));
        assert!(is_template_name("templates/legacy"));
        assert!(!is_template_name("regular"));
        assert!(!is_template_name("templated"));
    }

    // ── property_type_to_domain tests ─────────────────────────────────────────

    #[test]
    fn property_type_to_domain_string() {
        // "string" maps to Text
        assert_eq!(
            property_type_to_domain("string"),
            Some(PropertyType::Text)
        );
    }

    #[test]
    fn property_type_to_domain_number_variants() {
        // "integer" and "float" both map to Number
        assert_eq!(
            property_type_to_domain("integer"),
            Some(PropertyType::Number)
        );
        assert_eq!(
            property_type_to_domain("float"),
            Some(PropertyType::Number)
        );
    }

    #[test]
    fn property_type_to_domain_date() {
        assert_eq!(
            property_type_to_domain("date"),
            Some(PropertyType::Date)
        );
    }

    #[test]
    fn property_type_to_domain_boolean() {
        assert_eq!(
            property_type_to_domain("boolean"),
            Some(PropertyType::Checkbox)
        );
    }

    #[test]
    fn property_type_to_domain_ref() {
        assert_eq!(
            property_type_to_domain("ref"),
            Some(PropertyType::Node)
        );
    }

    #[test]
    fn property_type_to_domain_array_returns_none() {
        // "array" has no direct PropertyType mapping
        assert_eq!(property_type_to_domain("array"), None);
    }

    #[test]
    fn property_type_to_domain_unknown_returns_none() {
        // Completely unknown type hints return None
        assert_eq!(property_type_to_domain("unknown"), None);
        assert_eq!(property_type_to_domain(""), None);
        assert_eq!(property_type_to_domain("something-else"), None);
    }

    // ── TemplateProperty with property_type tests ───────────────────────────────

    #[test]
    fn template_property_deserializes_with_property_type() {
        use serde_json;

        #[derive(Debug, serde::Deserialize)]
        struct JsonProperty {
            key: String,
            value: String,
            #[serde(rename = "type")]
            r#type: String,
            property_type: Option<String>,
        }

        let json = r#"{"key":"title","value":"My Title","type":"string","property_type":"Text"}"#;
        let prop: JsonProperty = serde_json::from_str(json).unwrap();
        assert_eq!(prop.key, "title");
        assert_eq!(prop.value, "My Title");
        assert_eq!(prop.r#type, "string");
        assert_eq!(prop.property_type, Some("Text".to_string()));
    }

    #[test]
    fn template_property_deserializes_without_property_type_backward_compat() {
        use serde_json;

        #[derive(Debug, serde::Deserialize)]
        struct JsonProperty {
            key: String,
            value: String,
            #[serde(rename = "type")]
            r#type: String,
            property_type: Option<String>,
        }

        // Legacy JSON without property_type field should deserialize OK
        let json = r#"{"key":"title","value":"My Title","type":"string"}"#;
        let prop: JsonProperty = serde_json::from_str(json).unwrap();
        assert_eq!(prop.key, "title");
        assert_eq!(prop.property_type, None);
    }

    #[test]
    fn template_property_serializes_without_property_type_when_none() {
        use serde_json;
        use quilt_domain::properties::types::PropertyType;

        // TemplateProperty with property_type = None should NOT serialize property_type field
        let prop = TemplateProperty {
            key: "title".to_string(),
            value: "My Title".to_string(),
            r#type: "string".to_string(),
            property_type: None,
        };
        let json = serde_json::to_string(&prop).unwrap();
        assert!(!json.contains("property_type"));
        assert!(json.contains(r#""type":"string""#));
    }

    #[test]
    fn template_property_serializes_property_type_when_some() {
        use serde_json;
        use quilt_domain::properties::types::PropertyType;

        let prop = TemplateProperty {
            key: "status".to_string(),
            value: "todo".to_string(),
            r#type: "string".to_string(),
            property_type: Some(PropertyType::Text),
        };
        let json = serde_json::to_string(&prop).unwrap();
        assert!(json.contains("property_type"));
        assert!(json.contains(r#""property_type":"Text""#));
    }

    // ── TemplatePropertySchema tests ───────────────────────────────────────────

    #[test]
    fn template_property_schema_construction() {
        use quilt_domain::properties::types::PropertyType;
        use quilt_domain::value_objects::PropertyValue;

        let prop = TemplatePropertySchema {
            property: quilt_domain::properties::PropertyDefinition::new(
                quilt_domain::value_objects::Uuid::new_v4(),
                "status",
                "Status",
                PropertyType::Text,
            ),
            display_hint: DisplayHint {
                format: DisplayFormat::Bold,
                hidden: false,
                order: 1,
            },
            default: Some(PropertyValue::String("todo".to_string())),
        };

        assert_eq!(prop.property.db_ident, "status");
        assert_eq!(prop.display_hint.format, DisplayFormat::Bold);
        assert!(prop.default.is_some());
    }

    #[test]
    fn template_property_schema_default_can_be_none() {
        use quilt_domain::properties::types::PropertyType;

        let schema = TemplatePropertySchema {
            property: quilt_domain::properties::PropertyDefinition::new(
                quilt_domain::value_objects::Uuid::new_v4(),
                "tags",
                "Tags",
                PropertyType::Text,
            ),
            display_hint: DisplayHint::default(),
            default: None,
        };

        assert!(schema.default.is_none());
    }
}
