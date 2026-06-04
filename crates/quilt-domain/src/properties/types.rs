//! Property type system - types for typed properties

use std::fmt;

/// PropertyType defines the data type of a property.
///
/// Each PropertyType has a canonical string representation and validates
/// PropertyValue for type compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
    /// Get the canonical string representation for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyType::Text => "Text",
            PropertyType::Number => "Number",
            PropertyType::Date => "Date",
            PropertyType::DateTime => "DateTime",
            PropertyType::Url => "Url",
            PropertyType::Checkbox => "Checkbox",
            PropertyType::Node => "Node",
        }
    }

    /// Parse from canonical string representation
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Text" => Some(PropertyType::Text),
            "Number" => Some(PropertyType::Number),
            "Date" => Some(PropertyType::Date),
            "DateTime" => Some(PropertyType::DateTime),
            "Url" => Some(PropertyType::Url),
            "Checkbox" => Some(PropertyType::Checkbox),
            "Node" => Some(PropertyType::Node),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Cardinality defines how many values a property can have.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum Cardinality {
    /// Single value (default)
    #[default]
    One,
    /// Multiple values (array)
    Many,
}

impl Cardinality {
    /// Get the canonical string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Cardinality::One => "one",
            Cardinality::Many => "many",
        }
    }

    /// Parse from string representation
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

/// ViewContext determines where and how a property is displayed in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
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
    /// Get the canonical string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ViewContext::Page => "page",
            ViewContext::Block => "block",
            ViewContext::Never => "never",
        }
    }

    /// Parse from string representation
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

/// ClosedValue represents a predefined option for a property with closed set semantics.
///
/// Used for properties like status (TODO, DOING, DONE) or priority (A, B, C).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClosedValue {
    /// Unique identifier
    pub id: crate::value_objects::Uuid,
    /// Database identifier (e.g., "todo", "doing")
    pub db_ident: String,
    /// Display value
    pub value: String,
    /// Optional icon (emoji or icon name)
    pub icon: Option<String>,
    /// Sort order
    pub order: f64,
}

impl ClosedValue {
    /// Create a new closed value
    pub fn new(
        id: crate::value_objects::Uuid,
        db_ident: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            id,
            db_ident: db_ident.into(),
            value: value.into(),
            icon: None,
            order: 0.0,
        }
    }

    /// Set the icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the order
    pub fn with_order(mut self, order: f64) -> Self {
        self.order = order;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_type_roundtrip() {
        let types = vec![
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
    fn test_cardinality_roundtrip() {
        assert_eq!(Cardinality::One.as_str(), "one");
        assert_eq!(Cardinality::Many.as_str(), "many");
        assert_eq!(Cardinality::from_str("one"), Some(Cardinality::One));
        assert_eq!(Cardinality::from_str("many"), Some(Cardinality::Many));
    }

    #[test]
    fn test_view_context_roundtrip() {
        assert_eq!(ViewContext::Page.as_str(), "page");
        assert_eq!(ViewContext::Block.as_str(), "block");
        assert_eq!(ViewContext::Never.as_str(), "never");
        assert_eq!(ViewContext::from_str("page"), Some(ViewContext::Page));
        assert_eq!(ViewContext::from_str("block"), Some(ViewContext::Block));
        assert_eq!(ViewContext::from_str("never"), Some(ViewContext::Never));
    }

    #[test]
    fn test_closed_value_builder() {
        let id = crate::value_objects::Uuid::new_v4();
        let cv = ClosedValue::new(id, "todo", "To Do")
            .with_icon("📋")
            .with_order(1.0);

        assert_eq!(cv.db_ident, "todo");
        assert_eq!(cv.value, "To Do");
        assert_eq!(cv.icon, Some("📋".to_string()));
        assert_eq!(cv.order, 1.0);
    }
}
