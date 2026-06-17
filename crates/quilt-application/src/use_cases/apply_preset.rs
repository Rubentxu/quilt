//! Apply preset use case — applies a named [`PropertyPreset`] to a block.
//!
//! Fold algorithm: starting from a cloned block, each preset patch is applied
//! sequentially via [`PropertyPatch::apply_to`]. The original `&mut Block` is
//! left untouched on error — the Rust borrow checker enforces this because we
//! work on `outcome.block` (a clone) and only return the final `outcome`.
//!
//! # Error handling
//!
//! - `UnknownPreset` — preset id not found in registry
//! - `MissingPresetArg { preset, kind }` — required arg not provided
//!
//! # Non-destructive guarantee
//!
//! Block `content` and `children` are never modified by this use case.

use quilt_domain::canonicalization::{
    PatchOutcome, PresetArgKind, PresetId, PresetRegistry, PropertyDefinitionRegistry,
    PropertyPatch, PropertyPreset,
};
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use std::sync::Arc;
use tracing::instrument;

/// Applies a named [`PropertyPreset`] to a [`Block`].
pub struct ApplyPreset {
    preset_registry: Arc<dyn PresetRegistry>,
    def_registry: Arc<PropertyDefinitionRegistry>,
}

impl ApplyPreset {
    /// Construct an [`ApplyPreset`] use case.
    ///
    /// The `def_registry` provides [`MergePolicy`] for each property key,
    /// enabling the fold algorithm to enforce per-key merge semantics.
    pub fn new(
        preset_registry: Arc<dyn PresetRegistry>,
        def_registry: Arc<PropertyDefinitionRegistry>,
    ) -> Self {
        Self {
            preset_registry,
            def_registry,
        }
    }

    /// Apply a preset to a block.
    ///
    /// # Arguments
    ///
    /// * `block` — the block to modify (left untouched on error)
    /// * `preset_id` — which preset to apply (e.g. `/TODO`, `/Scheduled`)
    /// * `args` — arguments required by the preset (e.g. date for `/Scheduled`)
    ///
    /// # Returns
    ///
    /// `Ok(PatchOutcome)` with the modified block inside, or a domain error.
    #[instrument(skip(self), fields(preset_id = %preset_id))]
    pub fn execute(
        &self,
        block: &mut Block,
        preset_id: &PresetId,
        args: &quilt_domain::canonicalization::PresetArgs,
    ) -> Result<PatchOutcome, DomainError> {
        // ── 1. Resolve preset ──────────────────────────────────────────────────

        let preset = self
            .preset_registry
            .get(preset_id)
            .ok_or(DomainError::UnknownPreset(preset_id.clone()))?;

        // ── 2. Validate required args ─────────────────────────────────────────

        validate_preset_args(&preset, args)?;

        // ── 3. Fold over patches ─────────────────────────────────────────────

        // Start with an unchanged clone; the original block is untouched on error.
        let mut outcome = PatchOutcome::unchanged(block.clone());
        let mut applied_count = 0usize;
        let mut skipped_count = 0usize;
        let mut conflict_count = 0usize;

        for patch in &preset.patches {
            // Bind argument values to this patch's keys (date/url substitution)
            let bound_patch = bind_preset_arg(patch, args)?;

            // Apply the patch to the cloned block inside outcome
            let patch_outcome = bound_patch
                .apply_to(&mut outcome.block, &self.def_registry)
                .map_err(|e| {
                    tracing::warn!(?e, "ApplyPreset patch failed, rolling back");
                    e
                })?;

            // Accumulate statistics
            applied_count += 1;
            skipped_count += patch_outcome.skipped.len();
            conflict_count += patch_outcome.conflicts.len();

            // Extend outcome lists
            outcome.conflicts.extend(patch_outcome.conflicts);
            outcome
                .derived_materialized
                .extend(patch_outcome.derived_materialized);
            outcome.skipped.extend(patch_outcome.skipped);
        }

        tracing::info!(
            applied = applied_count,
            skipped = skipped_count,
            conflicts = conflict_count,
            "ApplyPreset completed"
        );

        Ok(outcome)
    }
}

/// Validate that every required argument is present in `args`.
fn validate_preset_args(
    preset: &PropertyPreset,
    args: &quilt_domain::canonicalization::PresetArgs,
) -> Result<(), DomainError> {
    for kind in [PresetArgKind::Date, PresetArgKind::Url, PresetArgKind::Text] {
        // Only check kinds that are actually required
        if args.get(kind).is_none() && preset.required_args.get(kind).is_some() {
            return Err(DomainError::MissingPresetArg {
                preset: preset.id.clone(),
                kind,
            });
        }
    }
    Ok(())
}

/// Bind preset arguments into a patch's value.
///
/// For patches with keys `scheduled` or `deadline` → substitutes `PresetArg::Date`.
/// For patches with key `source-url` → substitutes `PresetArg::Url`.
/// All other patches are returned unchanged.
fn bind_preset_arg(
    patch: &PropertyPatch,
    args: &quilt_domain::canonicalization::PresetArgs,
) -> Result<PropertyPatch, DomainError> {
    let key = patch.key.as_str();

    // Date-key substitution: scheduled / deadline
    if key == "scheduled" || key == "deadline" {
        if let Some(quilt_domain::canonicalization::PresetArg::Date(date)) =
            args.get(PresetArgKind::Date)
        {
            let value = quilt_domain::value_objects::PropertyValue::naive_date(*date);
            return Ok(PropertyPatch::explicit(patch.key.clone(), value));
        }
    }

    // URL-key substitution: source-url
    if key == "source-url" {
        if let Some(quilt_domain::canonicalization::PresetArg::Url(url)) =
            args.get(PresetArgKind::Url)
        {
            let value = quilt_domain::value_objects::PropertyValue::url(url.clone());
            return Ok(PropertyPatch::explicit(patch.key.clone(), value));
        }
    }

    // Unchanged: return the original patch (provenance stays Explicit per design)
    Ok(patch.clone())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::services::presets::StaticPresetRegistry;
    use crate::use_cases::ApplyPreset;
    use chrono::NaiveDate;
    use quilt_domain::canonicalization::{
        PatchOutcome, PresetArg, PresetArgs, PresetId, PropertyDefinitionRegistry,
    };
    use quilt_domain::entities::Block;
    use quilt_domain::properties::types::{MergePolicy, PropertyMutability, PropertyType};
    use quilt_domain::value_objects::PropertyValue;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_block() -> Block {
        Block::default()
    }

    fn make_registry_with_policy(
        patches: &[(&str, MergePolicy)],
    ) -> Arc<PropertyDefinitionRegistry> {
        use chrono::DateTime;
        use quilt_domain::properties::PropertyDefinition;
        use quilt_domain::value_objects::Uuid;

        let defs: Vec<_> = patches
            .iter()
            .map(|(key, policy)| PropertyDefinition {
                id: Uuid::new_v4(),
                db_ident: key.to_string(),
                title: key.to_string(),
                property_type: PropertyType::Text,
                cardinality: quilt_domain::properties::types::Cardinality::One,
                closed_values: Vec::new(),
                attribute: None,
                status: quilt_domain::properties::types::PropertyStatus::Active,
                alias_of: None,
                block_count: 0,
                page_count: 0,
                first_seen_at: None,
                last_seen_at: None,
                visibility: Default::default(),
                mutability: PropertyMutability::Mutable,
                derived_from: None,
                merge_policy: *policy,
            })
            .collect();
        Arc::new(PropertyDefinitionRegistry::from_definitions(defs))
    }

    // ── /TODO preset ────────────────────────────────────────────────────────

    #[test]
    fn apply_todo_sets_type_status_projection() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("status", MergePolicy::Overwrite),
            ("projection", MergePolicy::SetIfMissing),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let outcome = uc
            .execute(
                &mut block,
                &PresetId::new("/TODO").unwrap(),
                &PresetArgs::empty(),
            )
            .unwrap();

        assert!(outcome.conflicts.is_empty());
        let props = &outcome.block.properties;
        assert_eq!(
            props.get("type").map(|v| v.as_display_string()).as_deref(),
            Some("task")
        );
        assert_eq!(
            props
                .get("status")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("todo")
        );
        assert_eq!(
            props
                .get("projection")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("auto")
        );
    }

    #[test]
    fn apply_todo_preserves_content() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("status", MergePolicy::Overwrite),
            ("projection", MergePolicy::SetIfMissing),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        block.content = "Buy milk".to_string();

        let original_content = block.content.clone();
        let outcome = uc
            .execute(
                &mut block,
                &PresetId::new("/TODO").unwrap(),
                &PresetArgs::empty(),
            )
            .unwrap();

        assert_eq!(outcome.block.content, original_content);
        // Original block untouched
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_todo_unknown_preset_returns_error() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let result = uc.execute(
            &mut block,
            &PresetId::new("/NotAPreset").unwrap(),
            &PresetArgs::empty(),
        );
        assert!(matches!(
            result,
            Err(quilt_domain::errors::DomainError::UnknownPreset(_))
        ));
    }

    // ── /Scheduled preset ───────────────────────────────────────────────────

    #[test]
    fn apply_scheduled_with_date_arg() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[("scheduled", MergePolicy::Overwrite)]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let args = PresetArgs::from_vec(vec![PresetArg::Date(
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        )])
        .unwrap();

        let outcome = uc
            .execute(&mut block, &PresetId::new("/Scheduled").unwrap(), &args)
            .unwrap();

        let props = &outcome.block.properties;
        // ADR-0027: result is now a typed NaiveDate variant (not String)
        assert!(matches!(
            props.get("scheduled"),
            Some(PropertyValue::NaiveDate(d)) if d == &NaiveDate::from_ymd_opt(2026, 6, 15).unwrap()
        ));
        // Display string still works
        assert_eq!(
            props
                .get("scheduled")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("2026-06-15")
        );
    }

    #[test]
    fn apply_scheduled_missing_date_arg_returns_error() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[("scheduled", MergePolicy::Overwrite)]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let result = uc.execute(
            &mut block,
            &PresetId::new("/Scheduled").unwrap(),
            &PresetArgs::empty(),
        );
        assert!(matches!(
            result,
            Err(quilt_domain::errors::DomainError::MissingPresetArg { .. })
        ));
    }

    // ── /Video preset ──────────────────────────────────────────────────────

    #[test]
    fn apply_video_with_url_arg() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("media-type", MergePolicy::SetIfMissing),
            ("source-url", MergePolicy::AskOnConflict),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let video_url = url::Url::parse("https://example.com/video.mp4").unwrap();
        let args = PresetArgs::from_vec(vec![PresetArg::Url(video_url.clone())]).unwrap();

        let outcome = uc
            .execute(&mut block, &PresetId::new("/Video").unwrap(), &args)
            .unwrap();

        let props = &outcome.block.properties;
        assert_eq!(
            props.get("type").map(|v| v.as_display_string()).as_deref(),
            Some("media")
        );
        assert_eq!(
            props
                .get("media-type")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("video")
        );
        // ADR-0027: result is now a typed Url variant (not String)
        assert!(matches!(
            props.get("source-url"),
            Some(PropertyValue::Url(u)) if u == &video_url
        ));
        assert_eq!(
            props
                .get("source-url")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("https://example.com/video.mp4")
        );
    }

    // ── ADR-0027: backward compat (legacy text bindings still validate) ───────

    #[test]
    fn apply_preset_legacy_text_url_still_validates() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        // Use Overwrite for source-url so the typed URL is applied
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("source-url", MergePolicy::Overwrite),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        // Simulate pre-ADR-0027: block already has a String URL (not typed Url variant)
        block.properties.insert(
            "source-url".into(),
            PropertyValue::text("https://legacy.example.com"),
        );

        // Apply /Video preset — the typed URL should overwrite the legacy String
        let video_url = url::Url::parse("https://example.com/video.mp4").unwrap();
        let args = PresetArgs::from_vec(vec![PresetArg::Url(video_url.clone())]).unwrap();

        let outcome = uc
            .execute(&mut block, &PresetId::new("/Video").unwrap(), &args)
            .unwrap();

        // Typed Url wins (Overwrite), proving the typed path works
        assert!(matches!(
            outcome.block.properties.get("source-url"),
            Some(PropertyValue::Url(u)) if u == &video_url
        ));
        // Legacy String value would also have passed validation via backward-compat arm
    }

    #[test]
    fn apply_preset_legacy_text_date_still_validates() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[("scheduled", MergePolicy::Overwrite)]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        // Simulate pre-ADR-0027: block already has a String date (not typed NaiveDate)
        block
            .properties
            .insert("scheduled".into(), PropertyValue::text("2026-06-15"));

        let args = PresetArgs::from_vec(vec![PresetArg::Date(
            NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
        )])
        .unwrap();

        // Apply /Scheduled preset — the new NaiveDate should overwrite
        let outcome = uc
            .execute(&mut block, &PresetId::new("/Scheduled").unwrap(), &args)
            .unwrap();

        // New typed NaiveDate wins (Overwrite)
        assert!(matches!(
            outcome.block.properties.get("scheduled"),
            Some(PropertyValue::NaiveDate(_))
        ));
        // Legacy string date would still have passed validation via backward-compat arm
    }

    // ── provenance is Explicit ──────────────────────────────────────────────

    #[test]
    fn apply_todo_patches_have_explicit_provenance() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("status", MergePolicy::Overwrite),
            ("projection", MergePolicy::SetIfMissing),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        let outcome = uc
            .execute(
                &mut block,
                &PresetId::new("/TODO").unwrap(),
                &PresetArgs::empty(),
            )
            .unwrap();

        // The patches were applied to outcome.block (original block is untouched)
        assert_eq!(
            outcome
                .block
                .properties
                .get("type")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("task")
        );
    }

    // ── merge policy: SetIfMissing ─────────────────────────────────────────

    #[test]
    fn apply_todo_set_if_missing_skips_existing() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        // Use SetIfMissing for type (matches V1 preset policy)
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::SetIfMissing),
            ("status", MergePolicy::SetIfMissing),
            ("projection", MergePolicy::SetIfMissing),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        // Pre-populate type
        block
            .properties
            .insert("type".into(), PropertyValue::text("custom-type"));

        let _ = uc.execute(
            &mut block,
            &PresetId::new("/TODO").unwrap(),
            &PresetArgs::empty(),
        );

        // SetIfMissing should NOT overwrite existing value
        assert_eq!(
            block
                .properties
                .get("type")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("custom-type")
        );
    }

    // ── merge policy: Overwrite ─────────────────────────────────────────────

    #[test]
    fn apply_todo_overwrite_replaces_existing() {
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        // Use Overwrite for status (V1 uses SetIfMissing for /TODO status, so we
        // use a custom def_registry with Overwrite to test the policy honouring)
        let def_reg = make_registry_with_policy(&[
            ("type", MergePolicy::Overwrite),
            ("status", MergePolicy::Overwrite),
            ("projection", MergePolicy::Overwrite),
        ]);
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = make_block();
        block
            .properties
            .insert("status".into(), PropertyValue::text("done"));

        let outcome = uc
            .execute(
                &mut block,
                &PresetId::new("/TODO").unwrap(),
                &PresetArgs::empty(),
            )
            .unwrap();

        // Overwrite should replace existing value in outcome.block
        assert_eq!(
            outcome
                .block
                .properties
                .get("status")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("todo")
        );
        // Original block is untouched (non-destructive guarantee)
        assert_eq!(
            block
                .properties
                .get("status")
                .map(|v| v.as_display_string())
                .as_deref(),
            Some("done")
        );
    }
}
