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

// ── Unit tests (in addition to the integration test in tests/) ─────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn version_bump_works() {
        let v = Version::new();
        let v2 = v.bump();
        assert_eq!(v.as_u32(), 1);
        assert_eq!(v2.as_u32(), 2);
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
}
