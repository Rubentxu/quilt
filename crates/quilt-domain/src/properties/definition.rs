//! PropertyDefinition - schema for typed properties

use super::types::{
    Cardinality, ClosedValue, DerivedSource, MergePolicy, PropertyMutability, PropertyStatus,
    PropertyType, PropertyVisibility, ViewContext,
};
use crate::errors::DomainError;
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
    /// Optional attribute for custom external storage paths
    pub attribute: Option<String>,

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

    // ── ADR-0025: First-class configuration fields ──
    /// First-class visibility tier (ADR-0025). Defaults to `Inline`.
    #[serde(default)]
    pub visibility: PropertyVisibility,
    /// First-class mutability (ADR-0025). Defaults to `Mutable`.
    #[serde(default)]
    pub mutability: PropertyMutability,
    /// Source of a derived property (ADR-0025). `None` for user-authored.
    #[serde(default)]
    pub derived_from: Option<DerivedSource>,
    /// Merge policy for property patches (ADR-0025). Defaults to `SetIfMissing`.
    #[serde(default)]
    pub merge_policy: MergePolicy,
}

impl PropertyDefinition {
    /// Map `PropertyVisibility` to the legacy `view_context` SQL column string.
    ///
    /// Used by SQL bind sites to derive the `view_context` column value from
    /// the first-class `visibility` field (ADR-0025). This is the single
    /// authoritative mapping for all bind sites.
    ///
    /// | `PropertyVisibility` | SQL column value |
    /// |---------------------|------------------|
    /// | `Inline`            | `"inline"`        |
    /// | `Panel`             | `"panel"`         |
    /// | `System`            | `"never"`         |
    /// | `Hidden`            | `"hidden"`        |
    #[must_use]
    pub const fn visibility_to_sql_column(v: PropertyVisibility) -> &'static str {
        match v {
            PropertyVisibility::Inline => "inline",
            PropertyVisibility::Panel => "panel",
            PropertyVisibility::System => "never",
            PropertyVisibility::Hidden => "hidden",
        }
    }

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
            attribute: None,
            status: PropertyStatus::default(),
            alias_of: None,
            block_count: 0,
            page_count: 0,
            first_seen_at: None,
            last_seen_at: None,
            // ADR-0025: first-class configuration fields
            visibility: PropertyVisibility::default(),
            mutability: PropertyMutability::default(),
            derived_from: None,
            merge_policy: MergePolicy::default(),
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
        if !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
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

    /// Set the first-class visibility tier (ADR-0025).
    #[must_use]
    pub fn with_visibility(mut self, visibility: PropertyVisibility) -> Self {
        self.visibility = visibility;
        self
    }

    /// Set attribute
    pub fn with_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attribute = Some(attribute.into());
        self
    }

    // ── ADR-0025: First-class configuration builders ──

    /// Set the first-class mutability tier (ADR-0025).
    ///
    /// Immutable properties are read-only in the UI; changes come from
    /// system rules, importers, or privileged operations.
    #[must_use]
    pub fn with_mutability(mut self, mutability: PropertyMutability) -> Self {
        self.mutability = mutability;
        self
    }

    /// Set the derived-source provenance (ADR-0025).
    ///
    /// Setting a derived source also sets `mutability = Immutable` as a
    /// side-effect, per the ADR-0025 invariant
    /// `derived_from.is_some() ⇒ mutability == Immutable`.
    #[must_use]
    pub fn with_derived_from(mut self, source: DerivedSource) -> Self {
        self.derived_from = Some(source);
        self.mutability = PropertyMutability::Immutable;
        self
    }

    /// Set the merge policy for property patches (ADR-0025).
    #[must_use]
    pub fn with_merge_policy(mut self, policy: MergePolicy) -> Self {
        self.merge_policy = policy;
        self
    }

    // ── ADR-0025: Derived getters and invariants ──

    /// Returns `true` if this property is queryable (searchable).
    ///
    /// Per ADR-0025: `Hidden` properties ARE still queryable (they hide from
    /// UI but are searchable). `System` properties are NOT queryable
    /// (they are the non-searchable tier).
    ///
    /// Defined as `self.visibility != PropertyVisibility::System`.
    pub fn is_queryable(&self) -> bool {
        self.visibility != PropertyVisibility::System
    }

    /// Assert that the domain invariants hold for this property definition.
    ///
    /// Currently enforces:
    /// - `derived_from.is_some() ⇒ mutability == Immutable` (ADR-0025)
    ///
    /// Returns `Ok(())` if invariants hold, or `Err(DomainError::InvariantViolation)`
    /// describing which invariant was violated.
    pub fn assert_invariants(&self) -> Result<(), DomainError> {
        if let Some(ref _source) = self.derived_from {
            if self.mutability != PropertyMutability::Immutable {
                return Err(DomainError::InvariantViolation(
                    "derived property must be immutable",
                ));
            }
        }
        Ok(())
    }

    /// Construct a `PropertyDefinition` from the legacy field set.
    ///
    /// This is the infallible migration path for loading existing serialized
    /// data. The invariant check (`assert_invariants`) is a **separate**
    /// surface — callers who bypass builders (e.g., struct literals or
    /// row mappers) should call it separately.
    ///
    /// Visibility derivation:
    /// - `hidden = true` → `PropertyVisibility::Hidden` (dominant, regardless of `view_context`)
    /// - else `view_context = Block` → `PropertyVisibility::Inline`
    /// - else `view_context = Page` → `PropertyVisibility::Panel`
    /// - else `view_context = Never` → `PropertyVisibility::System`
    #[must_use]
    pub fn from_legacy_fields(
        id: Uuid,
        db_ident: impl Into<String>,
        title: impl Into<String>,
        property_type: PropertyType,
        _view_context: ViewContext,
        _public: bool,
        _queryable: bool,
        _hidden: bool,
        _read_only: bool,
    ) -> Self {
        // Derive visibility from view_context (per spec):
        // hidden=true → Hidden (dominant), else Block → Inline, Page → Panel, Never → System
        let visibility = if _hidden {
            PropertyVisibility::Hidden
        } else {
            PropertyVisibility::from_view_context(&_view_context)
        };

        Self {
            id,
            db_ident: db_ident.into(),
            title: title.into(),
            property_type,
            cardinality: Cardinality::One,
            closed_values: Vec::new(),
            attribute: None,
            status: PropertyStatus::default(),
            alias_of: None,
            block_count: 0,
            page_count: 0,
            first_seen_at: None,
            last_seen_at: None,
            // ADR-0025: first-class fields derived from legacy inputs
            visibility,
            mutability: PropertyMutability::from_read_only(_read_only),
            derived_from: None,
            merge_policy: MergePolicy::SetIfMissing,
        }
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

    // ── visibility_to_sql_column tests (T1) ───────────────────────────────────

    #[test]
    fn visibility_to_sql_column_inline() {
        assert_eq!(
            PropertyDefinition::visibility_to_sql_column(PropertyVisibility::Inline),
            "inline"
        );
    }

    #[test]
    fn visibility_to_sql_column_panel() {
        assert_eq!(
            PropertyDefinition::visibility_to_sql_column(PropertyVisibility::Panel),
            "panel"
        );
    }

    #[test]
    fn visibility_to_sql_column_system() {
        assert_eq!(
            PropertyDefinition::visibility_to_sql_column(PropertyVisibility::System),
            "never"
        );
    }

    #[test]
    fn visibility_to_sql_column_hidden() {
        assert_eq!(
            PropertyDefinition::visibility_to_sql_column(PropertyVisibility::Hidden),
            "hidden"
        );
    }

    #[test]
    fn test_property_definition_builder() {
        let id = Uuid::new_v4();
        let prop = PropertyDefinition::new(id, "status", "Status", PropertyType::Text)
            .with_cardinality(Cardinality::One)
            .with_visibility(PropertyVisibility::Panel);

        assert_eq!(prop.db_ident, "status");
        assert_eq!(prop.title, "Status");
        assert_eq!(prop.property_type, PropertyType::Text);
        assert_eq!(prop.cardinality, Cardinality::One);
        assert_eq!(prop.visibility, PropertyVisibility::Panel);
        // ADR-0025: mutability defaults to Mutable
        assert_eq!(prop.mutability, PropertyMutability::Mutable);
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
        let prop = PropertyDefinition::new(
            Uuid::new_v4(),
            "old_status",
            "Old Status",
            PropertyType::Text,
        )
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
        let prop =
            PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text).with_usage(42, 7);
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

    // ── ADR-0025: T3 — new field defaults ──

    #[test]
    fn new_definition_defaults_to_inline_visibility() {
        // New PropertyDefinition has Inline visibility (per spec)
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert_eq!(def.visibility, PropertyVisibility::Inline);
    }

    #[test]
    fn new_definition_defaults_to_mutable() {
        // New PropertyDefinition is Mutable (per spec)
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert_eq!(def.mutability, PropertyMutability::Mutable);
    }

    #[test]
    fn new_definition_defaults_to_no_derived_from() {
        // New PropertyDefinition has derived_from = None (per spec)
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert!(def.derived_from.is_none());
    }

    #[test]
    fn new_definition_defaults_to_set_if_missing() {
        // New PropertyDefinition has SetIfMissing policy (per spec)
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert_eq!(def.merge_policy, MergePolicy::SetIfMissing);
    }

    // ── ADR-0025: T3 — is_queryable semantics ──

    #[test]
    fn is_queryable_for_inline_and_panel() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text);
        assert!(def.is_queryable()); // Inline → queryable
        let def2 = def.with_visibility(PropertyVisibility::Panel);
        assert!(def2.is_queryable()); // Panel → queryable
    }

    #[test]
    fn is_queryable_for_system_and_hidden() {
        let def_sys = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_visibility(PropertyVisibility::System);
        assert!(!def_sys.is_queryable()); // System → NOT queryable

        let def_hidden = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_visibility(PropertyVisibility::Hidden);
        // ADR-0025: Hidden IS still queryable (persisted but not in default UI)
        assert!(def_hidden.is_queryable());
    }

    // ── ADR-0025: T4 — new builders ──

    #[test]
    fn with_visibility_sets_visibility() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_visibility(PropertyVisibility::Panel);
        assert_eq!(def.visibility, PropertyVisibility::Panel);
    }

    #[test]
    fn with_mutability_sets_mutability() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_mutability(PropertyMutability::Immutable);
        assert_eq!(def.mutability, PropertyMutability::Immutable);
    }

    #[test]
    fn with_derived_from_sets_source_and_mutability() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_derived_from(DerivedSource::BlockContent);
        assert_eq!(def.derived_from, Some(DerivedSource::BlockContent));
        // Side-effect: mutability becomes Immutable
        assert_eq!(def.mutability, PropertyMutability::Immutable);
    }

    #[test]
    fn with_merge_policy_sets_policy() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_merge_policy(MergePolicy::Overwrite);
        assert_eq!(def.merge_policy, MergePolicy::Overwrite);
    }

    #[test]
    fn builders_are_chainable() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_visibility(PropertyVisibility::Panel)
            .with_mutability(PropertyMutability::Immutable)
            .with_derived_from(DerivedSource::Canonicalization)
            .with_merge_policy(MergePolicy::Union);
        assert_eq!(def.visibility, PropertyVisibility::Panel);
        assert_eq!(def.mutability, PropertyMutability::Immutable);
        assert_eq!(def.derived_from, Some(DerivedSource::Canonicalization));
        assert_eq!(def.merge_policy, MergePolicy::Union);
    }

    #[test]
    fn with_derived_from_after_immutable_is_idempotent() {
        // Setting derived_from after Immutable is idempotent for mutability
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_mutability(PropertyMutability::Immutable)
            .with_derived_from(DerivedSource::Importer);
        assert_eq!(def.mutability, PropertyMutability::Immutable);
        assert_eq!(def.derived_from, Some(DerivedSource::Importer));
    }

    // ── ADR-0025: T4 — assert_invariants ──

    #[test]
    fn assert_invariants_passes_for_non_derived() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_mutability(PropertyMutability::Mutable);
        assert!(def.assert_invariants().is_ok());
    }

    #[test]
    fn assert_invariants_passes_for_derived_and_immutable() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_derived_from(DerivedSource::Markdown);
        assert!(def.assert_invariants().is_ok());
    }

    #[test]
    fn assert_invariants_fails_for_derived_and_mutable() {
        // Direct struct construction with derived_from + Mutable should fail the invariant
        let def = PropertyDefinition {
            derived_from: Some(DerivedSource::Markdown),
            mutability: PropertyMutability::Mutable,
            ..PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
        };
        let result = def.assert_invariants();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::InvariantViolation(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("derived property must be immutable"));
    }

    // ── ADR-0025: T4 — from_legacy_fields ──

    #[test]
    fn from_legacy_fields_page_maps_to_panel() {
        let def = PropertyDefinition::from_legacy_fields(
            Uuid::new_v4(),
            "x",
            "X",
            PropertyType::Text,
            ViewContext::Page,
            true,
            true,
            false,
            false,
        );
        assert_eq!(def.visibility, PropertyVisibility::Panel);
    }

    #[test]
    fn from_legacy_fields_never_hidden_maps_to_hidden() {
        // Never + hidden=true → Hidden (dominant signal)
        let def = PropertyDefinition::from_legacy_fields(
            Uuid::new_v4(),
            "x",
            "X",
            PropertyType::Text,
            ViewContext::Never,
            false,
            false,
            true,
            true,
        );
        assert_eq!(def.visibility, PropertyVisibility::Hidden);
    }

    #[test]
    fn from_legacy_fields_never_invents_derived_from() {
        // from_legacy_fields never sets derived_from (always None)
        for (vc, pub_, hidden, ro) in [
            (ViewContext::Block, true, false, false),
            (ViewContext::Page, true, false, false),
            (ViewContext::Never, false, false, true),
            (ViewContext::Never, false, true, true),
        ] {
            let def = PropertyDefinition::from_legacy_fields(
                Uuid::new_v4(),
                "x",
                "X",
                PropertyType::Text,
                vc,
                pub_,
                false,
                hidden,
                ro,
            );
            assert!(
                def.derived_from.is_none(),
                "from_legacy_fields should never set derived_from"
            );
        }
    }

    // ── ADR-0025: T5 — from_legacy_fields 12-combo derivation ─────────────────

    #[test]
    fn from_legacy_fields_12_combo_derivation() {
        // 3 ViewContext × 2 hidden × 2 read_only = 12 combos
        // Derivation rules:
        //   visibility: hidden=true → Hidden; else Block→Inline, Page→Panel, Never→System
        //   mutability: read_only=true → Immutable; read_only=false → Mutable
        //   derived_from: always None
        //   merge_policy: always SetIfMissing
        //   closed_values: always Vec::new()
        let combos: Vec<(ViewContext, bool, bool)> = vec![
            // (view_context, hidden, read_only)
            (ViewContext::Block, false, false),
            (ViewContext::Block, false, true),
            (ViewContext::Block, true, false),
            (ViewContext::Block, true, true),
            (ViewContext::Page, false, false),
            (ViewContext::Page, false, true),
            (ViewContext::Page, true, false),
            (ViewContext::Page, true, true),
            (ViewContext::Never, false, false),
            (ViewContext::Never, false, true),
            (ViewContext::Never, true, false),
            (ViewContext::Never, true, true),
        ];

        for (vc, hidden, read_only) in combos {
            // Compute expected values BEFORE moving `vc` into from_legacy_fields
            let expected_visibility = if hidden {
                PropertyVisibility::Hidden
            } else {
                PropertyVisibility::from_view_context(&vc)
            };
            let expected_mutability = PropertyMutability::from_read_only(read_only);

            let def = PropertyDefinition::from_legacy_fields(
                Uuid::new_v4(),
                "x",
                "X",
                PropertyType::Text,
                vc.clone(), // ViewContext is not Copy, so clone needed
                false,      // public (unused in new model)
                false,      // queryable (unused in new model)
                hidden,
                read_only,
            );
            assert_eq!(
                def.visibility, expected_visibility,
                "view_context={:?}, hidden={}, read_only={} → visibility {:?} (expected {:?})",
                vc, hidden, read_only, def.visibility, expected_visibility
            );

            assert_eq!(
                def.mutability, expected_mutability,
                "view_context={:?}, hidden={}, read_only={} → mutability {:?} (expected {:?})",
                vc, hidden, read_only, def.mutability, expected_mutability
            );

            // Invariants
            assert!(
                def.derived_from.is_none(),
                "derived_from must always be None"
            );
            assert_eq!(
                def.merge_policy, MergePolicy::SetIfMissing,
                "merge_policy must always be SetIfMissing"
            );
            assert!(
                def.closed_values.is_empty(),
                "closed_values must always be empty"
            );
        }
    }

    // ── ADR-0025: T4 — mutability builder ──

    #[test]
    fn with_mutability_immutable_sets_flag() {
        // with_mutability(PropertyMutability::Immutable) directly sets the flag
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_mutability(PropertyMutability::Immutable);
        assert_eq!(def.mutability, PropertyMutability::Immutable);
        assert!(def.mutability.to_read_only());
    }

    // ── ADR-0025: T5 — backward-compat serde ──

    #[test]
    fn legacy_json_without_new_fields_defaults_all_four() {
        // Pre-ADR-0025 JSON (with legacy fields) still deserializes cleanly
        // because serde ignores unknown fields. The ADR-0025 fields default.
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
        assert_eq!(def.visibility, PropertyVisibility::Inline); // default
        assert_eq!(def.mutability, PropertyMutability::Mutable); // default
        assert!(def.derived_from.is_none()); // default
        assert_eq!(def.merge_policy, MergePolicy::SetIfMissing); // default
    }

    #[test]
    fn modern_json_round_trips_losslessly() {
        let def = PropertyDefinition::new(Uuid::new_v4(), "x", "X", PropertyType::Text)
            .with_visibility(PropertyVisibility::Panel)
            .with_mutability(PropertyMutability::Immutable)
            .with_derived_from(DerivedSource::BlockContent)
            .with_merge_policy(MergePolicy::Overwrite);

        let json = serde_json::to_string(&def).unwrap();
        let restored: PropertyDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(def.visibility, restored.visibility);
        assert_eq!(def.mutability, restored.mutability);
        assert_eq!(def.derived_from, restored.derived_from);
        assert_eq!(def.merge_policy, restored.merge_policy);
    }
}

// ── ADR-0025: T10 — proptest for serde round-trip ──────────────────────────

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    // Implement Arbitrary for PropertyVisibility via prop_oneof
    impl Arbitrary for PropertyVisibility {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(PropertyVisibility::Inline),
                Just(PropertyVisibility::Panel),
                Just(PropertyVisibility::System),
                Just(PropertyVisibility::Hidden),
            ]
            .boxed()
        }
    }

    // Implement Arbitrary for PropertyMutability
    impl Arbitrary for PropertyMutability {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(PropertyMutability::Mutable),
                Just(PropertyMutability::Immutable),
            ]
            .boxed()
        }
    }

    // Implement Arbitrary for MergePolicy
    impl Arbitrary for MergePolicy {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(MergePolicy::SetIfMissing),
                Just(MergePolicy::Overwrite),
                Just(MergePolicy::Append),
                Just(MergePolicy::Union),
                Just(MergePolicy::RejectOnConflict),
                Just(MergePolicy::AskOnConflict),
            ]
            .boxed()
        }
    }

    // Implement Arbitrary for DerivedSource
    impl Arbitrary for DerivedSource {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(DerivedSource::BlockContent),
                Just(DerivedSource::Markdown),
                Just(DerivedSource::Canonicalization),
                Just(DerivedSource::Importer),
                ".*".prop_map(DerivedSource::OtherProperty),
            ]
            .boxed()
        }
    }

    // Implement Arbitrary for PropertyType
    impl Arbitrary for PropertyType {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(PropertyType::Text),
                Just(PropertyType::Number),
                Just(PropertyType::Date),
                Just(PropertyType::DateTime),
                Just(PropertyType::Url),
                Just(PropertyType::Checkbox),
                Just(PropertyType::Node),
            ]
            .boxed()
        }
    }

    // Implement Arbitrary for Cardinality
    impl Arbitrary for Cardinality {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![Just(Cardinality::One), Just(Cardinality::Many),].boxed()
        }
    }

    // Implement Arbitrary for PropertyStatus
    impl Arbitrary for PropertyStatus {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![
                Just(PropertyStatus::Active),
                Just(PropertyStatus::Deprecated),
                Just(PropertyStatus::Merged),
                Just(PropertyStatus::Alias),
            ]
            .boxed()
        }
    }

    proptest! {
        #[test]
        fn property_definition_serde_roundtrip_is无损(def in arbitrary_property_definition()) {
            // Serde round-trip must preserve all fields
            let json = serde_json::to_string(&def).unwrap();
            let restored: PropertyDefinition = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(def.visibility, restored.visibility);
            prop_assert_eq!(def.mutability, restored.mutability);
            prop_assert_eq!(def.derived_from, restored.derived_from);
            prop_assert_eq!(def.merge_policy, restored.merge_policy);
        }

        #[test]
        fn builder_constructed_def_always_satisfies_invariant(
            visibility in any::<PropertyVisibility>(),
            mutability in any::<PropertyMutability>(),
            merge_policy in any::<MergePolicy>(),
            source in any::<Option<DerivedSource>>()
        ) {
            // A PropertyDefinition built via new() + the 4 new builders
            // must always satisfy the ADR-0025 invariant
            let mut def = PropertyDefinition::new(
                crate::value_objects::Uuid::new_v4(),
                "test_prop",
                "Test Prop",
                PropertyType::Text,
            )
            .with_visibility(visibility)
            .with_mutability(mutability)
            .with_merge_policy(merge_policy);

            if let Some(src) = source {
                def = def.with_derived_from(src);
            }

            prop_assert!(def.assert_invariants().is_ok());
        }
    }

    // Arbitrary instance generator for PropertyDefinition covering all new enum variants
    fn arbitrary_property_definition() -> impl Strategy<Value = PropertyDefinition> {
        (
            any::<PropertyVisibility>(),
            any::<PropertyMutability>(),
            any::<Option<DerivedSource>>(),
            any::<MergePolicy>(),
            any::<PropertyType>(),
            any::<Cardinality>(),
            any::<PropertyStatus>(),
        )
            .prop_map(
                |(
                    visibility,
                    mutability,
                    derived_from,
                    merge_policy,
                    property_type,
                    cardinality,
                    status,
                )| {
                    let mut def = PropertyDefinition::new(
                        crate::value_objects::Uuid::new_v4(),
                        "test_prop",
                        "Test Prop",
                        property_type,
                    )
                    .with_visibility(visibility)
                    .with_mutability(mutability)
                    .with_merge_policy(merge_policy)
                    .with_cardinality(cardinality)
                    .with_status(status);

                    if let Some(src) = derived_from {
                        def = def.with_derived_from(src);
                    }

                    def
                },
            )
    }
}
