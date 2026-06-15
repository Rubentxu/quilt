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

// ─────────────────────────────────────────────────────────────────────────────
// PI-2: Property Lifecycle
// ─────────────────────────────────────────────────────────────────────────────

/// Lifecycle status of a property definition.
///
/// Properties follow a "grow, never break" philosophy — they are never
/// destructively deleted, only deprecated or merged.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum PropertyStatus {
    /// Active and in use
    #[default]
    Active,
    /// Deprecated — still readable but writes should warn
    Deprecated,
    /// Merged into another property — reads redirect to the target
    Merged,
    /// Alias — transparent redirect to another property key
    Alias,
}

impl PropertyStatus {
    /// Canonical string representation for storage
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyStatus::Active => "active",
            PropertyStatus::Deprecated => "deprecated",
            PropertyStatus::Merged => "merged",
            PropertyStatus::Alias => "alias",
        }
    }

    /// Parse from canonical string
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(PropertyStatus::Active),
            "deprecated" => Some(PropertyStatus::Deprecated),
            "merged" => Some(PropertyStatus::Merged),
            "alias" => Some(PropertyStatus::Alias),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyStatus {
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

// ─────────────────────────────────────────────────────────────────────────────
// Property Visibility — ADR-0025 four-tier model
// ─────────────────────────────────────────────────────────────────────────────

/// Where and how a property is visible in the UI — ADR-0025 four-tier model.
///
/// Replaces the legacy `view_context: ViewContext` plus `public: bool` and
/// `hidden: bool` booleans. `Hidden` hides from default UI surfaces but is
/// still persisted and queryable. `System` is the non-searchable tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyVisibility {
    /// Visible inline with block content (replaces `ViewContext::Block` + `public: true`).
    /// This is the default variant.
    Inline,
    /// Visible in the page/block properties panel (replaces `ViewContext::Page`).
    Panel,
    /// System/debug surface only — never shown in user content, not searchable.
    /// Replaces `ViewContext::Never` with `public: false`.
    System,
    /// Hidden from default UI surfaces but still persisted and queryable.
    /// Replaces `hidden: true` (dominant signal over `public`).
    Hidden,
}

impl Default for PropertyVisibility {
    fn default() -> Self {
        PropertyVisibility::Inline
    }
}

impl PropertyVisibility {
    /// Convert legacy `ViewContext` to `PropertyVisibility`.
    ///
    /// Block → Inline, Page → Panel, Never → System.
    /// This is the migration path for existing serialized definitions.
    #[must_use]
    pub fn from_view_context(vc: &ViewContext) -> Self {
        match vc {
            ViewContext::Block => PropertyVisibility::Inline,
            ViewContext::Page => PropertyVisibility::Panel,
            ViewContext::Never => PropertyVisibility::System,
        }
    }

    /// Canonical string representation for storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyVisibility::Inline => "inline",
            PropertyVisibility::Panel => "panel",
            PropertyVisibility::System => "system",
            PropertyVisibility::Hidden => "hidden",
        }
    }

    /// Parse from canonical string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "inline" => Some(PropertyVisibility::Inline),
            "panel" => Some(PropertyVisibility::Panel),
            "system" => Some(PropertyVisibility::System),
            "hidden" => Some(PropertyVisibility::Hidden),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property Mutability — Mutable vs Immutable (ADR-0025)
// ─────────────────────────────────────────────────────────────────────────────

/// Whether a property can be edited by the user from the UI.
///
/// Replaces the legacy `read_only: bool` flag. `Immutable` properties are
/// changed only by system rules, importers, or privileged operations.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum PropertyMutability {
    /// User-editable from the UI (default).
    #[default]
    Mutable,
    /// Read-only in the UI; changes come from system rules, importers,
    /// or privileged operations only.
    Immutable,
}

impl PropertyMutability {
    /// Convert from legacy `read_only` bool.
    ///
    /// false → Mutable, true → Immutable.
    #[must_use]
    pub fn from_read_only(read_only: bool) -> Self {
        if read_only {
            PropertyMutability::Immutable
        } else {
            PropertyMutability::Mutable
        }
    }

    /// Convert back to legacy `read_only` bool (lossless round-trip).
    #[must_use]
    pub fn to_read_only(self) -> bool {
        self == PropertyMutability::Immutable
    }

    /// Canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyMutability::Mutable => "mutable",
            PropertyMutability::Immutable => "immutable",
        }
    }

    /// Parse from canonical string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mutable" => Some(PropertyMutability::Mutable),
            "immutable" => Some(PropertyMutability::Immutable),
            _ => None,
        }
    }
}

impl fmt::Display for PropertyMutability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Derived Source — provenance tracking (ADR-0025)
// ─────────────────────────────────────────────────────────────────────────────

/// The source from which a derived property originates.
///
/// Carries provenance for properties whose values are computed rather than
/// user-authored. The invariant `derived_from.is_some() ⇒ mutability == Immutable`
/// is enforced by `PropertyDefinition::assert_invariants()`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DerivedSource {
    /// Derived from a syntactic feature of the block text (e.g., heading level
    /// from `# Title`, link URL from `[Text](url)`).
    #[serde(rename = "block_content")]
    BlockContent,
    /// Derived from generic Markdown canonicalization (list detection, quote
    /// detection, etc.).
    #[serde(rename = "markdown")]
    Markdown,
    /// Derived by the canonicalization pipeline as a whole (normalization,
    /// deduplication, key folding).
    #[serde(rename = "canonicalization")]
    Canonicalization,
    /// Derived by a data import adapter (Logseq DB import, Markdown bulk import).
    #[serde(rename = "importer")]
    Importer,
    /// Derived from another property (the string is the `db_ident` of the
    /// source property). Used for derived-of-derived chains.
    #[serde(rename = "other_property")]
    OtherProperty(String),
}

impl DerivedSource {
    /// Canonical string representation for the unit variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            DerivedSource::BlockContent => "block_content",
            DerivedSource::Markdown => "markdown",
            DerivedSource::Canonicalization => "canonicalization",
            DerivedSource::Importer => "importer",
            DerivedSource::OtherProperty(_) => "other_property",
        }
    }
}

impl fmt::Display for DerivedSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DerivedSource::OtherProperty(name) => write!(f, "other_property({})", name),
            other => write!(f, "{}", other.as_str()),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Merge Policy — per-property merge strategy (ADR-0025 V1)
// ─────────────────────────────────────────────────────────────────────────────

/// How a property patch combines with an existing value.
///
/// Each property declares its merge policy for patches arriving via slash
/// commands, importers, MCP requests, or CRDT merge. Metadata only in
/// this slice — actual enforcement is in the canonicalization pipeline.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MergePolicy {
    /// Write only if no value exists; keep existing otherwise.
    #[default]
    SetIfMissing,
    /// Unconditionally replace the existing value.
    Overwrite,
    /// Append the new value to an existing multi-value collection.
    Append,
    /// Compute the set union of existing and new values (multi-value props).
    Union,
    /// If values differ, reject the patch with a domain error.
    RejectOnConflict,
    /// If values differ, surface a confirmation prompt before applying.
    AskOnConflict,
}

impl MergePolicy {
    /// Canonical string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            MergePolicy::SetIfMissing => "set_if_missing",
            MergePolicy::Overwrite => "overwrite",
            MergePolicy::Append => "append",
            MergePolicy::Union => "union",
            MergePolicy::RejectOnConflict => "reject_on_conflict",
            MergePolicy::AskOnConflict => "ask_on_conflict",
        }
    }

    /// Parse from canonical string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "set_if_missing" => Some(MergePolicy::SetIfMissing),
            "overwrite" => Some(MergePolicy::Overwrite),
            "append" => Some(MergePolicy::Append),
            "union" => Some(MergePolicy::Union),
            "reject_on_conflict" => Some(MergePolicy::RejectOnConflict),
            "ask_on_conflict" => Some(MergePolicy::AskOnConflict),
            _ => None,
        }
    }

    /// ADR-0025 V1 merge policy table — maps known V1 property db_idents
    /// to their canonical merge policies.
    ///
    /// Does NOT list `content`, `text`, or `children` (those are never
    /// touched by a preset and have no merge policy).
    pub const ADR_0025_V1_TABLE: &[(&'static str, MergePolicy)] = &[
        ("type", MergePolicy::SetIfMissing),
        ("projection", MergePolicy::SetIfMissing),
        ("status", MergePolicy::Overwrite),
        ("focus", MergePolicy::Overwrite),
        ("tags", MergePolicy::Union),
        ("scheduled", MergePolicy::Overwrite),
        ("deadline", MergePolicy::Overwrite),
        ("media-type", MergePolicy::AskOnConflict),
        ("source-url", MergePolicy::AskOnConflict),
    ];
}

impl fmt::Display for MergePolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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

    // ── PropertyVisibility ─────────────────────────────────────────

    #[test]
    fn test_property_visibility_default_is_inline() {
        assert_eq!(PropertyVisibility::default(), PropertyVisibility::Inline);
    }

    #[test]
    fn test_property_visibility_serde_lowercase() {
        for variant in &[
            PropertyVisibility::Inline,
            PropertyVisibility::Panel,
            PropertyVisibility::System,
            PropertyVisibility::Hidden,
        ] {
            let json = serde_json::to_string(variant).unwrap();
            let round_trip: PropertyVisibility = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, round_trip);
        }
    }

    #[test]
    fn test_property_visibility_unknown_rejected() {
        let result: Result<PropertyVisibility, _> = serde_json::from_str(r#""invisible""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_property_visibility_from_view_context() {
        assert_eq!(
            PropertyVisibility::from_view_context(&ViewContext::Block),
            PropertyVisibility::Inline
        );
        assert_eq!(
            PropertyVisibility::from_view_context(&ViewContext::Page),
            PropertyVisibility::Panel
        );
        assert_eq!(
            PropertyVisibility::from_view_context(&ViewContext::Never),
            PropertyVisibility::System
        );
    }

    #[test]
    fn test_property_visibility_as_str_and_from_str() {
        assert_eq!(PropertyVisibility::Inline.as_str(), "inline");
        assert_eq!(PropertyVisibility::Panel.as_str(), "panel");
        assert_eq!(PropertyVisibility::System.as_str(), "system");
        assert_eq!(PropertyVisibility::Hidden.as_str(), "hidden");
        assert_eq!(
            PropertyVisibility::from_str("inline"),
            Some(PropertyVisibility::Inline)
        );
        assert_eq!(
            PropertyVisibility::from_str("panel"),
            Some(PropertyVisibility::Panel)
        );
        assert_eq!(
            PropertyVisibility::from_str("system"),
            Some(PropertyVisibility::System)
        );
        assert_eq!(
            PropertyVisibility::from_str("hidden"),
            Some(PropertyVisibility::Hidden)
        );
        assert_eq!(PropertyVisibility::from_str("unknown"), None);
    }

    // ── PropertyMutability ────────────────────────────────────────

    #[test]
    fn test_property_mutability_default_is_mutable() {
        assert_eq!(PropertyMutability::default(), PropertyMutability::Mutable);
    }

    #[test]
    fn test_property_mutability_serde_lowercase() {
        for variant in &[PropertyMutability::Mutable, PropertyMutability::Immutable] {
            let json = serde_json::to_string(variant).unwrap();
            let round_trip: PropertyMutability = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, round_trip);
        }
    }

    #[test]
    fn test_property_mutability_from_read_only() {
        assert_eq!(
            PropertyMutability::from_read_only(false),
            PropertyMutability::Mutable
        );
        assert_eq!(
            PropertyMutability::from_read_only(true),
            PropertyMutability::Immutable
        );
    }

    #[test]
    fn test_property_mutability_to_read_only() {
        assert!(!PropertyMutability::Mutable.to_read_only());
        assert!(PropertyMutability::Immutable.to_read_only());
    }

    #[test]
    fn test_property_mutability_roundtrip() {
        assert_eq!(PropertyMutability::Mutable.as_str(), "mutable");
        assert_eq!(PropertyMutability::Immutable.as_str(), "immutable");
        assert_eq!(
            PropertyMutability::from_str("mutable"),
            Some(PropertyMutability::Mutable)
        );
        assert_eq!(
            PropertyMutability::from_str("immutable"),
            Some(PropertyMutability::Immutable)
        );
        assert_eq!(PropertyMutability::from_str("readonly"), None);
    }

    // ── DerivedSource ─────────────────────────────────────────────

    #[test]
    fn test_derived_source_block_content_serde() {
        let ds = DerivedSource::BlockContent;
        let json = serde_json::to_string(&ds).unwrap();
        assert_eq!(json, r#""block_content""#);
        let round_trip: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, round_trip);
    }

    #[test]
    fn test_derived_source_markdown_serde() {
        let ds = DerivedSource::Markdown;
        let json = serde_json::to_string(&ds).unwrap();
        assert_eq!(json, r#""markdown""#);
        let round_trip: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, round_trip);
    }

    #[test]
    fn test_derived_source_canonicalization_serde() {
        let ds = DerivedSource::Canonicalization;
        let json = serde_json::to_string(&ds).unwrap();
        assert_eq!(json, r#""canonicalization""#);
        let round_trip: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, round_trip);
    }

    #[test]
    fn test_derived_source_importer_serde() {
        let ds = DerivedSource::Importer;
        let json = serde_json::to_string(&ds).unwrap();
        assert_eq!(json, r#""importer""#);
        let round_trip: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, round_trip);
    }

    #[test]
    fn test_derived_source_other_property_serde() {
        let ds = DerivedSource::OtherProperty("status".to_string());
        let json = serde_json::to_string(&ds).unwrap();
        let round_trip: DerivedSource = serde_json::from_str(&json).unwrap();
        assert_eq!(ds, round_trip);
    }

    #[test]
    fn test_derived_source_unknown_rejected() {
        // Plain strings that aren't a known variant must fail
        let result: Result<DerivedSource, _> = serde_json::from_str(r#""unknown_source""#);
        assert!(result.is_err());
    }

    // ── MergePolicy ───────────────────────────────────────────────

    #[test]
    fn test_merge_policy_default_is_set_if_missing() {
        assert_eq!(MergePolicy::default(), MergePolicy::SetIfMissing);
    }

    #[test]
    fn test_merge_policy_all_variants_serde_snake_case() {
        let variants = &[
            (MergePolicy::SetIfMissing, "set_if_missing"),
            (MergePolicy::Overwrite, "overwrite"),
            (MergePolicy::Append, "append"),
            (MergePolicy::Union, "union"),
            (MergePolicy::RejectOnConflict, "reject_on_conflict"),
            (MergePolicy::AskOnConflict, "ask_on_conflict"),
        ];
        for (variant, expected_str) in variants {
            let json = serde_json::to_string(variant).unwrap();
            assert_eq!(json, format!(r#""{}""#, expected_str));
            let round_trip: MergePolicy = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, round_trip);
        }
    }

    #[test]
    fn test_merge_policy_unknown_rejected() {
        let result: Result<MergePolicy, _> = serde_json::from_str(r#""last_write_wins""#);
        assert!(result.is_err());
    }

    #[test]
    fn test_merge_policy_as_str_and_from_str() {
        assert_eq!(MergePolicy::SetIfMissing.as_str(), "set_if_missing");
        assert_eq!(MergePolicy::Overwrite.as_str(), "overwrite");
        assert_eq!(
            MergePolicy::from_str("set_if_missing"),
            Some(MergePolicy::SetIfMissing)
        );
        assert_eq!(
            MergePolicy::from_str("overwrite"),
            Some(MergePolicy::Overwrite)
        );
        assert_eq!(MergePolicy::from_str("last_write_wins"), None);
    }

    #[test]
    fn test_merge_policy_adr_0025_v1_table_entries() {
        let table = MergePolicy::ADR_0025_V1_TABLE;
        let tags_policy = table.iter().find(|(k, _)| *k == "tags");
        assert_eq!(tags_policy, Some(&("tags", MergePolicy::Union)));

        let status_policy = table.iter().find(|(k, _)| *k == "status");
        assert_eq!(status_policy, Some(&("status", MergePolicy::Overwrite)));

        let projection_policy = table.iter().find(|(k, _)| *k == "projection");
        assert_eq!(
            projection_policy,
            Some(&("projection", MergePolicy::SetIfMissing))
        );

        let media_type_policy = table.iter().find(|(k, _)| *k == "media-type");
        assert_eq!(
            media_type_policy,
            Some(&("media-type", MergePolicy::AskOnConflict))
        );
    }

    #[test]
    fn test_merge_policy_v1_table_excludes_content_text_children() {
        let table = MergePolicy::ADR_0025_V1_TABLE;
        let keys: Vec<&str> = table.iter().map(|(k, _)| *k).collect();
        assert!(!keys.contains(&"content"));
        assert!(!keys.contains(&"text"));
        assert!(!keys.contains(&"children"));
    }
}
