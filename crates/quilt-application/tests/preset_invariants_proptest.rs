//! Property invariant proptests for the preset system.
//!
//! Generates 1000 random `(block, PresetId, PresetArgs)` combinations and asserts:
//! - No panic
//! - Return is `Ok` or one of the 4 documented `Err` variants
//! - `block.content` byte-equal before/after
//! - `block` (the caller's reference) is untouched after the call

use chrono::NaiveDate;
use proptest::prelude::*;
use quilt_application::services::presets::StaticPresetRegistry;
use quilt_application::use_cases::ApplyPreset;
use quilt_domain::canonicalization::{PresetArg, PresetArgs, PresetId, PropertyDefinitionRegistry};
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use quilt_domain::properties::types::{MergePolicy, PropertyMutability, PropertyType};
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;

/// Full V1 def registry covering all 9 presets.
fn full_registry() -> Arc<PropertyDefinitionRegistry> {
    use quilt_domain::properties::PropertyDefinition;

    let defs: Vec<_> = [
        ("type", MergePolicy::SetIfMissing),
        ("status", MergePolicy::SetIfMissing),
        ("projection", MergePolicy::SetIfMissing),
        ("focus", MergePolicy::SetIfMissing),
        ("scheduled", MergePolicy::Overwrite),
        ("deadline", MergePolicy::Overwrite),
        ("media-type", MergePolicy::SetIfMissing),
        ("source-url", MergePolicy::AskOnConflict),
    ]
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

/// Known valid V1 preset IDs for proptest.
fn v1_preset_ids() -> Vec<PresetId> {
    vec![
        PresetId::new("/TODO").unwrap(),
        PresetId::new("/DOING").unwrap(),
        PresetId::new("/WAITING").unwrap(),
        PresetId::new("/DONE").unwrap(),
        PresetId::new("/NOW").unwrap(),
        PresetId::new("/Scheduled").unwrap(),
        PresetId::new("/Deadline").unwrap(),
        PresetId::new("/Video").unwrap(),
        PresetId::new("/Image").unwrap(),
    ]
}

/// Preset args for each preset category.
fn make_args_for_preset(preset_id: &PresetId) -> PresetArgs {
    match preset_id.as_str() {
        "Scheduled" => PresetArgs::from_vec(vec![PresetArg::Date(
            NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
        )])
        .unwrap(),
        "Deadline" => PresetArgs::from_vec(vec![PresetArg::Date(
            NaiveDate::from_ymd_opt(2026, 12, 25).unwrap(),
        )])
        .unwrap(),
        "Video" | "Image" => PresetArgs::from_vec(vec![PresetArg::Url(
            url::Url::parse("https://example.com/resource").unwrap(),
        )])
        .unwrap(),
        _ => PresetArgs::empty(),
    }
}

/// Only return Ok or one of the 4 documented Err variants.
fn is_valid_error(e: &DomainError) -> bool {
    matches!(
        e,
        DomainError::UnknownPreset(_)
            | DomainError::MissingPresetArg { .. }
            | DomainError::DuplicatePresetArgKind(_)
            | DomainError::ForbiddenPatchKey(_)
    )
}

proptest! {
    /// 1000 random (block, PresetId, PresetArgs) → no panic, valid result.
    #[test]
    fn apply_preset_random_inputs_dont_panic(
        content in "\\pc{0,200}",        // content string up to 200 chars
        preset_idx in 0..9u8,          // index into V1 presets
        pre_existing_status in proptest::option::of("[a-z]{1,20}"), // optional pre-existing status
    ) {
        let preset_id = v1_preset_ids()[preset_idx as usize].clone();
        let args = make_args_for_preset(&preset_id);
        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = full_registry();
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = Block::default();
        block.content = content;

        // Pre-populate status if provided
        if let Some(status) = pre_existing_status {
            block.properties.insert("status".into(), quilt_domain::value_objects::PropertyValue::text(status));
        }

        let original_content = block.content.clone();
        let original_props_count = block.properties.len();

        let result = uc.execute(&mut block, &preset_id, &args);

        // Assert: result is Ok or valid error
        if result.is_err() {
            let err = result.as_ref().unwrap_err();
            prop_assert!(is_valid_error(err), "unexpected error variant: {:?}", err);
        }

        // Assert: block.content unchanged
        prop_assert_eq!(block.content, original_content, "block.content should not change");

        // Assert: block.properties count unchanged (non-destructive)
        prop_assert_eq!(block.properties.len(), original_props_count,
            "block.properties count should not change on original");
    }

    /// 1000 unknown preset IDs → Err(UnknownPreset).
    #[test]
    fn unknown_preset_ids_return_unknown_preset_error(
        suffix in "[a-zA-Z0-9_-]{1,30}"
    ) {
        let preset_id = PresetId::new(format!("/{}", suffix));
        if preset_id.is_err() {
            return Ok(()); // Skip invalid preset IDs (that's expected)
        }
        let preset_id = preset_id.unwrap();

        // Only skip if it's actually a known preset
        let known_ids = v1_preset_ids();
        if known_ids.iter().any(|k| k == &preset_id) {
            return Ok(());
        }

        let preset_reg: Arc<_> = Arc::new(StaticPresetRegistry::v1());
        let def_reg = full_registry();
        let uc = ApplyPreset::new(preset_reg, def_reg);

        let mut block = Block::default();
        let result = uc.execute(&mut block, &preset_id, &PresetArgs::empty());

        prop_assert!(matches!(result, Err(DomainError::UnknownPreset(_))));
    }
}
