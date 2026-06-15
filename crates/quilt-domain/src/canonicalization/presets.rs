//! Value objects for property presets — named bundles of [`PropertyPatch`] entries.
//!
//! A [`PropertyPreset`] is a reusable, named collection of [`PropertyPatch`]es that can be
//! applied to a block via the [`ApplyPreset`](super::super::application::use_cases::apply_preset::ApplyPreset)
//! use case. Presets never touch block `content`, `text`, or `children` — they only
//! set typed properties with explicit merge policies.
//!
//! # Example
//!
//! ```rust
//! use quilt_domain::canonicalization::{PresetId, PropertyPreset, PropertyPatch, PresetArgs, PresetArgKind};
//! use quilt_domain::entities::PropertyKey;
//! use quilt_domain::value_objects::PropertyValue;
//!
//! // /TODO preset — sets type:: task, status:: todo, projection:: auto
//! let preset = PropertyPreset::new(
//!     PresetId::new("/TODO").unwrap(),
//!     vec![
//!         PropertyPatch::explicit(
//!             PropertyKey::new("type").unwrap(),
//!             PropertyValue::text("task"),
//!         ),
//!         PropertyPatch::explicit(
//!             PropertyKey::new("status").unwrap(),
//!             PropertyValue::text("todo"),
//!         ),
//!         PropertyPatch::explicit(
//!             PropertyKey::new("projection").unwrap(),
//!             PropertyValue::text("auto"),
//!         ),
//!     ],
//!     PresetArgs::empty(),
//!     "Marks a block as a TODO task".into(),
//! ).unwrap();
//! ```

use crate::canonicalization::PropertyPatch;
use crate::errors::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;

/// A preset identifier — a slash command name like `/TODO`, `/Scheduled`, `/Video`.
///
/// # Validation
///
/// A valid `PresetId`:
/// - Starts with `/`
/// - Contains no internal whitespace
/// - Contains no second `/`
/// - Is non-empty after the `/`
///
/// # Case sensitivity
///
/// `PresetId` is **case-sensitive**: `/TODO` and `/todo` are distinct presets.
/// This matches the slash command UX where `/TODO` triggers the TODO marker
/// canonicalizer and `/todo` would not.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresetId(Arc<str>);

impl Ord for PresetId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for PresetId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PresetId {
    /// Construct a [`PresetId`] from a string slice.
    ///
    /// Returns `Err(DomainError::InvalidPresetId)` if the string does not meet
    /// the validation rules.
    pub fn new(s: impl AsRef<str>) -> Result<Self, DomainError> {
        let s = s.as_ref();

        // Empty after '/'
        if s.is_empty() || s == "/" {
            return Err(DomainError::InvalidPresetId(s.into()));
        }

        // Must start with '/'
        if !s.starts_with('/') {
            return Err(DomainError::InvalidPresetId(s.into()));
        }

        let after_slash = &s[1..];

        // Empty after '/'
        if after_slash.is_empty() {
            return Err(DomainError::InvalidPresetId(s.into()));
        }

        // Internal whitespace
        if after_slash.chars().any(|c| c.is_whitespace()) {
            return Err(DomainError::InvalidPresetId(s.into()));
        }

        // Second '/'
        if after_slash.contains('/') {
            return Err(DomainError::InvalidPresetId(s.into()));
        }

        Ok(PresetId(Arc::from(after_slash)))
    }

    /// Borrow the underlying string (without the leading `/`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PresetId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PresetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "/{}", self.0)
    }
}

impl Serialize for PresetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a plain string: "/TODO"
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PresetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PresetId::new(&s).map_err(serde::de::Error::custom)
    }
}

/// The kind of a preset argument — determines what type of value is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetArgKind {
    /// A calendar date argument.
    Date,
    /// A URL argument (for media presets).
    Url,
    /// Free-form text argument.
    Text,
}

/// A typed argument value for a preset.
#[derive(Debug, Clone, PartialEq)]
pub enum PresetArg {
    /// A calendar date value.
    Date(chrono::NaiveDate),
    /// A URL value.
    Url(url::Url),
    /// A free-form text value.
    Text(String),
}

// Manual serde impl: internally-tagged with kind as field name and content directly
// Format: {"date": "2026-12-25", "url": "https://...", "text": "hello"}
impl serde::Serialize for PresetArg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        match self {
            PresetArg::Date(date) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("date", &date.to_string())?;
                map.end()
            }
            PresetArg::Url(url) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("url", &url.to_string())?;
                map.end()
            }
            PresetArg::Text(text) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("text", text)?;
                map.end()
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for PresetArg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess};

        struct PresetArgVisitor;
        impl<'de> de::Visitor<'de> for PresetArgVisitor {
            type Value = PresetArg;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a preset argument object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                // We expect exactly one key: "date", "url", or "text"
                let key = match map.next_key::<String>()? {
                    Some(k) => k,
                    None => return Err(de::Error::missing_field("preset arg key")),
                };
                match key.as_str() {
                    "date" => {
                        let date_str = map.next_value::<String>()?;
                        let date = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                            .map_err(|_| de::Error::custom("invalid date format"))?;
                        Ok(PresetArg::Date(date))
                    }
                    "url" => {
                        let url_str = map.next_value::<String>()?;
                        let url = url::Url::parse(&url_str)
                            .map_err(|_| de::Error::custom("invalid url"))?;
                        Ok(PresetArg::Url(url))
                    }
                    "text" => {
                        let text = map.next_value::<String>()?;
                        Ok(PresetArg::Text(text))
                    }
                    _ => Err(de::Error::custom("unknown preset arg kind")),
                }
            }
        }

        deserializer.deserialize_map(PresetArgVisitor)
    }
}

impl fmt::Display for PresetArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetArg::Date(date) => write!(f, "{}", date),
            PresetArg::Url(url) => write!(f, "{}", url),
            PresetArg::Text(text) => write!(f, "{}", text),
        }
    }
}

/// A collection of preset arguments.
///
/// # Invariant
///
/// No two arguments may have the same [`PresetArgKind`].
#[derive(Debug, Clone, PartialEq)]
pub struct PresetArgs(Vec<PresetArg>);

impl Default for PresetArgs {
    fn default() -> Self {
        Self::empty()
    }
}

// Manual serde impl: PresetArgs serializes as a Vec<PresetArg>
impl serde::Serialize for PresetArgs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for PresetArgs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = Vec::<PresetArg>::deserialize(deserializer)?;
        // Reconstruct to validate (reject duplicates)
        Self::from_vec(v).map_err(serde::de::Error::custom)
    }
}

impl PresetArgs {
    /// Construct an empty argument list.
    #[must_use]
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    /// Construct a [`PresetArgs`] from a vector of [`PresetArg`]s.
    ///
    /// Returns `Err(DomainError::DuplicatePresetArgKind)` if two arguments
    /// share the same [`PresetArgKind`].
    pub fn from_vec(v: Vec<PresetArg>) -> Result<Self, DomainError> {
        // Check for duplicate kinds
        let mut seen: Vec<PresetArgKind> = Vec::new();
        for arg in &v {
            let kind = match arg {
                PresetArg::Date(_) => PresetArgKind::Date,
                PresetArg::Url(_) => PresetArgKind::Url,
                PresetArg::Text(_) => PresetArgKind::Text,
            };
            if seen.contains(&kind) {
                return Err(DomainError::DuplicatePresetArgKind(kind));
            }
            seen.push(kind);
        }
        Ok(Self(v))
    }

    /// Returns `true` if the argument list is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of arguments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Get the argument for a given kind, if present.
    #[must_use]
    pub fn get(&self, kind: PresetArgKind) -> Option<&PresetArg> {
        self.0.iter().find(|arg| {
            let k = match arg {
                PresetArg::Date(_) => PresetArgKind::Date,
                PresetArg::Url(_) => PresetArgKind::Url,
                PresetArg::Text(_) => PresetArgKind::Text,
            };
            k == kind
        })
    }

    /// Iterate over the arguments.
    pub fn iter(&self) -> std::slice::Iter<'_, PresetArg> {
        self.0.iter()
    }
}

impl IntoIterator for PresetArgs {
    type Item = PresetArg;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// A named bundle of property patches that can be applied to a block.
///
/// # Forbidden keys
///
/// A preset may **not** contain patches for the following keys:
/// - `content`
/// - `text`
/// - `children`
///
/// These keys are structural block fields and are never modified by a preset.
/// The constructor enforces this invariant; attempting to construct a
/// `PropertyPreset` with a forbidden key returns `Err(DomainError::ForbiddenPatchKey)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyPreset {
    /// Unique identifier for this preset.
    pub id: PresetId,
    /// Ordered list of property patches to apply.
    pub patches: Vec<PropertyPatch>,
    /// Arguments required by this preset.
    pub required_args: PresetArgs,
    /// Human-readable description of this preset.
    #[serde(skip)]
    pub description: String,
}

impl PropertyPreset {
    /// Construct a new [`PropertyPreset`].
    ///
    /// Returns `Err(DomainError::ForbiddenPatchKey)` if any patch targets
    /// `content`, `text`, or `children`.
    pub fn new(
        id: PresetId,
        patches: Vec<PropertyPatch>,
        required_args: PresetArgs,
        description: impl Into<String>,
    ) -> Result<Self, DomainError> {
        // Forbidden-key guard: presets must not touch structural fields
        for patch in &patches {
            let key = patch.key.as_str();
            if key == "content" || key == "text" || key == "children" {
                return Err(DomainError::ForbiddenPatchKey(key.into()));
            }
        }

        Ok(Self {
            id,
            patches,
            required_args,
            description: description.into(),
        })
    }

    /// Override the description, returning a new instance.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonicalization::PropertyPatchProvenance;
    use crate::entities::PropertyKey;
    use crate::value_objects::PropertyValue;
    use std::collections::HashMap;

    fn make_key(s: &str) -> PropertyKey {
        PropertyKey::new(s).expect("valid key")
    }

    fn explicit_patch(key: &str, value: &str) -> PropertyPatch {
        PropertyPatch {
            key: make_key(key),
            value: PropertyValue::text(value),
            provenance: PropertyPatchProvenance::Explicit,
        }
    }

    // ── PresetId ─────────────────────────────────────────────────────────────

    #[test]
    fn preset_id_valid_slash_command() {
        let id = PresetId::new("/TODO").unwrap();
        assert_eq!(id.as_str(), "TODO");
        assert_eq!(id.to_string(), "/TODO");
    }

    #[test]
    fn preset_id_case_sensitive() {
        // /TODO and /todo are different
        let todo = PresetId::new("/TODO").unwrap();
        let lower = PresetId::new("/todo").unwrap();
        assert_ne!(todo, lower);
    }

    #[test]
    fn preset_id_rejects_empty() {
        let r = PresetId::new("");
        assert!(r.is_err());
        if let Err(DomainError::InvalidPresetId(s)) = r {
            assert_eq!(s, "");
        } else {
            panic!("expected InvalidPresetId, got {r:?}");
        }
    }

    #[test]
    fn preset_id_rejects_only_slash() {
        let r = PresetId::new("/");
        assert!(r.is_err());
        if let Err(DomainError::InvalidPresetId(s)) = r {
            assert_eq!(s, "/");
        } else {
            panic!("expected InvalidPresetId, got {r:?}");
        }
    }

    #[test]
    fn preset_id_rejects_missing_leading_slash() {
        let r = PresetId::new("TODO");
        assert!(r.is_err());
        assert!(matches!(r, Err(DomainError::InvalidPresetId(_))));
    }

    #[test]
    fn preset_id_rejects_internal_whitespace() {
        let r = PresetId::new("/TO DO");
        assert!(r.is_err());
        assert!(matches!(r, Err(DomainError::InvalidPresetId(_))));
    }

    #[test]
    fn preset_id_rejects_second_slash() {
        let r = PresetId::new("/foo/bar");
        assert!(r.is_err());
        assert!(matches!(r, Err(DomainError::InvalidPresetId(_))));
    }

    #[test]
    fn preset_id_as_ref_str() {
        let id = PresetId::new("/Video").unwrap();
        assert_eq!(<PresetId as AsRef<str>>::as_ref(&id), "Video");
    }

    #[test]
    fn preset_id_serde_roundtrip() {
        let id = PresetId::new("/Scheduled").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"/Scheduled\"");
        let restored: PresetId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn preset_id_display() {
        let id = PresetId::new("/Deadline").unwrap();
        assert_eq!(format!("{}", id), "/Deadline");
    }

    // ── PresetArgKind serde ──────────────────────────────────────────────────

    #[test]
    fn preset_arg_kind_serialize_lowercase() {
        assert_eq!(serde_json::to_string(&PresetArgKind::Date).unwrap(), "\"date\"");
        assert_eq!(serde_json::to_string(&PresetArgKind::Url).unwrap(), "\"url\"");
        assert_eq!(serde_json::to_string(&PresetArgKind::Text).unwrap(), "\"text\"");
    }

    #[test]
    fn preset_arg_kind_roundtrip() {
        for kind in [PresetArgKind::Date, PresetArgKind::Url, PresetArgKind::Text] {
            let json = serde_json::to_string(&kind).unwrap();
            let restored: PresetArgKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, restored);
        }
    }

    // ── PresetArg serde ─────────────────────────────────────────────────────

    #[test]
    fn preset_arg_date_roundtrip() {
        use chrono::NaiveDate;
        let arg = PresetArg::Date(NaiveDate::from_ymd_opt(2026, 12, 25).unwrap());
        let json = serde_json::to_string(&arg).unwrap();
        assert!(json.contains("\"date\""));
        assert!(json.contains("2026-12-25"));
        let restored: PresetArg = serde_json::from_str(&json).unwrap();
        assert_eq!(arg, restored);
    }

    #[test]
    fn preset_arg_url_roundtrip() {
        let arg = PresetArg::Url(url::Url::parse("https://example.com/video.mp4").unwrap());
        let json = serde_json::to_string(&arg).unwrap();
        assert!(json.contains("\"url\""));
        assert!(json.contains("https://example.com/video.mp4"));
        let restored: PresetArg = serde_json::from_str(&json).unwrap();
        assert_eq!(arg, restored);
    }

    #[test]
    fn preset_arg_text_roundtrip() {
        let arg = PresetArg::Text("hello world".into());
        let json = serde_json::to_string(&arg).unwrap();
        // Format: {"text": "hello world"} (variant name as key, value directly)
        assert_eq!(json, "{\"text\":\"hello world\"}");
        let restored: PresetArg = serde_json::from_str(&json).unwrap();
        assert_eq!(arg, restored);
    }

    // ── PresetArgs ──────────────────────────────────────────────────────────

    #[test]
    fn preset_args_empty() {
        let args = PresetArgs::empty();
        assert!(args.is_empty());
        assert_eq!(args.len(), 0);
    }

    #[test]
    fn preset_args_from_vec_ok() {
        use chrono::NaiveDate;
        let args = PresetArgs::from_vec(vec![
            PresetArg::Date(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            PresetArg::Text("note".into()),
        ])
        .unwrap();
        assert!(!args.is_empty());
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn preset_args_rejects_duplicate_kind() {
        let args = PresetArgs::from_vec(vec![
            PresetArg::Text("first".into()),
            PresetArg::Text("second".into()),
        ]);
        assert!(matches!(args, Err(DomainError::DuplicatePresetArgKind(PresetArgKind::Text))));
    }

    #[test]
    fn preset_args_get_returns_first_match() {
        use chrono::NaiveDate;
        let args = PresetArgs::from_vec(vec![
            PresetArg::Date(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            PresetArg::Text("note".into()),
            PresetArg::Url(url::Url::parse("https://example.com").unwrap()),
        ])
        .unwrap();
        // Should get the Date
        let date = args.get(PresetArgKind::Date);
        assert!(date.is_some());
        let date = date.unwrap();
        match date {
            PresetArg::Date(d) => assert_eq!(d.to_string(), "2026-06-15"),
            _ => panic!("expected Date"),
        }
    }

    #[test]
    fn preset_args_get_missing() {
        use chrono::NaiveDate;
        let args = PresetArgs::from_vec(vec![PresetArg::Date(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap())])
            .unwrap();
        assert!(args.get(PresetArgKind::Url).is_none());
    }

    #[test]
    fn preset_args_iter() {
        use chrono::NaiveDate;
        let args = PresetArgs::from_vec(vec![
            PresetArg::Date(NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()),
            PresetArg::Text("a".into()),
        ])
        .unwrap();
        let kinds: Vec<_> = args.iter().map(|a| match a {
            PresetArg::Date(_) => "date",
            PresetArg::Text(_) => "text",
            PresetArg::Url(_) => "url",
        }).collect();
        assert_eq!(kinds, vec!["date", "text"]);
    }

    #[test]
    fn preset_args_into_iter() {
        let args = PresetArgs::from_vec(vec![PresetArg::Text("x".into())]).unwrap();
        let count = args.into_iter().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn preset_args_default_is_empty() {
        let args = PresetArgs::default();
        assert!(args.is_empty());
    }

    // ── PropertyPreset ───────────────────────────────────────────────────────

    #[test]
    fn property_preset_new_ok() {
        let preset = PropertyPreset::new(
            PresetId::new("/TODO").unwrap(),
            vec![explicit_patch("type", "task")],
            PresetArgs::empty(),
            "A test preset",
        )
        .unwrap();
        assert_eq!(preset.id.as_str(), "TODO");
        assert_eq!(preset.description, "A test preset");
        assert_eq!(preset.patches.len(), 1);
    }

    #[test]
    fn property_preset_rejects_forbidden_key_content() {
        let result = PropertyPreset::new(
            PresetId::new("/Bad").unwrap(),
            vec![explicit_patch("content", "nope")],
            PresetArgs::empty(),
            "bad",
        );
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "content"));
    }

    #[test]
    fn property_preset_rejects_forbidden_key_text() {
        let result = PropertyPreset::new(
            PresetId::new("/Bad").unwrap(),
            vec![explicit_patch("text", "nope")],
            PresetArgs::empty(),
            "bad",
        );
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "text"));
    }

    #[test]
    fn property_preset_rejects_forbidden_key_children() {
        let result = PropertyPreset::new(
            PresetId::new("/Bad").unwrap(),
            vec![explicit_patch("children", "nope")],
            PresetArgs::empty(),
            "bad",
        );
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "children"));
    }

    #[test]
    fn property_preset_with_description() {
        let preset = PropertyPreset::new(
            PresetId::new("/TODO").unwrap(),
            vec![explicit_patch("type", "task")],
            PresetArgs::empty(),
            "original",
        )
        .unwrap()
        .with_description("updated");
        assert_eq!(preset.description, "updated");
    }

    #[test]
    fn property_preset_serde_roundtrip() {
        let preset = PropertyPreset::new(
            PresetId::new("/Video").unwrap(),
            vec![
                explicit_patch("type", "media"),
                explicit_patch("media-type", "video"),
            ],
            PresetArgs::empty(),
            "Video preset",
        )
        .unwrap();
        let json = serde_json::to_string(&preset).unwrap();
        let restored: PropertyPreset = serde_json::from_str(&json).unwrap();
        assert_eq!(preset.id, restored.id);
        assert_eq!(preset.patches.len(), restored.patches.len());
        // description is skip-serialized
        assert_eq!(restored.description, "");
    }

    // ── PropertPatch provenance check ───────────────────────────────────────

    #[test]
    fn preset_patch_has_explicit_provenance() {
        // Per design: all preset patches carry provenance = Explicit
        // (this is enforced at the use-case level, not here)
        let patch = explicit_patch("status", "todo");
        assert_eq!(patch.provenance, PropertyPatchProvenance::Explicit);
    }
}
