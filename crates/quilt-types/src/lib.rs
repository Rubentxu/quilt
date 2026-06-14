//! Shared property type definitions for Quilt.
//!
//! This crate contains the canonical type enums used by both the backend
//! (quilt-domain) and the WASM layer (quilt-core). Both crates re-export
//! from here so there is a single source of truth.
//!
//! # Types
//!
//! - [`PropertyType`] — data type of a property value (text, number, date, etc.)
//! - [`Cardinality`] — single value vs. multiple values
//! - [`ViewContext`] — where a property is displayed in the UI
//!
//! All enums serialize as lowercase strings via `#[serde(rename_all = "lowercase")]`
//! and parse case-insensitively for backward compatibility.

use serde::{Deserialize, Serialize};
use std::fmt;

// ── PropertyType ────────────────────────────────────────────────────

/// Data type of a property value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    /// Plain text value
    Text,
    /// Numeric value (integer or float)
    Number,
    /// Date only (no time component)
    Date,
    /// Date with time
    DateTime,
    /// URL string
    Url,
    /// Boolean checkbox
    Checkbox,
    /// Reference to another block or page (node reference)
    Node,
}

impl PropertyType {
    /// Canonical lowercase string for wire format and storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyType::Text => "text",
            PropertyType::Number => "number",
            PropertyType::Date => "date",
            PropertyType::DateTime => "datetime",
            PropertyType::Url => "url",
            PropertyType::Checkbox => "checkbox",
            PropertyType::Node => "node",
        }
    }

    /// Parse from string (case-insensitive for backward compatibility).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Some(PropertyType::Text),
            "number" => Some(PropertyType::Number),
            "date" => Some(PropertyType::Date),
            "datetime" => Some(PropertyType::DateTime),
            "url" => Some(PropertyType::Url),
            "checkbox" => Some(PropertyType::Checkbox),
            "node" => Some(PropertyType::Node),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Cardinality ─────────────────────────────────────────────────────

/// Whether a property accepts single or multiple values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cardinality {
    /// Single value (default)
    #[default]
    One,
    /// Multiple values (array)
    Many,
}

impl Cardinality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Cardinality::One => "one",
            Cardinality::Many => "many",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "one" => Some(Cardinality::One),
            "many" => Some(Cardinality::Many),
            _ => None,
        }
    }
}

impl fmt::Display for Cardinality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── ViewContext ─────────────────────────────────────────────────────

/// Where a property is displayed in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewContext {
    /// Show in the page properties panel
    Page,
    /// Show inline with the block
    #[default]
    Block,
    /// Never show in UI (hidden)
    Never,
}

impl ViewContext {
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewContext::Page => "page",
            ViewContext::Block => "block",
            ViewContext::Never => "never",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "page" => Some(ViewContext::Page),
            "block" => Some(ViewContext::Block),
            "never" => Some(ViewContext::Never),
            _ => None,
        }
    }
}

impl fmt::Display for ViewContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_type_roundtrip() {
        let types = [
            PropertyType::Text,
            PropertyType::Number,
            PropertyType::Date,
            PropertyType::DateTime,
            PropertyType::Url,
            PropertyType::Checkbox,
            PropertyType::Node,
        ];
        for pt in types {
            let s = pt.as_str();
            let parsed = PropertyType::from_str(s).unwrap();
            assert_eq!(pt, parsed);
        }
    }

    #[test]
    fn property_type_case_insensitive() {
        assert_eq!(PropertyType::from_str("Text"), Some(PropertyType::Text));
        assert_eq!(PropertyType::from_str("NUMBER"), Some(PropertyType::Number));
        assert_eq!(PropertyType::from_str("date"), Some(PropertyType::Date));
    }

    #[test]
    fn cardinality_roundtrip() {
        assert_eq!(Cardinality::One.as_str(), "one");
        assert_eq!(Cardinality::Many.as_str(), "many");
    }

    #[test]
    fn view_context_roundtrip() {
        assert_eq!(ViewContext::Page.as_str(), "page");
        assert_eq!(ViewContext::Block.as_str(), "block");
        assert_eq!(ViewContext::Never.as_str(), "never");
    }
}
