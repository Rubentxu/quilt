//! PropertyDefinition - schema for typed properties

use super::types::{Cardinality, ClosedValue, PropertyStatus, PropertyType, ViewContext};
use crate::value_objects::Uuid;
use chrono::{DateTime, Utc};

/// Maximum length for a canonical property key.
pub const MAX_KEY_LENGTH: usize = 32;

/// Regex pattern for valid canonical keys: lowercase alphanumeric + underscore.
pub const KEY_PATTERN: &str = r"^[a-z0-9_]+$";

/// PropertyDefinition defines the schema for a typed property.
///
/// Each property has a unique identifier, database identifier, display title,
/// type information, and constraints.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PropertyDefinition {
    /// Unique identifier
    pub id: Uuid,
    /// Database identifier / canonical key (snake_case, lowercase, <=32 chars)
    pub db_ident: String,
    /// Display title (human-readable)
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
    #[serde(default)]
    pub read_only: bool,

    // ── PI-2: Lifecycle & usage metadata ──

    /// Lifecycle status of this property.
    #[serde(default)]
    pub status: PropertyStatus,
    /// For Merged/Alias — the target property key this redirects to.
    #[serde(default)]
    pub alias_of: Option<String>,
    /// Number of blocks using this property (cached, updated periodically).
    #[serde(default)]
    pub block_count: u64,
    /// Number of pages using this property (cached, updated periodically).
    #[serde(default)]
    pub page_count: u64,
    /// When this property was first observed.
    #[serde(default)]
    pub first_seen_at: Option<DateTime<Utc>>,
    /// When this property was last used.
    #[serde(default)]
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl PropertyDefinition {
    /// Create a new property definition.
    ///
    /// The `db_ident` is stored as-is. Use [`Self::normalize_key`] to
    /// convert human-readable input before passing it here.
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
            status: PropertyStatus::default(),
            alias_of: None,
            block_count: 0,
            page_count: 0,
            first_seen_at: None,
            last_seen_at: None,
        }
    }

    /// Create a property definition with normalized key.
    ///
    /// Use this for user-defined properties where the key should be
    /// canonical (lowercase, snake_case, <=32 chars).
    pub fn new_normalized(
        id: Uuid,
        raw_key: impl AsRef<str>,
        title: impl Into<String>,
        property_type: PropertyType,
    ) -> Self {
        let db_ident = Self::normalize_key(raw_key.as_ref());
        Self::new(id, db_ident, title, property_type)
    }

    // ── PI-2: Key normalization & validation ──

    /// Normalize a human-readable key to canonical form.
    ///
    /// - Lowercase
    /// - Spaces/hyphens → underscores
    /// - Strip characters not in `[a-z0-9_]`
    /// - Truncate to 32 chars
    pub fn normalize_key(raw: &str) -> String {
        let normalized: String = raw
            .to_lowercase()
            .chars()
            .map(|c| if c == ' ' || c == '-' { '_' } else { c })
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let truncated = &normalized[..normalized.len().min(MAX_KEY_LENGTH)];
        truncated.to_string()
    }

    /// Validate that a key is in canonical form.
    ///
    /// Returns `Ok(())` if the key matches `[a-z0-9_]+` and `<= 32` chars.
    pub fn validate_key(key: &str) -> Result<(), KeyValidationError> {
        if key.is_empty() {
            return Err(KeyValidationError::Empty);
        }
        if key.len() > MAX_KEY_LENGTH {
            return Err(KeyValidationError::TooLong {
                max: MAX_KEY_LENGTH,
                actual: key.len(),
            });
        }
        if !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(KeyValidationError::InvalidChars {
                key: key.to_string(),
            });
        }
        Ok(())
    }

    /// Check if this property is active (not deprecated/merged/alias).
    pub fn is_active(&self) -> bool {
        self.status == PropertyStatus::Active
    }

    /// Check if this property redirects to another (merged or alias).
    pub fn is_redirect(&self) -> bool {
        matches!(self.status, PropertyStatus::Merged | PropertyStatus::Alias)
    }

    // ── Builder methods ──

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

    // ── PI-2: Lifecycle builders ──

    /// Set the lifecycle status.
    pub fn with_status(mut self, status: PropertyStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the alias target (for Merged/Alias status).
    pub fn with_alias_of(mut self, target_key: impl Into<String>) -> Self {
        self.alias_of = Some(target_key.into());
        self
    }

    /// Set usage counts.
    pub fn with_usage(mut self, block_count: u64, page_count: u64) -> Self {
        self.block_count = block_count;
        self.page_count = page_count;
        self
    }

    /// Set first/last seen timestamps.
    pub fn with_seen_at(
        mut self,
        first: Option<DateTime<Utc>>,
        last: Option<DateTime<Utc>>,
    ) -> Self {
        self.first_seen_at = first;
        self.last_seen_at = last;
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

/// Errors from canonical key validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum KeyValidationError {
    #[error("property key must not be empty")]
    Empty,
    #[error("property key too long: max {max} chars, got {actual}")]
    TooLong { max: usize, actual: usize },
    #[error("property key contains invalid characters: {key:?}")]
    InvalidChars { key: String },
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

    // ── PI-2: Key normalization ──

    #[test]
    fn normalize_key_lowercases() {
        assert_eq!(PropertyDefinition::normalize_key("Status"), "status");
    }

    #[test]
    fn normalize_key_replaces_spaces_with_underscore() {
        assert_eq!(
            PropertyDefinition::normalize_key("Review Status"),
            "review_status"
        );
    }

    #[test]
    fn normalize_key_replaces_hyphens_with_underscore() {
        assert_eq!(PropertyDefinition::normalize_key("due-date"), "due_date");
    }

    #[test]
    fn normalize_key_strips_special_chars() {
        // $ @ # are stripped; a, b, c remain
        assert_eq!(PropertyDefinition::normalize_key("a$b@c#"), "abc");
    }

    #[test]
    fn normalize_key_truncates_to_32() {
        let long = "a".repeat(50);
        let normalized = PropertyDefinition::normalize_key(&long);
        assert_eq!(normalized.len(), 32);
    }

    #[test]
    fn new_normalized_normalizes_key() {
        let prop = PropertyDefinition::new_normalized(
            Uuid::new_v4(),
            "Review Status",
            "Review Status",
            PropertyType::Text,
        );
        assert_eq!(prop.db_ident, "review_status");
    }

    #[test]
    fn new_preserves_key_as_is() {
        let prop = PropertyDefinition::new(
            Uuid::new_v4(),
            "quilt.property/status",
            "Status",
            PropertyType::Text,
        );
        assert_eq!(prop.db_ident, "quilt.property/status");
    }

    // ── PI-2: Key validation ──

    #[test]
    fn validate_key_accepts_valid() {
        assert!(PropertyDefinition::validate_key("review_status").is_ok());
        assert!(PropertyDefinition::validate_key("abc123").is_ok());
        assert!(PropertyDefinition::validate_key("a").is_ok());
    }

    #[test]
    fn validate_key_rejects_empty() {
        assert_eq!(
            PropertyDefinition::validate_key(""),
            Err(KeyValidationError::Empty)
        );
    }

    #[test]
    fn validate_key_rejects_too_long() {
        let long = "a".repeat(33);
        assert_eq!(
            PropertyDefinition::validate_key(&long),
            Err(KeyValidationError::TooLong {
                max: 32,
                actual: 33
            })
        );
    }

    #[test]
    fn validate_key_rejects_uppercase() {
        assert!(matches!(
            PropertyDefinition::validate_key("MyKey"),
            Err(KeyValidationError::InvalidChars { .. })
        ));
    }

    #[test]
    fn validate_key_rejects_spaces() {
        assert!(matches!(
            PropertyDefinition::validate_key("my key"),
            Err(KeyValidationError::InvalidChars { .. })
        ));
    }

    // ── PI-2: Lifecycle ──

    #[test]
    fn status_defaults_to_active() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert_eq!(prop.status, PropertyStatus::Active);
        assert!(prop.is_active());
    }

    #[test]
    fn with_status_builder() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_status(PropertyStatus::Deprecated);
        assert_eq!(prop.status, PropertyStatus::Deprecated);
        assert!(!prop.is_active());
    }

    #[test]
    fn merged_is_redirect() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "old_status", "Old Status", PropertyType::Text)
            .with_status(PropertyStatus::Merged)
            .with_alias_of("status");
        assert!(prop.is_redirect());
        assert_eq!(prop.alias_of, Some("status".to_string()));
    }

    #[test]
    fn alias_is_redirect() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "review", "Review", PropertyType::Text)
            .with_status(PropertyStatus::Alias)
            .with_alias_of("review_status");
        assert!(prop.is_redirect());
        assert_eq!(prop.alias_of, Some("review_status".to_string()));
    }

    #[test]
    fn active_is_not_redirect() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert!(!prop.is_redirect());
    }

    // ── PI-2: Usage metadata ──

    #[test]
    fn usage_defaults_to_zero() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert_eq!(prop.block_count, 0);
        assert_eq!(prop.page_count, 0);
    }

    #[test]
    fn with_usage_builder() {
        let prop = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_usage(42, 7);
        assert_eq!(prop.block_count, 42);
        assert_eq!(prop.page_count, 7);
    }

    // ── PI-2: Forward-compat deserialization ──

    #[test]
    fn legacy_json_without_pi2_fields_deserializes_cleanly() {
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
            "attribute": null,
            "read_only": false
        }"#;
        let def: PropertyDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.status, PropertyStatus::Active);
        assert!(def.alias_of.is_none());
        assert_eq!(def.block_count, 0);
        assert_eq!(def.page_count, 0);
        assert!(def.first_seen_at.is_none());
        assert!(def.last_seen_at.is_none());
    }
}
