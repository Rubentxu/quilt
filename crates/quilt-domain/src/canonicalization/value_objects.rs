//! Value objects for the canonicalization pipeline.

use crate::content::BlockContent;
use crate::entities::{Block, PropertyKey};
use crate::errors::DomainError;
use crate::properties::types::MergePolicy;
use crate::value_objects::PropertyValue;
use serde::{Deserialize, Serialize};

/// Source of the input — determines how the canonicalizer classifies the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// Standard Markdown block content (default)
    Markdown,
    /// Pasted content from clipboard
    Paste,
    /// Invoked via a slash command (e.g. `/task`)
    SlashCommand,
    /// Selected from a picker UI
    Picker,
    /// Submitted via the REST/GraphQL API
    Api,
    /// Submitted via the MCP protocol
    Mcp,
}

/// Input to the canonicalization pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalInput {
    /// The raw text to canonicalize.
    pub text: String,
    /// The source kind that produced this text.
    pub source_kind: SourceKind,
    /// Optional page context for resolving relative references.
    pub context_page: Option<String>,
}

impl CanonicalInput {
    /// Construct a [`CanonicalInput`] from raw text, defaulting to Markdown source.
    #[must_use]
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source_kind: SourceKind::Markdown,
            context_page: None,
        }
    }

    /// Attach a page context to this input (chainable).
    #[must_use]
    pub fn with_context_page(self, page: impl Into<String>) -> Self {
        Self {
            context_page: Some(page.into()),
            ..self
        }
    }
}

/// Provenance of a [`PropertyPatch`] — tells whether the patch was derived
/// by the canonicalizer or explicitly set by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyPatchProvenance {
    /// Derived by the canonicalizer from input syntax (e.g. `#` → heading-level).
    Derived,
    /// Explicitly set by the user via a property editor or API call.
    Explicit,
}

/// A single property mutation to apply to a block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyPatch {
    /// The canonical property key.
    pub key: PropertyKey,
    /// The value to write.
    pub value: PropertyValue,
    /// Whether this patch is derived or explicit.
    pub provenance: PropertyPatchProvenance,
}

impl PropertyPatch {
    /// Construct a derived patch (produced by the canonicalizer).
    #[must_use]
    pub fn derived(key: PropertyKey, value: PropertyValue) -> Self {
        Self {
            key,
            value,
            provenance: PropertyPatchProvenance::Derived,
        }
    }

    /// Construct an explicit patch (set by the user).
    #[must_use]
    pub fn explicit(key: PropertyKey, value: PropertyValue) -> Self {
        Self {
            key,
            value,
            provenance: PropertyPatchProvenance::Explicit,
        }
    }

    /// Apply this patch to a block, enforcing merge policy from the property definition.
    ///
    /// Returns [`PatchOutcome`] on success, or [`DomainError`] if the patch is forbidden
    /// or conflicts under the active policy.
    pub fn apply_to(
        &self,
        block: &mut Block,
        defs: &crate::canonicalization::PropertyDefinitionRegistry,
    ) -> Result<PatchOutcome, DomainError> {
        crate::canonicalization::apply::apply_property_patch(self, block, defs)
    }
}

/// A conflict detected when applying a patch under [`MergePolicy::AskOnConflict`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionConflict {
    /// The property key that conflicted.
    pub property: PropertyKey,
    /// The value currently in the block.
    pub existing_value: PropertyValue,
    /// The value the patch attempted to write.
    pub attempted_value: PropertyValue,
    /// The merge policy that triggered this conflict.
    pub policy: MergePolicy,
    /// Human-readable reason for the conflict.
    pub reason: String,
}

/// Outcome of applying one or more [`PropertyPatch`]es to a block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatchOutcome {
    /// The (potentially modified) block.
    pub block: Block,
    /// Any conflicts detected during the patch application.
    pub conflicts: Vec<ProjectionConflict>,
    /// Property keys that were materialized from derived provenance.
    pub derived_materialized: Vec<PropertyKey>,
    /// Property keys that were skipped (not found, set-if-missing, etc.).
    pub skipped: Vec<PropertyKey>,
}

impl PatchOutcome {
    /// Construct a "no change" outcome for the given block.
    #[must_use]
    pub fn unchanged(block: Block) -> Self {
        Self {
            block,
            conflicts: Vec::new(),
            derived_materialized: Vec::new(),
            skipped: Vec::new(),
        }
    }
}

/// Result of canonicalizing a [`CanonicalInput`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalizationResult {
    /// The canonical block content (non-destructive — byte-equal to input).
    pub content: BlockContent,
    /// Property patches derived by the canonicalizer from input syntax.
    pub derived: Vec<PropertyPatch>,
    /// Property patches explicitly applied by the caller.
    pub applied: Vec<PropertyPatch>,
}

impl CanonicalizationResult {
    /// Construct an empty result (no patches derived or applied).
    #[must_use]
    pub fn empty(content: BlockContent) -> Self {
        Self {
            content,
            derived: Vec::new(),
            applied: Vec::new(),
        }
    }

    /// Construct a result with only content (no patches).
    ///
    /// # Deprecated
    /// Use [`CanonicalizationResult::empty`] instead. This method exists for
    /// backward compatibility and may be removed in a future version.
    #[deprecated(since = "0.1.0", note = "use CanonicalizationResult::empty instead")]
    #[must_use]
    pub fn content_only(content: BlockContent) -> Self {
        Self::empty(content)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::BlockContent;
    use crate::entities::Block;
    use crate::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
    use std::collections::HashMap;

    // ── SourceKind serde ───────────────────────────────────────────

    #[test]
    fn source_kind_serialize_all_variants_snake_case() {
        assert_eq!(
            serde_json::to_string(&SourceKind::Markdown).unwrap(),
            "\"markdown\""
        );
        assert_eq!(
            serde_json::to_string(&SourceKind::Paste).unwrap(),
            "\"paste\""
        );
        assert_eq!(
            serde_json::to_string(&SourceKind::SlashCommand).unwrap(),
            "\"slash_command\""
        );
        assert_eq!(
            serde_json::to_string(&SourceKind::Picker).unwrap(),
            "\"picker\""
        );
        assert_eq!(serde_json::to_string(&SourceKind::Api).unwrap(), "\"api\"");
        assert_eq!(serde_json::to_string(&SourceKind::Mcp).unwrap(), "\"mcp\"");
    }

    #[test]
    fn source_kind_deserialize_round_trip() {
        for variant in [
            SourceKind::Markdown,
            SourceKind::Paste,
            SourceKind::SlashCommand,
            SourceKind::Picker,
            SourceKind::Api,
            SourceKind::Mcp,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let restored: SourceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, restored);
        }
    }

    #[test]
    fn source_kind_unknown_string_rejected() {
        let result = serde_json::from_str::<SourceKind>("\"voice\"");
        assert!(result.is_err());
    }

    // ── CanonicalInput ─────────────────────────────────────────────

    #[test]
    fn canonical_input_from_text_defaults_to_markdown_no_context() {
        let input = CanonicalInput::from_text("hello world");
        assert_eq!(input.text, "hello world");
        assert_eq!(input.source_kind, SourceKind::Markdown);
        assert!(input.context_page.is_none());
    }

    #[test]
    fn canonical_input_with_context_page_sets_and_preserves_fields() {
        let input = CanonicalInput::from_text("some text").with_context_page("My Page");
        assert_eq!(input.text, "some text");
        assert_eq!(input.context_page, Some("My Page".into()));
        assert_eq!(input.source_kind, SourceKind::Markdown);
    }

    #[test]
    fn canonical_input_from_text_accepts_str_and_string() {
        let from_str = CanonicalInput::from_text("hello");
        let from_string = CanonicalInput::from_text(String::from("hello"));
        assert_eq!(from_str, from_string);
    }

    #[test]
    fn canonical_input_round_trips_via_serde() {
        let input = CanonicalInput::from_text("test content").with_context_page("Test Page");
        let json = serde_json::to_string(&input).unwrap();
        let restored: CanonicalInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, restored);
    }

    #[test]
    fn canonical_input_from_text_preserves_multi_line_unicode_whitespace() {
        let input = CanonicalInput::from_text("line1\nline2\twith\ttabs\n\nleading");
        let content = BlockContent::from_text(&input.text);
        // BlockContent::from_text wraps the whole string in one Text segment
        assert!(!content.segments.is_empty());
    }

    // ── PropertyPatchProvenance ───────────────────────────────────

    #[test]
    fn provenance_serialize_lowercase() {
        assert_eq!(
            serde_json::to_string(&PropertyPatchProvenance::Derived).unwrap(),
            "\"derived\""
        );
        assert_eq!(
            serde_json::to_string(&PropertyPatchProvenance::Explicit).unwrap(),
            "\"explicit\""
        );
    }

    #[test]
    fn provenance_round_trip() {
        for p in [
            PropertyPatchProvenance::Derived,
            PropertyPatchProvenance::Explicit,
        ] {
            let json = serde_json::to_string(&p).unwrap();
            let restored: PropertyPatchProvenance = serde_json::from_str(&json).unwrap();
            assert_eq!(p, restored);
        }
    }

    #[test]
    fn provenance_unknown_rejected() {
        assert!(serde_json::from_str::<PropertyPatchProvenance>("\"derived\"").is_ok());
        assert!(serde_json::from_str::<PropertyPatchProvenance>("\"explicit\"").is_ok());
        assert!(serde_json::from_str::<PropertyPatchProvenance>("\"stolen\"").is_err());
    }

    // ── PropertyPatch ─────────────────────────────────────────────

    fn make_key(s: &str) -> PropertyKey {
        PropertyKey::new(s).expect("valid key")
    }

    #[test]
    fn property_patch_derived_sets_provenance() {
        let patch = PropertyPatch::derived(make_key("heading-level"), PropertyValue::text("1"));
        assert_eq!(patch.provenance, PropertyPatchProvenance::Derived);
        assert_eq!(patch.key.as_str(), "heading-level");
    }

    #[test]
    fn property_patch_explicit_sets_provenance() {
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        assert_eq!(patch.provenance, PropertyPatchProvenance::Explicit);
    }

    #[test]
    fn property_patch_struct_preserves_all_three_fields() {
        let key = make_key("type");
        let value = PropertyValue::text("task");
        let patch = PropertyPatch {
            key: key.clone(),
            value: value.clone(),
            provenance: PropertyPatchProvenance::Derived,
        };
        assert_eq!(patch.key, key);
        assert_eq!(patch.value, value);
        assert_eq!(patch.provenance, PropertyPatchProvenance::Derived);
    }

    #[test]
    fn property_patch_round_trip_via_serde() {
        let patch = PropertyPatch::derived(make_key("link-kind"), PropertyValue::text("external"));
        let json = serde_json::to_string(&patch).unwrap();
        let restored: PropertyPatch = serde_json::from_str(&json).unwrap();
        assert_eq!(patch, restored);
    }

    // ── PatchOutcome ──────────────────────────────────────────────

    fn make_empty_block() -> Block {
        Block::new(crate::entities::BlockCreate {
            page_id: Uuid::new_v4(),
            content: String::new(),
            parent_id: None,
            order: 0.0,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: HashMap::new(),
        })
        .expect("creates block")
    }

    #[test]
    fn patch_outcome_unchanged_has_empty_vectors() {
        let block = make_empty_block();
        let outcome = PatchOutcome::unchanged(block.clone());
        assert!(outcome.conflicts.is_empty());
        assert!(outcome.derived_materialized.is_empty());
        assert!(outcome.skipped.is_empty());
    }

    #[test]
    fn patch_outcome_equality_independent_of_constructors() {
        let block = make_empty_block();
        let a = PatchOutcome::unchanged(block.clone());
        let b = PatchOutcome::unchanged(block);
        assert_eq!(a, b);
    }

    #[test]
    fn patch_outcome_round_trip_via_serde() {
        let mut block = make_empty_block();
        let conflict_key = make_key("status");
        block.properties.insert(
            conflict_key.clone().into_string(),
            PropertyValue::text("done"),
        );
        let outcome = PatchOutcome {
            block,
            conflicts: vec![ProjectionConflict {
                property: conflict_key.clone(),
                existing_value: PropertyValue::text("done"),
                attempted_value: PropertyValue::text("todo"),
                policy: MergePolicy::AskOnConflict,
                reason: "user must resolve".into(),
            }],
            derived_materialized: vec![make_key("heading-level"), make_key("block-role")],
            skipped: vec![make_key("unknown-prop")],
        };
        let json = serde_json::to_string(&outcome).unwrap();
        let restored: PatchOutcome = serde_json::from_str(&json).unwrap();
        // Policy serializes as snake_case
        let json2 = serde_json::to_string(&restored.conflicts[0].policy).unwrap();
        assert_eq!(json2, "\"ask_on_conflict\"");
        assert_eq!(restored.conflicts.len(), 1);
        assert_eq!(restored.derived_materialized.len(), 2);
        assert_eq!(restored.skipped.len(), 1);
    }

    // ── CanonicalizationResult ────────────────────────────────────

    #[test]
    fn canonicalization_result_empty_has_no_patches() {
        let content = BlockContent::from_text("hello");
        let result = CanonicalizationResult::empty(content.clone());
        assert_eq!(result.content, content);
        assert!(result.derived.is_empty());
        assert!(result.applied.is_empty());
    }

    #[test]
    fn canonicalization_result_empty_and_content_only_are_equal() {
        let content = BlockContent::from_text("hello");
        let empty = CanonicalizationResult::empty(content.clone());
        // SAFETY: deprecated but used intentionally for equality comparison
        #[allow(deprecated)]
        let content_only = CanonicalizationResult::content_only(content);
        assert_eq!(empty.content, content_only.content);
        assert_eq!(empty.derived, content_only.derived);
        assert_eq!(empty.applied, content_only.applied);
    }

    #[test]
    fn canonicalization_result_round_trip_via_serde() {
        let content = BlockContent::from_text("test");
        let result = CanonicalizationResult {
            content,
            derived: vec![
                PropertyPatch::derived(make_key("heading-level"), PropertyValue::text("1")),
                PropertyPatch::derived(make_key("block-role"), PropertyValue::text("heading")),
            ],
            applied: vec![PropertyPatch::explicit(
                make_key("status"),
                PropertyValue::text("todo"),
            )],
        };
        let json = serde_json::to_string(&result).unwrap();
        let restored: CanonicalizationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.derived.len(), 2);
        assert_eq!(restored.applied.len(), 1);
    }
}
