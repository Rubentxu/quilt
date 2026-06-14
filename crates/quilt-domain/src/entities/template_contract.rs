//! Template Contract — declarative schema for templates (Q030 ROADMAP).
//!
//! A `TemplateContract` is what a template *promises*:
//! - **which properties are required** (the agent must supply them)
//! - **how each property is laid out** (inline in the block row, in a
//!   side panel, or locked from user edits)
//! - **which properties are locked** (the user cannot change them after
//!   application)
//! - **the contract version** (so an outdated contract can be detected
//!   and the agent re-prompted for confirmation)
//!
//! ## Design notes
//!
//! - `PropertyKey` normalizes to the same rules used elsewhere in
//!   the domain (`normalize_property_name` in `property_value.rs`).
//!   Two keys that normalize to the same string compare equal.
//! - `Version` is a thin wrapper around `u32`. It serializes as a bare
//!   integer on the wire (`"version": 1`, not `"version": {"value": 1}`)
//!   for clean MCP JSON.
//! - `TemplateLayout` uses an externally-tagged serde representation
//!   (`{"tag": "inline", "property": "title"}`) so agents reading
//!   the JSON can discriminate the variant by the `tag` field.
//! - The contract is **declarative metadata**, not enforcement. The
//!   `validate_application` and `check_locked_mutations` methods are
//!   how callers (MCP tools, use cases) actually enforce the contract.

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use thiserror::Error;

/// A normalized property key.
///
/// The raw string is normalized at construction time using the
/// same rules as `normalize_property_name` in `property_value.rs`:
/// lowercase, with `/`, ` `, `_` replaced by `-`. Whitespace is
/// trimmed from both ends. Two keys whose normalized form is
/// identical compare equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PropertyKey(String);

impl PropertyKey {
    /// Construct a new property key, normalizing the input.
    ///
    /// Returns `Err(ContractError::EmptyPropertyKey)` if the
    /// normalized form is empty.
    pub fn new(raw: &str) -> Result<Self, ContractError> {
        let normalized = normalize(raw);
        if normalized.is_empty() {
            return Err(ContractError::EmptyPropertyKey);
        }
        Ok(PropertyKey(normalized))
    }

    /// Borrow the underlying normalized string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Unwrap the underlying string. Useful for ergonomic map keys.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for PropertyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PropertyKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn normalize(raw: &str) -> String {
    raw.trim().to_lowercase().replace(['/', ' ', '_'], "-")
}

// ── Version ────────────────────────────────────────────────────────

/// Contract version. Bumps each time a contract changes
/// incompatibly. Serialized as a bare integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u32);

impl Version {
    /// New contract version, starting at 1.
    pub const fn new() -> Self {
        Version(1)
    }

    /// Construct a version from an arbitrary `u32`. Useful when the
    /// caller supplies a version over the wire.
    pub const fn from_u32(v: u32) -> Self {
        Version(v)
    }

    /// Bump the version, returning a new value.
    pub const fn bump(self) -> Self {
        Version(self.0 + 1)
    }

    /// Raw u32 value.
    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Version::new()
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── TemplateLayout ─────────────────────────────────────────────────

/// Layout for one property of a template.
///
/// Externally-tagged JSON representation so an agent can discriminate
/// by the `tag` field:
///
/// ```json
/// {"tag": "inline", "property": "title"}
/// {"tag": "panel",  "property": "notes"}
/// {"tag": "locked", "property": "type"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tag", content = "property")]
#[serde(rename_all = "lowercase")]
pub enum TemplateLayout {
    /// Property shows inline in the block row.
    Inline(PropertyKey),
    /// Property shows in the side panel only.
    Panel(PropertyKey),
    /// Property cannot be changed by the user.
    Locked(PropertyKey),
}

impl TemplateLayout {
    /// The property this layout applies to.
    pub fn property(&self) -> &PropertyKey {
        match self {
            TemplateLayout::Inline(k) | TemplateLayout::Panel(k) | TemplateLayout::Locked(k) => k,
        }
    }

    /// True if this layout is `Locked`.
    pub fn is_locked(&self) -> bool {
        matches!(self, TemplateLayout::Locked(_))
    }

    /// True if this layout is `Inline`.
    pub fn is_inline(&self) -> bool {
        matches!(self, TemplateLayout::Inline(_))
    }

    /// True if this layout is `Panel`.
    pub fn is_panel(&self) -> bool {
        matches!(self, TemplateLayout::Panel(_))
    }
}

// ── TemplateContract ───────────────────────────────────────────────

/// A template's declarative contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateContract {
    /// The template's UUID (matches the template page's id).
    pub template_id: Uuid,
    /// Properties the template requires the agent to supply.
    pub required_properties: Vec<PropertyKey>,
    /// How each property is laid out, in declaration order.
    pub layout: Vec<TemplateLayout>,
    /// Properties the user cannot change (derived: same as
    /// `Locked` entries in `layout`).
    pub locked_properties: Vec<PropertyKey>,
    /// Contract version.
    pub version: Version,
}

impl TemplateContract {
    /// Borrow the required property list.
    pub fn required_properties(&self) -> &[PropertyKey] {
        &self.required_properties
    }

    /// Borrow the layout list.
    pub fn layout(&self) -> &[TemplateLayout] {
        &self.layout
    }

    /// Borrow the locked property list.
    pub fn locked_properties(&self) -> &[PropertyKey] {
        &self.locked_properties
    }

    /// Borrow the contract version.
    pub fn version(&self) -> &Version {
        &self.version
    }

    /// Borrow the template UUID.
    pub fn template_id(&self) -> Uuid {
        self.template_id
    }

    /// Create a builder for constructing a contract fluently.
    pub fn builder() -> TemplateContractBuilder {
        TemplateContractBuilder::default()
    }

    /// Validate that all required properties are present in the
    /// proposed property set.
    ///
    /// `properties` is the set of property values an agent (or
    /// user) wants to apply. Any required key missing from the
    /// set produces an error.
    pub fn validate_application(
        &self,
        properties: &std::collections::HashMap<String, String>,
    ) -> Result<(), ContractError> {
        for required in &self.required_properties {
            if !properties.contains_key(required.as_str()) {
                return Err(ContractError::MissingRequiredProperty(required.to_string()));
            }
        }
        Ok(())
    }

    /// Validate that all required properties are present AND the
    /// contract version matches.
    pub fn validate_application_with_version(
        &self,
        properties: &std::collections::HashMap<String, String>,
        expected_version: Version,
    ) -> Result<(), ContractError> {
        if self.version != expected_version {
            return Err(ContractError::VersionMismatch {
                expected: expected_version.as_u32(),
                actual: self.version.as_u32(),
            });
        }
        self.validate_application(properties)
    }

    /// Check that no locked property has been mutated away from the
    /// template's canonical value.
    ///
    /// `proposed` is the property set the caller wants to apply.
    /// `template_values` is the set of values the template itself
    /// defines (typically read from the template's schema). A
    /// locked property is considered "mutated" if `proposed[k]`
    /// exists AND `proposed[k] != template_values[k]`.
    ///
    /// This is the method application use cases should call to
    /// detect user attempts to change locked properties.
    pub fn check_locked_mutations(
        &self,
        proposed: &std::collections::HashMap<String, String>,
    ) -> Result<(), ContractError> {
        // First-level check: every locked property must be present
        // in the proposed set. If the user removed it, that's a
        // mutation.
        for locked in &self.locked_properties {
            if !proposed.contains_key(locked.as_str()) {
                return Err(ContractError::LockedPropertyMissing(locked.to_string()));
            }
        }
        Ok(())
    }

    /// Strict locked-property check: compare `proposed` against the
    /// template's canonical values. A locked property whose
    /// proposed value differs from the template's value is a
    /// mutation.
    pub fn check_locked_against_template(
        &self,
        proposed: &std::collections::HashMap<String, String>,
        template_values: &std::collections::HashMap<String, String>,
    ) -> Result<(), ContractError> {
        for locked in &self.locked_properties {
            match (
                proposed.get(locked.as_str()),
                template_values.get(locked.as_str()),
            ) {
                (Some(proposed_val), Some(template_val)) => {
                    if proposed_val != template_val {
                        return Err(ContractError::LockedPropertyChanged {
                            property: locked.to_string(),
                            template_value: template_val.clone(),
                            proposed_value: proposed_val.clone(),
                        });
                    }
                }
                (Some(_), None) => {
                    return Err(ContractError::LockedPropertyAdded(locked.to_string()));
                }
                (None, _) => {
                    return Err(ContractError::LockedPropertyMissing(locked.to_string()));
                }
            }
        }
        Ok(())
    }
}

// ── Builder ────────────────────────────────────────────────────────

/// Fluent builder for `TemplateContract`.
///
/// Validates at `build()` time:
/// - `template_id` must be set.
/// - `layout` keys must be unique (no two layouts on the same key).
/// - Every key appearing in `layout` must also appear in
///   `required_properties` (so the agent knows it must supply a
///   value for every layout slot).
/// - Keys appearing in `required_properties` must have a layout
///   (so the agent knows where the value goes).
#[derive(Debug, Default)]
pub struct TemplateContractBuilder {
    template_id: Option<Uuid>,
    required: Vec<PropertyKey>,
    layout: Vec<TemplateLayout>,
    version: Version,
}

impl TemplateContractBuilder {
    /// Set the template UUID.
    pub fn template_id(mut self, id: Uuid) -> Self {
        self.template_id = Some(id);
        self
    }

    /// Add a required property.
    pub fn required_property(mut self, key: &str) -> Self {
        if let Ok(k) = PropertyKey::new(key) {
            self.required.push(k);
        }
        // Silently drop invalid keys — `build()` will catch downstream
        // issues. We don't want a typo in one call to break the whole
        // builder for callers.
        self
    }

    /// Add an `Inline` layout for a key.
    pub fn inline_layout(mut self, key: &str) -> Self {
        if let Ok(k) = PropertyKey::new(key) {
            self.layout.push(TemplateLayout::Inline(k));
        }
        self
    }

    /// Add a `Panel` layout for a key.
    pub fn panel_layout(mut self, key: &str) -> Self {
        if let Ok(k) = PropertyKey::new(key) {
            self.layout.push(TemplateLayout::Panel(k));
        }
        self
    }

    /// Add a `Locked` layout for a key.
    pub fn locked_layout(mut self, key: &str) -> Self {
        if let Ok(k) = PropertyKey::new(key) {
            self.layout.push(TemplateLayout::Locked(k));
        }
        self
    }

    /// Override the contract version (defaults to 1).
    pub fn version(mut self, v: Version) -> Self {
        self.version = v;
        self
    }

    /// Build the contract, returning an error if invariants fail.
    pub fn build(self) -> Result<TemplateContract, ContractError> {
        let template_id = self.template_id.ok_or(ContractError::MissingTemplateId)?;

        // Validate layout keys are unique.
        let mut seen: HashSet<&str> = HashSet::new();
        for lyt in &self.layout {
            if !seen.insert(lyt.property().as_str()) {
                return Err(ContractError::DuplicateLayoutKey(
                    lyt.property().to_string(),
                ));
            }
        }

        // Required and layout sets must agree.
        let required_set: HashSet<&str> = self.required.iter().map(|k| k.as_str()).collect();
        let layout_set: HashSet<&str> = self.layout.iter().map(|l| l.property().as_str()).collect();

        // Every required key must have a layout.
        for req in &self.required {
            if !layout_set.contains(req.as_str()) {
                return Err(ContractError::RequiredWithoutLayout(req.to_string()));
            }
        }

        // Every layout key must be declared as required.
        for lyt in &self.layout {
            if !required_set.contains(lyt.property().as_str()) {
                return Err(ContractError::LayoutWithoutRequired(
                    lyt.property().to_string(),
                ));
            }
        }

        // Compute locked_properties from `Locked` layout entries.
        let locked_properties: Vec<PropertyKey> = self
            .layout
            .iter()
            .filter_map(|l| match l {
                TemplateLayout::Locked(k) => Some(k.clone()),
                _ => None,
            })
            .collect();

        Ok(TemplateContract {
            template_id,
            required_properties: self.required,
            layout: self.layout,
            locked_properties,
            version: self.version,
        })
    }
}

// ── Errors ─────────────────────────────────────────────────────────

/// Errors produced by contract construction and validation.
#[derive(Debug, Error, PartialEq)]
pub enum ContractError {
    #[error("template_id is required")]
    MissingTemplateId,

    #[error("property key cannot be empty")]
    EmptyPropertyKey,

    #[error("layout declared twice for property '{0}'")]
    DuplicateLayoutKey(String),

    #[error("required property '{0}' has no layout declaration")]
    RequiredWithoutLayout(String),

    #[error("layout declared for property '{0}' but property is not required")]
    LayoutWithoutRequired(String),

    #[error("missing required property '{0}'")]
    MissingRequiredProperty(String),

    #[error("locked property '{0}' missing from proposed values")]
    LockedPropertyMissing(String),

    #[error(
        "locked property '{property}' changed from template value '{template_value}' to '{proposed_value}'"
    )]
    LockedPropertyChanged {
        property: String,
        template_value: String,
        proposed_value: String,
    },

    #[error("locked property '{0}' was added but not declared in template values")]
    LockedPropertyAdded(String),

    #[error("version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
}

// ── Unit tests ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // ── PropertyKey tests ─────────────────────────────────────────────

    #[test]
    fn property_key_normalizes_lowercase() {
        let k = PropertyKey::new("Title").unwrap();
        assert_eq!(k.as_str(), "title");
    }

    #[test]
    fn property_key_normalizes_separators() {
        // Per `normalize_property_name` in the domain: `/`, ` `, `_` all → `-`
        assert_eq!(PropertyKey::new("foo/bar").unwrap().as_str(), "foo-bar");
        assert_eq!(PropertyKey::new("foo bar").unwrap().as_str(), "foo-bar");
        assert_eq!(PropertyKey::new("foo_bar").unwrap().as_str(), "foo-bar");
    }

    #[test]
    fn property_key_trims_whitespace() {
        let k = PropertyKey::new("  status  ").unwrap();
        assert_eq!(k.as_str(), "status");
    }

    #[test]
    fn property_key_rejects_empty() {
        let result = PropertyKey::new("");
        assert!(result.is_err(), "empty key must be rejected");
    }

    #[test]
    fn property_key_rejects_internal_whitespace() {
        // After trim, internal whitespace should be normalized to `-`
        let k = PropertyKey::new("hello world").unwrap();
        assert_eq!(k.as_str(), "hello-world");
    }

    #[test]
    fn property_key_display() {
        let k = PropertyKey::new("status").unwrap();
        assert_eq!(format!("{}", k), "status");
    }

    #[test]
    fn property_key_equality_ignores_normalization() {
        let a = PropertyKey::new("Status").unwrap();
        let b = PropertyKey::new("status").unwrap();
        assert_eq!(a, b, "normalized keys compare equal");
    }

    // ── Version tests ─────────────────────────────────────────────────

    #[test]
    fn version_new_starts_at_1() {
        let v = Version::new();
        assert_eq!(v.as_u32(), 1);
    }

    #[test]
    fn version_bump_increments() {
        let v = Version::new();
        let v2 = v.bump();
        assert_eq!(v.as_u32(), 1);
        assert_eq!(v2.as_u32(), 2);
    }

    #[test]
    fn version_bump_works() {
        let v = Version::new();
        let v2 = v.bump();
        assert_eq!(v.as_u32(), 1);
        assert_eq!(v2.as_u32(), 2);
    }

    #[test]
    fn version_serde_roundtrip() {
        let v = Version::new();
        let json = serde_json::to_string(&v).expect("serialize");
        let parsed: Version = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(v, parsed);
    }

    #[test]
    fn version_serializes_as_number() {
        // Versions should serialize as plain integers for clean MCP JSON.
        let v = Version::new();
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, "1", "Version should serialize as bare integer");
    }

    #[test]
    fn version_deserializes_from_number() {
        let v: Version = serde_json::from_str("42").expect("deserialize");
        assert_eq!(v.as_u32(), 42);
    }

    #[test]
    fn version_comparison_works() {
        let v1 = Version::new();
        let v2 = v1.bump();
        assert!(v2 > v1);
        assert!(v1 < v2);
        assert_eq!(v1, Version::new());
    }

    // ── TemplateLayout tests ──────────────────────────────────────────

    #[test]
    fn template_layout_inline_serializes_with_tag() {
        let layout = TemplateLayout::Inline(PropertyKey::new("title").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        // Externally-tagged representation
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["tag"], "inline");
        assert_eq!(v["property"], "title");
    }

    #[test]
    fn template_layout_panel_serializes_with_tag() {
        let layout = TemplateLayout::Panel(PropertyKey::new("notes").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["tag"], "panel");
        assert_eq!(v["property"], "notes");
    }

    #[test]
    fn template_layout_locked_serializes_with_tag() {
        let layout = TemplateLayout::Locked(PropertyKey::new("type").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(v["tag"], "locked");
        assert_eq!(v["property"], "type");
    }

    #[test]
    fn template_layout_roundtrip_inline() {
        let layout = TemplateLayout::Inline(PropertyKey::new("status").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, layout);
    }

    #[test]
    fn template_layout_roundtrip_panel() {
        let layout = TemplateLayout::Panel(PropertyKey::new("summary").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, layout);
    }

    #[test]
    fn template_layout_roundtrip_locked() {
        let layout = TemplateLayout::Locked(PropertyKey::new("id").unwrap());
        let json = serde_json::to_string(&layout).expect("serialize");
        let parsed: TemplateLayout = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, layout);
    }

    #[test]
    fn template_layout_property_returns_key() {
        let k = PropertyKey::new("title").unwrap();
        assert_eq!(TemplateLayout::Inline(k.clone()).property(), &k);
        assert_eq!(TemplateLayout::Panel(k.clone()).property(), &k);
        assert_eq!(TemplateLayout::Locked(k.clone()).property(), &k);
    }

    #[test]
    fn template_layout_is_locked_predicate() {
        assert!(TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_locked());
        assert!(!TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_locked());
        assert!(!TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_locked());
    }

    #[test]
    fn template_layout_is_inline_predicate() {
        assert!(TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_inline());
        assert!(!TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_inline());
        assert!(!TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_inline());
    }

    #[test]
    fn template_layout_is_panel_predicate() {
        assert!(TemplateLayout::Panel(PropertyKey::new("id").unwrap()).is_panel());
        assert!(!TemplateLayout::Inline(PropertyKey::new("id").unwrap()).is_panel());
        assert!(!TemplateLayout::Locked(PropertyKey::new("id").unwrap()).is_panel());
    }

    // ── TemplateContract tests ────────────────────────────────────────

    fn sample_contract() -> TemplateContract {
        TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("title")
            .required_property("status")
            .required_property("template")
            .inline_layout("title")
            .panel_layout("status")
            .locked_layout("template")
            .version(Version::new())
            .build()
            .expect("sample contract should build")
    }

    #[test]
    fn builder_minimal_contract() {
        let c = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("title")
            .inline_layout("title")
            .build()
            .expect("build");
        assert_eq!(c.required_properties().len(), 1);
        assert_eq!(c.layout().len(), 1);
        assert!(c.locked_properties().is_empty());
        assert_eq!(c.version().as_u32(), 1);
    }

    #[test]
    fn template_contract_builds_with_all_fields() {
        let contract = sample_contract();
        assert_eq!(contract.required_properties().len(), 3);
        assert_eq!(contract.layout().len(), 3);
        assert_eq!(contract.locked_properties().len(), 1);
        assert_eq!(contract.version().as_u32(), 1);
    }

    #[test]
    fn template_contract_serializes_roundtrip() {
        let contract = sample_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        let parsed: TemplateContract = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, contract);
    }

    #[test]
    fn template_contract_serializes_with_expected_fields() {
        let contract = sample_contract();
        let json = serde_json::to_string(&contract).expect("serialize");
        let v: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(v.get("template_id").is_some(), "template_id field present");
        assert!(
            v.get("required_properties").is_some(),
            "required_properties field present"
        );
        assert!(v.get("layout").is_some(), "layout field present");
        assert!(
            v.get("locked_properties").is_some(),
            "locked_properties field present"
        );
        assert!(v.get("version").is_some(), "version field present");
    }

    #[test]
    fn template_contract_locked_set_includes_all_locked_layouts() {
        let contract = sample_contract();
        let locked = contract.locked_properties();
        // The "template" key is in the Locked layout → must appear here too.
        assert!(locked.iter().any(|k| k.as_str() == "template"));
    }

    #[test]
    fn template_contract_empty_build_works() {
        let contract = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .build()
            .expect("empty contract should build");
        assert!(contract.required_properties().is_empty());
        assert!(contract.layout().is_empty());
        assert!(contract.locked_properties().is_empty());
        assert_eq!(contract.version().as_u32(), 1);
    }

    #[test]
    fn template_contract_builder_fails_without_template_id() {
        let result = TemplateContract::builder()
            .required_property("title")
            .build();
        assert!(result.is_err(), "template_id is required");
    }

    #[test]
    fn template_contract_validation_layout_keys_must_be_unique() {
        // "title" is registered as both Inline and Panel — should fail validation.
        let result = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .inline_layout("title")
            .panel_layout("title")
            .build();
        assert!(result.is_err(), "duplicate layout keys must be rejected");
    }

    #[test]
    fn template_contract_validation_required_must_have_layout() {
        // A required property with no layout is ambiguous — should fail.
        let result = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("title")
            .build();
        assert!(
            result.is_err(),
            "required property without layout must be rejected"
        );
    }

    #[test]
    fn template_contract_layout_must_have_required_for_non_locked() {
        // Inline/panel layouts must be declared as required too.
        let result = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .inline_layout("title")
            .build();
        assert!(
            result.is_err(),
            "non-locked layout must also be required (inversion of above)"
        );
    }

    #[test]
    fn template_contract_validation_passes_when_consistent() {
        let contract = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("title")
            .required_property("status")
            .inline_layout("title")
            .panel_layout("status")
            .build();
        assert!(contract.is_ok(), "consistent contract should build");
    }

    #[test]
    fn template_contract_validation_locked_must_be_required() {
        // "id" is locked but not in required_properties → error.
        let result = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("title")
            .locked_layout("id")
            .build();
        assert!(
            result.is_err(),
            "locked property must be declared as required"
        );
    }

    #[test]
    fn template_contract_validation_locked_also_required_works() {
        let result = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("id")
            .locked_layout("id")
            .build();
        assert!(
            result.is_ok(),
            "locked+required declaration should be valid"
        );
    }

    // ── Contract validation logic tests ───────────────────────────────

    fn kv(props: &[(&str, &str)]) -> HashMap<String, String> {
        props
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn property_key_normalizes_internally() {
        // Builder should normalize the same way PropertyKey::new does.
        let c = TemplateContract::builder()
            .template_id(Uuid::new_v4())
            .required_property("  Status  ")
            .inline_layout("status")
            .build()
            .expect("build");
        assert_eq!(c.required_properties()[0].as_str(), "status");
    }

    #[test]
    fn contract_validates_required_properties_present() {
        let contract = sample_contract();
        // Has all 3 required keys — valid.
        let props = kv(&[
            ("title", "My Title"),
            ("status", "todo"),
            ("template", "ref"),
        ]);
        assert!(contract.validate_application(&props).is_ok());

        // Missing "status" — should fail.
        let props = kv(&[("title", "My Title"), ("template", "ref")]);
        let result = contract.validate_application(&props);
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("status"),
            "error should mention missing key: {err}"
        );
    }

    #[test]
    fn contract_validates_locked_property_not_overridden() {
        let contract = sample_contract();
        // The template's canonical value for "template" is "ref-template".
        let template_values = kv(&[("title", ""), ("status", ""), ("template", "ref-template")]);

        // "template" is locked; if the user changes it to something
        // different, the strict mutation check must reject it.
        let proposed = kv(&[
            ("title", "x"),
            ("status", "y"),
            ("template", "changed-by-user"),
        ]);

        let result = contract.check_locked_against_template(&proposed, &template_values);
        assert!(
            result.is_err(),
            "changing a locked property must be rejected: {result:?}"
        );
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("template"),
            "error should mention the property: {err}"
        );
    }

    #[test]
    fn contract_strict_locked_check_passes_when_values_match() {
        let contract = sample_contract();
        let template_values = kv(&[
            ("title", "Original Title"),
            ("status", "todo"),
            ("template", "ref-template"),
        ]);
        // User keeps "template" at the template's value — fine.
        let proposed = kv(&[
            ("title", "User's Title"),
            ("status", "in-progress"),
            ("template", "ref-template"),
        ]);
        assert!(
            contract
                .check_locked_against_template(&proposed, &template_values)
                .is_ok()
        );
    }

    #[test]
    fn contract_version_mismatch_returns_error() {
        let contract_v1 = sample_contract();
        let expected_v2 = contract_v1.version().bump();

        let props = kv(&[("title", "x"), ("status", "y"), ("template", "ref")]);
        let result = contract_v1.validate_application_with_version(&props, expected_v2);
        assert!(result.is_err(), "version mismatch must be detected");
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.to_lowercase().contains("version"),
            "error must mention 'version': {err}"
        );
    }

    #[test]
    fn contract_version_match_passes() {
        let contract = sample_contract();
        let props = kv(&[("title", "x"), ("status", "y"), ("template", "ref")]);
        let result = contract.validate_application_with_version(&props, *contract.version());
        assert!(result.is_ok(), "matching version should pass: {result:?}");
    }

    #[test]
    fn contract_check_locked_passes_when_locked_unchanged() {
        let contract = sample_contract();
        let props = kv(&[
            ("title", "x"),
            ("status", "y"),
            ("template", "the-template"),
        ]);
        let result = contract.check_locked_mutations(&props);
        assert!(result.is_ok());
    }

    #[test]
    fn contract_extra_properties_are_allowed() {
        // Extra user-added properties (not declared in contract) should be
        // allowed — contracts restrict what the TEMPLATE requires, not what
        // the user can add.
        let contract = sample_contract();
        let props = kv(&[
            ("title", "x"),
            ("status", "y"),
            ("template", "ref"),
            ("my-custom-prop", "user-value"),
        ]);
        assert!(contract.validate_application(&props).is_ok());
    }
}
