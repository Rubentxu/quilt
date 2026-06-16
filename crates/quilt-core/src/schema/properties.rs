//! Property schema types and synchronous validation
//!
//! Pure types extracted from `quilt-domain::properties`:
//! - PropertyType, Cardinality, ViewContext, ClosedValue, PropertyDefinition
//! - Sync validation functions for type/cardinality/closed-set checking
//! - Builtin property definitions (status, priority, deadline, scheduled, url)

use ::serde::de::Visitor;
use serde::{Deserialize, Serialize};
use std::fmt;

// ── PropertyType ────────────────────────────────────────────────────

/// Data type of a property value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyType {
    Text,
    Number,
    Date,
    DateTime,
    Url,
    Checkbox,
    Node,
}

impl PropertyType {
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

// ── Cardinality ─────────────────────────────────────────────────────

/// Whether a property accepts single or multiple values.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cardinality {
    One,
    Many,
}

impl Cardinality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Cardinality::One => "one",
            Cardinality::Many => "many",
        }
    }

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

impl Default for Cardinality {
    fn default() -> Self {
        Cardinality::One
    }
}

// ── ViewContext ─────────────────────────────────────────────────────

/// Where a property is displayed in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewContext {
    Page,
    Block,
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

impl Default for ViewContext {
    fn default() -> Self {
        ViewContext::Block
    }
}

// ── ClosedValue ─────────────────────────────────────────────────────

/// A predefined option for a property with closed set semantics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedValue {
    pub id: uuid::Uuid,
    pub db_ident: String,
    pub value: String,
    pub icon: Option<String>,
    pub order: f64,
}

impl ClosedValue {
    pub fn new(id: uuid::Uuid, db_ident: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id,
            db_ident: db_ident.into(),
            value: value.into(),
            icon: None,
            order: 0.0,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_order(mut self, order: f64) -> Self {
        self.order = order;
        self
    }
}

// ── PropertyVisibility (ADR-0025) ──────────────────────────────────────────────

/// Where and how a property is visible in the UI — ADR-0025 four-tier model.
/// Duplicated from `quilt_domain::properties::types::PropertyVisibility`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyVisibility {
    Inline,
    Panel,
    System,
    Hidden,
}

impl Default for PropertyVisibility {
    fn default() -> Self {
        PropertyVisibility::Inline
    }
}

impl PropertyVisibility {
    /// Convert legacy `ViewContext` to `PropertyVisibility`.
    #[must_use]
    pub fn from_view_context(vc: &ViewContext) -> Self {
        match vc {
            ViewContext::Block => PropertyVisibility::Inline,
            ViewContext::Page => PropertyVisibility::Panel,
            ViewContext::Never => PropertyVisibility::System,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyVisibility::Inline => "inline",
            PropertyVisibility::Panel => "panel",
            PropertyVisibility::System => "system",
            PropertyVisibility::Hidden => "hidden",
        }
    }

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

// ── PropertyMutability (ADR-0025) ─────────────────────────────────────────────

/// Whether a property can be edited by the user from the UI.
/// Duplicated from `quilt_domain::properties::types::PropertyMutability`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyMutability {
    #[default]
    Mutable,
    Immutable,
}

impl PropertyMutability {
    #[must_use]
    pub fn from_read_only(read_only: bool) -> Self {
        if read_only {
            PropertyMutability::Immutable
        } else {
            PropertyMutability::Mutable
        }
    }

    #[must_use]
    pub fn to_read_only(self) -> bool {
        self == PropertyMutability::Immutable
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyMutability::Mutable => "mutable",
            PropertyMutability::Immutable => "immutable",
        }
    }

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

// ── DerivedSource (ADR-0025) ────────────────────────────────────────────────

/// Source of a derived property.
/// Duplicated from `quilt_domain::properties::types::DerivedSource`.
/// Serializes as a plain string for unit variants (e.g. `"block_content"`)
/// and as `{"type":"other_property","value":"..."}` for `OtherProperty`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedSource {
    BlockContent,
    Markdown,
    Canonicalization,
    Importer,
    OtherProperty(String),
}

impl Serialize for DerivedSource {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            DerivedSource::BlockContent => s.serialize_str("block_content"),
            DerivedSource::Markdown => s.serialize_str("markdown"),
            DerivedSource::Canonicalization => s.serialize_str("canonicalization"),
            DerivedSource::Importer => s.serialize_str("importer"),
            DerivedSource::OtherProperty(name) => {
                use serde::ser::SerializeStruct;
                let mut s = s.serialize_struct("DerivedSource", 2)?;
                s.serialize_field("type", "other_property")?;
                s.serialize_field("value", name)?;
                s.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for DerivedSource {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DerivedSourceVisitor;
        impl<'de> Visitor<'de> for DerivedSourceVisitor {
            type Value = DerivedSource;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a derived source string or object")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: ::serde::de::Error,
            {
                match v {
                    "block_content" => Ok(DerivedSource::BlockContent),
                    "markdown" => Ok(DerivedSource::Markdown),
                    "canonicalization" => Ok(DerivedSource::Canonicalization),
                    "importer" => Ok(DerivedSource::Importer),
                    _ => Err(::serde::de::Error::custom(format!(
                        "unknown derived source: {v}"
                    ))),
                }
            }
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: ::serde::de::MapAccess<'de>,
            {
                // {type: "other_property", value: "..."}
                let mut typ: Option<String> = None;
                let mut value: Option<String> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "type" => {
                            typ = Some(map.next_value()?);
                        }
                        "value" => {
                            value = Some(map.next_value()?);
                        }
                        _ => {}
                    }
                }
                if typ == Some("other_property".to_string()) {
                    if let Some(v) = value {
                        return Ok(DerivedSource::OtherProperty(v));
                    }
                }
                Err(::serde::de::Error::custom("invalid DerivedSource object"))
            }
        }
        d.deserialize_any(DerivedSourceVisitor)
    }
}

impl DerivedSource {
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

// ── MergePolicy (ADR-0025) ────────────────────────────────────────────────

/// How a property patch combines with an existing value.
/// Duplicated from `quilt_domain::properties::types::MergePolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergePolicy {
    #[default]
    SetIfMissing,
    Overwrite,
    Append,
    Union,
    RejectOnConflict,
    AskOnConflict,
}

impl MergePolicy {
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
    /// to their canonical merge policies. Does NOT list `content`, `text`,
    /// or `children` (those are never touched by a preset).
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

// ── PropertyDefinition ──────────────────────────────────────────────

/// Schema definition for a typed property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    pub id: uuid::Uuid,
    pub db_ident: String,
    pub title: String,
    pub property_type: PropertyType,
    pub cardinality: Cardinality,
    pub closed_values: Vec<ClosedValue>,
    pub attribute: Option<String>,
    // ADR-0025: First-class configuration fields
    #[serde(default)]
    pub visibility: PropertyVisibility,
    #[serde(default)]
    pub mutability: PropertyMutability,
    #[serde(default)]
    pub derived_from: Option<DerivedSource>,
    #[serde(default)]
    pub merge_policy: MergePolicy,
}

impl PropertyDefinition {
    pub fn new(
        id: uuid::Uuid,
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
            // ADR-0025: first-class fields default
            visibility: PropertyVisibility::default(),
            mutability: PropertyMutability::default(),
            derived_from: None,
            merge_policy: MergePolicy::default(),
        }
    }

    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

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

    pub fn with_attribute(mut self, attribute: impl Into<String>) -> Self {
        self.attribute = Some(attribute.into());
        self
    }

    // ── ADR-0025: First-class configuration builders ──

    /// Set the first-class mutability tier (ADR-0025).
    #[must_use]
    pub fn with_mutability(mut self, mutability: PropertyMutability) -> Self {
        self.mutability = mutability;
        self
    }

    /// Set the derived-source provenance (ADR-0025). Also sets mutability to Immutable.
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

    // ── ADR-0025: Derived getters ──

    /// Returns `true` if this property is queryable (searchable).
    /// `Hidden` IS queryable; `System` is NOT.
    pub fn is_queryable(&self) -> bool {
        self.visibility != PropertyVisibility::System
    }

    // ── from_legacy_fields ──

    /// Construct from the legacy field set (infallible).
    #[must_use]
    pub fn from_legacy_fields(
        id: uuid::Uuid,
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
            visibility,
            mutability: PropertyMutability::from_read_only(_read_only),
            derived_from: None,
            merge_policy: MergePolicy::SetIfMissing,
        }
    }

    /// Check if this property has a closed set of allowed values.
    pub fn has_closed_values(&self) -> bool {
        !self.closed_values.is_empty()
    }

    /// Check if a value is in the closed set (matches by value or db_ident).
    pub fn is_value_allowed(&self, value: &str) -> bool {
        if self.closed_values.is_empty() {
            true
        } else {
            self.closed_values
                .iter()
                .any(|cv| cv.value == value || cv.db_ident == value)
        }
    }
}

// ── Sync Validation ─────────────────────────────────────────────────

type ValidationResult = Result<(), String>;

/// Get a human-readable name for a JSON value type.
fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Validate that a JSON value type matches the property type.
pub fn validate_type(def: &PropertyDefinition, value: &serde_json::Value) -> ValidationResult {
    let compatible = match (&def.property_type, value) {
        (PropertyType::Text, serde_json::Value::String(_)) => true,
        (PropertyType::Number, serde_json::Value::Number(_)) => true,
        (PropertyType::Date, serde_json::Value::String(_)) => true,
        (PropertyType::DateTime, serde_json::Value::String(_)) => true,
        (PropertyType::Url, serde_json::Value::String(_)) => true,
        (PropertyType::Checkbox, serde_json::Value::Bool(_)) => true,
        (PropertyType::Node, serde_json::Value::String(_)) => true,
        _ => false,
    };

    if !compatible {
        return Err(format!(
            "Type mismatch: expected {} but got {}",
            def.property_type.as_str(),
            json_type_name(value)
        ));
    }

    Ok(())
}

/// Validate cardinality constraint (single vs array).
pub fn validate_cardinality(
    def: &PropertyDefinition,
    value: &serde_json::Value,
) -> ValidationResult {
    match (&def.cardinality, value) {
        (Cardinality::One, serde_json::Value::Array(_)) => Err(format!(
            "Cardinality violation: expected single value but got array (cardinality is {})",
            def.cardinality.as_str()
        )),
        _ => Ok(()),
    }
}

/// Validate that a value belongs to the closed set (if defined).
pub fn validate_closed_set(
    def: &PropertyDefinition,
    value: &serde_json::Value,
) -> ValidationResult {
    if def.closed_values.is_empty() {
        return Ok(());
    }

    let value_str = match value {
        serde_json::Value::String(s) => Some(s.as_str()),
        _ => None,
    };

    if let Some(value_str) = value_str {
        if !def.is_value_allowed(value_str) {
            let allowed: Vec<&str> = def
                .closed_values
                .iter()
                .map(|cv| cv.value.as_str())
                .collect();
            return Err(format!(
                "Value '{}' is not in the closed set: {:?}",
                value_str, allowed
            ));
        }
    }

    Ok(())
}

/// Validate a value against a property definition (type + cardinality + closed set).
pub fn validate_property(def: &PropertyDefinition, value: &serde_json::Value) -> ValidationResult {
    validate_type(def, value)?;
    validate_cardinality(def, value)?;
    validate_closed_set(def, value)?;
    Ok(())
}

// ── Builtin Properties ──────────────────────────────────────────────

/// Get all builtin property definitions.
pub fn builtin_properties() -> Vec<PropertyDefinition> {
    let mut props = Vec::new();

    // status: closed set TODO, DOING, DONE, LATER, CANCELLED
    let status_closed = vec![
        ClosedValue::new(
            make_uuid("a1000001-0000-0000-0000-000000000001"),
            "todo",
            "To Do",
        )
        .with_icon("📋")
        .with_order(1.0),
        ClosedValue::new(
            make_uuid("a1000001-0000-0000-0000-000000000002"),
            "doing",
            "Doing",
        )
        .with_icon("🏃")
        .with_order(2.0),
        ClosedValue::new(
            make_uuid("a1000001-0000-0000-0000-000000000003"),
            "done",
            "Done",
        )
        .with_icon("✅")
        .with_order(3.0),
        ClosedValue::new(
            make_uuid("a1000001-0000-0000-0000-000000000004"),
            "later",
            "Later",
        )
        .with_icon("⏰")
        .with_order(4.0),
        ClosedValue::new(
            make_uuid("a1000001-0000-0000-0000-000000000005"),
            "cancelled",
            "Cancelled",
        )
        .with_icon("❌")
        .with_order(5.0),
    ];
    props.push(
        PropertyDefinition::new(
            make_uuid("a1000001-0000-0000-0000-000000000000"),
            "quilt.property/status",
            "Status",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_closed_values(status_closed)
        .with_visibility(PropertyVisibility::Panel),
    );

    // priority: closed set A, B, C
    let priority_closed = vec![
        ClosedValue::new(make_uuid("a1000002-0000-0000-0000-000000000001"), "a", "A")
            .with_icon("🔴")
            .with_order(1.0),
        ClosedValue::new(make_uuid("a1000002-0000-0000-0000-000000000002"), "b", "B")
            .with_icon("🟡")
            .with_order(2.0),
        ClosedValue::new(make_uuid("a1000002-0000-0000-0000-000000000003"), "c", "C")
            .with_icon("🟢")
            .with_order(3.0),
    ];
    props.push(
        PropertyDefinition::new(
            make_uuid("a1000002-0000-0000-0000-000000000000"),
            "quilt.property/priority",
            "Priority",
            PropertyType::Text,
        )
        .with_cardinality(Cardinality::One)
        .with_closed_values(priority_closed)
        .with_visibility(PropertyVisibility::Panel),
    );

    // deadline (Date type)
    props.push(
        PropertyDefinition::new(
            make_uuid("a1000003-0000-0000-0000-000000000000"),
            "quilt.property/deadline",
            "Deadline",
            PropertyType::Date,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel),
    );

    // scheduled (Date type)
    props.push(
        PropertyDefinition::new(
            make_uuid("a1000004-0000-0000-0000-000000000000"),
            "quilt.property/scheduled",
            "Scheduled",
            PropertyType::Date,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel),
    );

    // url (Url type)
    props.push(
        PropertyDefinition::new(
            make_uuid("a1000005-0000-0000-0000-000000000000"),
            "quilt.property/url",
            "URL",
            PropertyType::Url,
        )
        .with_cardinality(Cardinality::One)
        .with_visibility(PropertyVisibility::Panel),
    );

    props
}

/// Get a builtin property by db_ident.
pub fn get_builtin_property(db_ident: &str) -> Option<PropertyDefinition> {
    builtin_properties()
        .into_iter()
        .find(|p| p.db_ident == db_ident)
}

fn make_uuid(s: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(s).expect("Invalid hardcoded UUID")
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── PropertyType ────────────────────────────────────────────────

    #[test]
    fn test_property_type_roundtrip() {
        for pt in &[
            PropertyType::Text,
            PropertyType::Number,
            PropertyType::Date,
            PropertyType::DateTime,
            PropertyType::Url,
            PropertyType::Checkbox,
            PropertyType::Node,
        ] {
            let s = pt.as_str();
            let parsed = PropertyType::from_str(s).unwrap();
            assert_eq!(*pt, parsed);
        }
    }

    #[test]
    fn test_property_type_display() {
        assert_eq!(PropertyType::Text.to_string(), "Text");
        assert_eq!(PropertyType::Number.to_string(), "Number");
    }

    // ── Cardinality ─────────────────────────────────────────────────

    #[test]
    fn test_cardinality_roundtrip() {
        assert_eq!(Cardinality::One.as_str(), "one");
        assert_eq!(Cardinality::Many.as_str(), "many");
        assert_eq!(Cardinality::from_str("one"), Some(Cardinality::One));
        assert_eq!(Cardinality::from_str("many"), Some(Cardinality::Many));
    }

    #[test]
    fn test_cardinality_default() {
        assert_eq!(Cardinality::default(), Cardinality::One);
    }

    // ── ViewContext ─────────────────────────────────────────────────

    #[test]
    fn test_view_context_roundtrip() {
        assert_eq!(ViewContext::Page.as_str(), "page");
        assert_eq!(ViewContext::Block.as_str(), "block");
        assert_eq!(ViewContext::Never.as_str(), "never");
        assert_eq!(ViewContext::from_str("page"), Some(ViewContext::Page));
        assert_eq!(ViewContext::from_str("never"), Some(ViewContext::Never));
    }

    #[test]
    fn test_visibility_default() {
        // ADR-0025: new PropertyDefinition has Inline visibility (default)
        assert_eq!(PropertyVisibility::default(), PropertyVisibility::Inline);
    }

    // ── ClosedValue ─────────────────────────────────────────────────

    #[test]
    fn test_closed_value_builder() {
        let id = uuid::Uuid::new_v4();
        let cv = ClosedValue::new(id, "todo", "To Do")
            .with_icon("📋")
            .with_order(1.0);

        assert_eq!(cv.db_ident, "todo");
        assert_eq!(cv.value, "To Do");
        assert_eq!(cv.icon, Some("📋".to_string()));
        assert_eq!(cv.order, 1.0);
    }

    // ── PropertyDefinition ──────────────────────────────────────────

    #[test]
    fn test_property_definition_builder() {
        let id = uuid::Uuid::new_v4();
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
        let id = uuid::Uuid::new_v4();
        let closed = vec![
            ClosedValue::new(uuid::Uuid::new_v4(), "todo", "To Do"),
            ClosedValue::new(uuid::Uuid::new_v4(), "doing", "Doing"),
            ClosedValue::new(uuid::Uuid::new_v4(), "done", "Done"),
        ];
        let prop = PropertyDefinition::new(id, "status", "Status", PropertyType::Text)
            .with_closed_values(closed);

        assert!(prop.has_closed_values());
        assert!(prop.is_value_allowed("To Do"));
        assert!(prop.is_value_allowed("todo"));
        assert!(!prop.is_value_allowed("invalid"));
    }

    #[test]
    fn test_open_set_allows_any_value() {
        let id = uuid::Uuid::new_v4();
        let prop = PropertyDefinition::new(id, "name", "Name", PropertyType::Text);
        assert!(!prop.has_closed_values());
        assert!(prop.is_value_allowed("any value"));
    }

    #[test]
    fn test_property_definition_serialize() {
        let id = uuid::Uuid::new_v4();
        let prop = PropertyDefinition::new(id, "test", "Test", PropertyType::Text);
        let json = serde_json::to_string(&prop).unwrap();
        let restored: PropertyDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(prop, restored);
    }

    // ── WASM parity: from_legacy_fields ─────────────────────────────

    #[test]
    fn wasm_from_legacy_fields_produces_visibility_system_and_mutability_immutable() {
        // Parity with domain: from_legacy_fields(Never, false, false, true) →
        // visibility=System, mutability=Immutable
        let def = PropertyDefinition::from_legacy_fields(
            uuid::Uuid::new_v4(),
            "test",
            "Test",
            PropertyType::Text,
            ViewContext::Never,
            false,
            false,
            false,
            true,
        );
        assert_eq!(def.visibility, PropertyVisibility::System);
        assert_eq!(def.mutability, PropertyMutability::Immutable);
        assert!(def.derived_from.is_none());
        assert_eq!(def.merge_policy, MergePolicy::SetIfMissing);
    }

    // ── Sync Validation ─────────────────────────────────────────────

    fn text_def() -> PropertyDefinition {
        PropertyDefinition::new(uuid::Uuid::new_v4(), "name", "Name", PropertyType::Text)
    }

    fn status_def() -> PropertyDefinition {
        let closed = vec![
            ClosedValue::new(uuid::Uuid::new_v4(), "todo", "To Do"),
            ClosedValue::new(uuid::Uuid::new_v4(), "done", "Done"),
        ];
        PropertyDefinition::new(uuid::Uuid::new_v4(), "status", "Status", PropertyType::Text)
            .with_closed_values(closed)
    }

    #[test]
    fn test_validate_type_text_ok() {
        assert!(validate_type(&text_def(), &serde_json::Value::String("hello".into())).is_ok());
    }

    #[test]
    fn test_validate_type_text_fails_on_number() {
        let result = validate_type(&text_def(), &serde_json::Value::Number(42.into()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Type mismatch"));
    }

    #[test]
    fn test_validate_type_number_ok() {
        let def =
            PropertyDefinition::new(uuid::Uuid::new_v4(), "count", "Count", PropertyType::Number);
        assert!(validate_type(&def, &serde_json::Value::Number(42.into())).is_ok());
        assert!(validate_type(&def, &serde_json::json!(3.14)).is_ok());
    }

    #[test]
    fn test_validate_type_checkbox_ok() {
        let def =
            PropertyDefinition::new(uuid::Uuid::new_v4(), "done", "Done", PropertyType::Checkbox);
        assert!(validate_type(&def, &serde_json::Value::Bool(true)).is_ok());
        assert!(validate_type(&def, &serde_json::Value::Bool(false)).is_ok());
    }

    #[test]
    fn test_validate_cardinality_array_rejected_for_one() {
        let def = PropertyDefinition::new(uuid::Uuid::new_v4(), "name", "Name", PropertyType::Text)
            .with_cardinality(Cardinality::One);
        let arr = serde_json::json!(["a", "b"]);
        let result = validate_cardinality(&def, &arr);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cardinality violation"));
    }

    #[test]
    fn test_validate_cardinality_many_accepts_array() {
        let def = PropertyDefinition::new(uuid::Uuid::new_v4(), "tags", "Tags", PropertyType::Text)
            .with_cardinality(Cardinality::Many);
        assert!(validate_cardinality(&def, &serde_json::json!(["a", "b"])).is_ok());
    }

    #[test]
    fn test_validate_closed_set_valid_value() {
        assert!(validate_closed_set(&status_def(), &serde_json::json!("To Do")).is_ok());
        assert!(validate_closed_set(&status_def(), &serde_json::json!("todo")).is_ok());
    }

    #[test]
    fn test_validate_closed_set_invalid_value() {
        let result = validate_closed_set(&status_def(), &serde_json::json!("INVALID"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in the closed set"));
    }

    #[test]
    fn test_validate_property_full() {
        let def = status_def();
        assert!(validate_property(&def, &serde_json::json!("To Do")).is_ok());
        assert!(validate_property(&def, &serde_json::json!(42)).is_err()); // wrong type
        assert!(validate_property(&def, &serde_json::json!("INVALID")).is_err());
        // closed set
    }

    // ── Builtins ────────────────────────────────────────────────────

    #[test]
    fn test_builtin_properties_exist() {
        let props = builtin_properties();
        let idents: Vec<&str> = props.iter().map(|p| p.db_ident.as_str()).collect();
        assert!(idents.contains(&"quilt.property/status"));
        assert!(idents.contains(&"quilt.property/priority"));
        assert!(idents.contains(&"quilt.property/deadline"));
        assert!(idents.contains(&"quilt.property/scheduled"));
        assert!(idents.contains(&"quilt.property/url"));
    }

    #[test]
    fn test_get_builtin_property() {
        let status = get_builtin_property("quilt.property/status").unwrap();
        assert_eq!(status.property_type, PropertyType::Text);
        assert!(status.has_closed_values());
    }

    #[test]
    fn test_builtin_priority_values() {
        let priority = get_builtin_property("quilt.property/priority").unwrap();
        assert!(priority.is_value_allowed("A"));
        assert!(priority.is_value_allowed("B"));
        assert!(priority.is_value_allowed("C"));
        assert!(!priority.is_value_allowed("D"));
    }
}
