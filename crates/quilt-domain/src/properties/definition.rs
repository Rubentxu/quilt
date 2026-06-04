//! PropertyDefinition - schema for typed properties

use super::types::{Cardinality, ClosedValue, PropertyType, ViewContext};
use crate::value_objects::Uuid;

/// PropertyDefinition defines the schema for a typed property.
///
/// Each property has a unique identifier, database identifier, display title,
/// type information, and constraints.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PropertyDefinition {
    /// Unique identifier
    pub id: Uuid,
    /// Database identifier (e.g., "logseq.property/status")
    pub db_ident: String,
    /// Display title
    pub title: String,
    /// The data type of this property
    pub property_type: PropertyType,
    /// Whether this property accepts single or multiple values
    pub cardinality: Cardinality,
    /// Predefined values for closed-set properties
    pub closed_values: Vec<ClosedValue>,
    /// Where to display this property in the UI
    pub view_context: ViewContext,
    /// Whether this property is publicly visible
    pub public: bool,
    /// Whether this property is queryable (searchable)
    pub queryable: bool,
    /// Whether this property is hidden in UI
    pub hidden: bool,
    /// Optional attribute for custom external storage paths
    pub attribute: Option<String>,
    /// Whether this property is read-only (system/computed).
    /// Defaults to `false` for user-defined properties. System properties
    /// (`id`, `created_at`, `updated_at`) are registered with `read_only: true`
    /// in `BUILTIN_PROPERTIES` and reject writes from `update_properties` via
    /// `DomainError::PropertyReadOnly`. Legacy JSON without this field
    /// deserializes to `false` via `#[serde(default)]`.
    #[serde(default)]
    pub read_only: bool,
}

impl PropertyDefinition {
    /// Create a new property definition
    pub fn new(
        id: Uuid,
        db_ident: impl Into<String>,
        title: impl Into<String>,
        property_type: PropertyType,
    ) -> Self {
        Self {
            id,
            db_ident: db_ident.into(),
            title: title.into(),
            property_type,
            cardinality: Cardinality::One,
            closed_values: Vec::new(),
            view_context: ViewContext::default(),
            public: true,
            queryable: true,
            hidden: false,
            attribute: None,
            read_only: false,
        }
    }

    /// Set cardinality
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    /// Set closed values
    pub fn with_closed_values(mut self, closed_values: Vec<ClosedValue>) -> Self {
        self.closed_values = closed_values;
        self
    }

    /// Set view context
    pub fn with_view_context(mut self, view_context: ViewContext) -> Self {
        self.view_context = view_context;
        self
    }

    /// Set visibility flags
    pub fn with_visibility(mut self, public: bool, queryable: bool, hidden: bool) -> Self {
        self.public = public;
        self.queryable = queryable;
        self.hidden = hidden;
        self
    }

    /// Set attribute
    pub fn with_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attribute = Some(attribute.into());
        self
    }

    /// Mark this property as read-only (or back to writable). System/computed
    /// properties like `id`, `created_at`, `updated_at` are registered with
    /// `read_only: true` in `BUILTIN_PROPERTIES`.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Check if this property has a closed set of allowed values
    pub fn has_closed_values(&self) -> bool {
        !self.closed_values.is_empty()
    }

    /// Check if a value is in the closed set
    pub fn is_value_allowed(&self, value: &str) -> bool {
        if self.closed_values.is_empty() {
            true // Open set - any value is allowed
        } else {
            self.closed_values
                .iter()
                .any(|cv| cv.value == value || cv.db_ident == value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_definition_builder() {
        let id = Uuid::new_v4();
        let prop = PropertyDefinition::new(id, "status", "Status", PropertyType::Text)
            .with_cardinality(Cardinality::One)
            .with_view_context(ViewContext::Page);

        assert_eq!(prop.db_ident, "status");
        assert_eq!(prop.title, "Status");
        assert_eq!(prop.property_type, PropertyType::Text);
        assert_eq!(prop.cardinality, Cardinality::One);
        assert_eq!(prop.view_context, ViewContext::Page);
        assert!(prop.public);
        assert!(prop.queryable);
        assert!(!prop.hidden);
        // F9: read_only defaults to false
        assert!(!prop.read_only);
    }

    #[test]
    fn test_closed_values_check() {
        let id = Uuid::new_v4();
        let closed_values = vec![
            ClosedValue::new(Uuid::new_v4(), "todo", "To Do"),
            ClosedValue::new(Uuid::new_v4(), "doing", "Doing"),
            ClosedValue::new(Uuid::new_v4(), "done", "Done"),
        ];

        let prop = PropertyDefinition::new(id, "status", "Status", PropertyType::Text)
            .with_closed_values(closed_values);

        assert!(prop.has_closed_values());
        assert!(prop.is_value_allowed("To Do"));
        assert!(prop.is_value_allowed("todo"));
        assert!(!prop.is_value_allowed("invalid"));
    }

    #[test]
    fn test_open_set_allows_any_value() {
        let id = Uuid::new_v4();
        let prop = PropertyDefinition::new(id, "name", "Name", PropertyType::Text);

        assert!(!prop.has_closed_values());
        assert!(prop.is_value_allowed("any value"));
        assert!(prop.is_value_allowed("anything"));
    }

    // ── F9: read_only flag ──

    #[test]
    fn read_only_defaults_to_false() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert!(!prop.read_only);
    }

    #[test]
    fn with_read_only_builder_sets_flag() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_read_only(true);
        assert!(prop.read_only);
    }

    #[test]
    fn with_read_only_can_toggle_back() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_read_only(true)
            .with_read_only(false);
        assert!(!prop.read_only);
    }

    #[test]
    fn legacy_json_without_read_only_field_deserializes_to_false() {
        // F9 forward-compat: a PropertyDefinition serialized before this
        // change (no `read_only` field) must deserialize with `read_only: false`
        // (the safe default — writable, not system).
        // Serde uses the variant name for enums (One, Many, Block, etc.) since
        // the types module doesn't have a custom serde representation.
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "db_ident": "my-prop",
            "title": "My Prop",
            "property_type": "Text",
            "cardinality": "One",
            "closed_values": [],
            "view_context": "Block",
            "public": true,
            "queryable": true,
            "hidden": false,
            "attribute": null
        }"#;
        let def: PropertyDefinition = serde_json::from_str(json).unwrap();
        assert!(!def.read_only);
        assert_eq!(def.db_ident, "my-prop");
    }
}
