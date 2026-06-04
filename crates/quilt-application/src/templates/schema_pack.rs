//! Schema Pack — G6 template metadata as JSON-in-string-property.
//!
//! V1 field set: `card_shape`, `icon`, `cssclass`, `link_verbs`,
//! `default_properties`, `display_hints`.
//!
//! Stored as the `schema-pack::` string property on a template page.
//! No `PropertyValue::Json` variant — pure string parse with serde_json.
//!
//! ## Backward Compatibility
//!
//! Legacy `card-shape::`, `icon::`, `cssclass::` properties continue to work.
//! The `with_schema_pack(legacy, pack)` function applies schema pack fields
//! as overrides, falling back to legacy values when pack fields are absent.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Display format hint for a property.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayHint {
    /// How to render this property value.
    #[serde(default)]
    pub format: DisplayFormat,
    /// Whether to hide this property in the UI.
    #[serde(default)]
    pub hidden: bool,
    /// Display order (lower = earlier).
    #[serde(default)]
    pub order: u32,
}

impl Default for DisplayHint {
    fn default() -> Self {
        Self {
            format: DisplayFormat::Raw,
            hidden: false,
            order: 0,
        }
    }
}

/// Display format for property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayFormat {
    Raw,
    Bold,
    Italic,
    Code,
}

impl Default for DisplayFormat {
    fn default() -> Self {
        DisplayFormat::Raw
    }
}

impl DisplayFormat {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            DisplayFormat::Raw => "raw",
            DisplayFormat::Bold => "bold",
            DisplayFormat::Italic => "italic",
            DisplayFormat::Code => "code",
        }
    }
}

/// One default property declaration in a schema pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DefaultProperty {
    pub key: String,
    pub value_type: String,
    pub default: String,
}

/// Map from property key → display hint.
pub type DisplayHintsMap = BTreeMap<String, DisplayHint>;

/// Schema pack — template metadata as structured JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaPack {
    /// Card shape override (e.g., "reference", "content", "inline").
    #[serde(default)]
    pub card_shape: String,
    /// Icon emoji or text.
    #[serde(default)]
    pub icon: Option<String>,
    /// CSS class(es) for custom styling.
    #[serde(default)]
    pub cssclass: Option<String>,
    /// Link verbs for reference-style cards (e.g., "see also", "references").
    #[serde(default)]
    pub link_verbs: Vec<String>,
    /// Default property values to apply when creating a new block.
    #[serde(default)]
    pub default_properties: Vec<DefaultProperty>,
    /// Per-property display hints (format, visibility, order).
    #[serde(default)]
    pub display_hints: DisplayHintsMap,
}

impl Default for SchemaPack {
    fn default() -> Self {
        Self {
            card_shape: String::new(),
            icon: None,
            cssclass: None,
            link_verbs: Vec::new(),
            default_properties: Vec::new(),
            display_hints: BTreeMap::new(),
        }
    }
}

impl SchemaPack {
    /// Parse a JSON string into a SchemaPack.
    ///
    /// Returns `Err(SchemaPackError::InvalidJson)` on malformed JSON.
    /// Returns `Err(SchemaPackError::UnknownValueType)` on unknown type strings.
    pub fn from_json(s: &str) -> Result<Self, SchemaPackError> {
        // Reject multiline JSON — schema-pack:: must be single-line
        if s.contains('\n') {
            return Err(SchemaPackError::InvalidJson(serde_json::Value::String(
                s.to_string(),
            )));
        }

        let value: serde_json::Value = serde_json::from_str(s)
            .map_err(|e| SchemaPackError::InvalidJson(serde_json::Value::String(s.to_string())))?;

        let pack: SchemaPack = serde_json::from_value(value)
            .map_err(|e| SchemaPackError::InvalidJson(serde_json::Value::String(s.to_string())))?;

        // Validate value_type values
        for dp in &pack.default_properties {
            if !is_known_value_type(&dp.value_type) {
                return Err(SchemaPackError::UnknownValueType(dp.value_type.clone()));
            }
        }

        Ok(pack)
    }
}

/// True for known JSON-ish value type strings and PropertyType canonical names.
fn is_known_value_type(t: &str) -> bool {
    matches!(
        t,
        // Legacy JSON-ish types
        "string"
            | "boolean"
            | "number"
            | "integer"
            | "float"
            | "date"
            | "array"
            | "object"
            // PropertyType canonical names (from quilt-domain)
            | "Text"
            | "Number"
            | "Date"
            | "DateTime"
            | "Url"
            | "Checkbox"
            | "Node"
    )
}

/// Errors that can occur when parsing a schema pack.
#[derive(Debug, Error, PartialEq)]
pub enum SchemaPackError {
    #[error("Invalid JSON in schema-pack property: {0}")]
    InvalidJson(serde_json::Value),

    #[error("Unknown value_type '{0}' in schema-pack default_properties")]
    UnknownValueType(String),
}

// ── Backward-compatibility helpers ─────────────────────────────────────────

/// Legacy card-shape property key.
pub const LEGACY_CARD_SHAPE_KEY: &str = "card-shape";
/// Legacy icon property key.
pub const LEGACY_ICON_KEY: &str = "icon";
/// Legacy cssclass property key.
pub const LEGACY_CSSCLASS_KEY: &str = "cssclass";

/// Merge legacy properties with a schema pack.
///
/// Schema pack fields take priority. Missing pack fields fall back
/// to legacy property values.
pub fn with_schema_pack(
    legacy_card_shape: Option<&str>,
    legacy_icon: Option<&str>,
    legacy_cssclass: Option<&str>,
    pack: &SchemaPack,
) -> (String, Option<String>, Option<String>) {
    let card_shape = if pack.card_shape.is_empty() {
        legacy_card_shape.unwrap_or("").to_string()
    } else {
        pack.card_shape.clone()
    };

    let icon = pack.icon.clone().or_else(|| legacy_icon.map(String::from));
    let cssclass = pack
        .cssclass
        .clone()
        .or_else(|| legacy_cssclass.map(String::from));

    (card_shape, icon, cssclass)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_pack_from_json_valid() {
        let json = r#"{"card_shape":"reference","icon":"📚"}"#;
        let pack = SchemaPack::from_json(json).unwrap();
        assert_eq!(pack.card_shape, "reference");
        assert_eq!(pack.icon, Some("📚".to_string()));
    }

    #[test]
    fn schema_pack_from_json_empty() {
        let pack = SchemaPack::from_json("{}").unwrap();
        assert!(pack.card_shape.is_empty());
        assert!(pack.icon.is_none());
    }

    #[test]
    fn schema_pack_invalid_json() {
        let result = SchemaPack::from_json("not json");
        assert!(matches!(result, Err(SchemaPackError::InvalidJson(_))));
    }

    #[test]
    fn schema_pack_multiline_rejected() {
        let json = "{\n  \"card_shape\": \"x\"\n}";
        let result = SchemaPack::from_json(json);
        assert!(matches!(result, Err(SchemaPackError::InvalidJson(_))));
    }

    #[test]
    fn schema_pack_unknown_value_type() {
        let json = r#"{"default_properties":[{"key":"x","value_type":"invalid","default":"y"}]}"#;
        let result = SchemaPack::from_json(json);
        assert!(matches!(
            result,
            Err(SchemaPackError::UnknownValueType(t)) if t == "invalid"
        ));
    }

    #[test]
    fn schema_pack_value_type_accepts_property_type_variants() {
        // PropertyType canonical names should be accepted
        let type_names = [
            "Text", "Number", "Date", "DateTime", "Url", "Checkbox", "Node",
        ];
        for type_name in type_names {
            let json = format!(
                r#"{{"default_properties":[{{"key":"status","value_type":"{}","default":"todo"}}]}}"#,
                type_name
            );
            let result = SchemaPack::from_json(&json);
            assert!(
                result.is_ok(),
                "Expected {} to be accepted but got {:?}",
                type_name,
                result
            );
        }
    }

    #[test]
    fn schema_pack_value_type_accepts_legacy_json_types() {
        // Legacy JSON-ish type names should still be accepted
        let type_names = ["string", "boolean", "number", "integer", "float", "date", "array", "object"];
        for type_name in type_names {
            let json = format!(
                r#"{{"default_properties":[{{"key":"status","value_type":"{}","default":"todo"}}]}}"#,
                type_name
            );
            let result = SchemaPack::from_json(&json);
            assert!(
                result.is_ok(),
                "Expected {} to be accepted but got {:?}",
                type_name,
                result
            );
        }
    }

    #[test]
    fn with_schema_pack_pack_wins() {
        // Pack card_shape overrides legacy
        let pack = SchemaPack {
            card_shape: "reference".to_string(),
            ..Default::default()
        };
        let (cs, icon, css) = with_schema_pack(Some("content"), Some("⭐"), None, &pack);
        assert_eq!(cs, "reference"); // pack wins
        assert_eq!(icon, Some("⭐".to_string())); // pack empty, legacy used
    }

    #[test]
    fn with_schema_pack_legacy_fallback() {
        let pack = SchemaPack::default();
        let (cs, icon, css) = with_schema_pack(Some("content"), Some("⭐"), Some("myclass"), &pack);
        assert_eq!(cs, "content");
        assert_eq!(icon, Some("⭐".to_string()));
        assert_eq!(css, Some("myclass".to_string()));
    }

    #[test]
    fn with_schema_pack_all_empty_legacy() {
        let pack = SchemaPack::default();
        let (cs, icon, css) = with_schema_pack(None, None, None, &pack);
        assert_eq!(cs, "");
        assert!(icon.is_none());
        assert!(css.is_none());
    }

    #[test]
    fn display_hint_default() {
        let hint = DisplayHint::default();
        assert_eq!(hint.format, DisplayFormat::Raw);
        assert!(!hint.hidden);
        assert_eq!(hint.order, 0);
    }

    #[test]
    fn display_format_as_str() {
        assert_eq!(DisplayFormat::Raw.as_str(), "raw");
        assert_eq!(DisplayFormat::Bold.as_str(), "bold");
        assert_eq!(DisplayFormat::Italic.as_str(), "italic");
        assert_eq!(DisplayFormat::Code.as_str(), "code");
    }
}
