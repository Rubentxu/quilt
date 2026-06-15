//! PropertyPatch application logic — merge policy enforcement.
//!
//! This module is the behavioral heart of the canonicalization slice.
//! It implements [`super::PropertyPatch::apply_to`].

use crate::canonicalization::{PatchOutcome, PropertyDefinitionRegistry, PropertyPatch};
use crate::entities::Block;
use crate::errors::DomainError;
use crate::value_objects::PropertyValue;

/// Apply a single property patch to a block, enforcing merge policy.
///
/// This is the internal implementation called by [`PropertyPatch::apply_to`].
/// Guards (forbidden keys, missing definitions, immutable+explicit) are applied
/// first, then the policy dispatch handles the actual write.
pub fn apply_property_patch(
    patch: &PropertyPatch,
    block: &mut Block,
    defs: &PropertyDefinitionRegistry,
) -> Result<PatchOutcome, DomainError> {
    let key_str = patch.key.as_str();

    // Guard 1: forbidden keys — content, text, children are never writable
    if key_str == "content" || key_str == "text" || key_str == "children" {
        return Err(DomainError::ForbiddenPatchKey(key_str.into()));
    }

    // Guard 2: unknown key → skip with no modification
    let Some(def) = defs.get(key_str) else {
        let mut outcome = PatchOutcome::unchanged(block.clone());
        outcome.skipped.push(patch.key.clone());
        return Ok(outcome);
    };

    // Guard 3: immutable + explicit → error
    if def.mutability == crate::properties::types::PropertyMutability::Immutable
        && patch.provenance == crate::canonicalization::PropertyPatchProvenance::Explicit
    {
        return Err(DomainError::ImmutableProperty(key_str.into()));
    }

    // Policy dispatch — defined in WU-6
    apply_policy(patch, block, &def)
}

/// Apply the merge policy for a patch that passed all guards.
fn apply_policy(
    patch: &PropertyPatch,
    block: &mut Block,
    def: &crate::properties::PropertyDefinition,
) -> Result<PatchOutcome, DomainError> {
    use crate::canonicalization::PropertyPatchProvenance;
    use crate::properties::types::MergePolicy;

    let key_str = patch.key.as_str();
    let key_owned = patch.key.clone();
    let existing = block.properties.get(key_str).cloned();
    let mut outcome = PatchOutcome::unchanged(block.clone());

    // Track derived_materialized when applicable
    let track_derived = |outcome: &mut PatchOutcome| {
        if patch.provenance == PropertyPatchProvenance::Derived {
            outcome.derived_materialized.push(key_owned.clone());
        }
    };

    match def.merge_policy {
        MergePolicy::SetIfMissing => {
            if block.properties.contains_key(key_str) {
                outcome.skipped.push(key_owned);
                return Ok(outcome);
            }
            track_derived(&mut outcome);
            block.properties.insert(key_owned.into_string(), patch.value.clone());
            Ok(outcome)
        }
        MergePolicy::Overwrite => {
            track_derived(&mut outcome);
            block.properties.insert(key_owned.into_string(), patch.value.clone());
            Ok(outcome)
        }
        MergePolicy::Append => {
            apply_append(patch, block, def, &mut outcome)?;
            track_derived(&mut outcome);
            Ok(outcome)
        }
        MergePolicy::Union => {
            apply_union(patch, block, def, &mut outcome)?;
            track_derived(&mut outcome);
            Ok(outcome)
        }
        MergePolicy::RejectOnConflict => {
            apply_reject_on_conflict(patch, block, &mut outcome, existing)
        }
        MergePolicy::AskOnConflict => {
            apply_ask_on_conflict(patch, block, &mut outcome, existing)
        }
    }
}

// ── Policy helpers ────────────────────────────────────────────────────────────

fn apply_append(
    patch: &PropertyPatch,
    block: &mut Block,
    def: &crate::properties::PropertyDefinition,
    _outcome: &mut PatchOutcome,
) -> Result<(), DomainError> {
    use crate::properties::types::Cardinality;
    use crate::value_objects::PropertyValue;

    let key_str = patch.key.as_str();
    let key_owned = patch.key.clone();
    let cardinality = def.cardinality.clone();

    match (&block.properties.get(key_str), cardinality) {
        // No existing value + Many → seed array
        (None, Cardinality::Many) => {
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(vec![patch.value.clone()]));
        }
        // Existing array + Many → push (wrap scalar if needed)
        (Some(PropertyValue::Array(arr)), Cardinality::Many) => {
            let mut new_arr = arr.clone();
            push_dedup(&mut new_arr, &patch.value);
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(new_arr));
        }
        // Scalar + Many → convert to array
        (Some(scalar), Cardinality::Many) => {
            let new_arr = vec![(*scalar).clone(), patch.value.clone()];
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(new_arr));
        }
        // Any + One → overwrite
        (_, Cardinality::One) => {
            block.properties.insert(key_owned.into_string(), patch.value.clone());
        }
    }
    Ok(())
}

fn apply_union(
    patch: &PropertyPatch,
    block: &mut Block,
    def: &crate::properties::PropertyDefinition,
    _outcome: &mut PatchOutcome,
) -> Result<(), DomainError> {
    use crate::properties::types::Cardinality;
    use crate::value_objects::PropertyValue;

    let key_str = patch.key.as_str();
    let key_owned = patch.key.clone();
    let cardinality = def.cardinality.clone();

    match (&block.properties.get(key_str), cardinality) {
        // No existing value + Many → seed array
        (None, Cardinality::Many) => {
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(vec![patch.value.clone()]));
        }
        // Existing array + Many → set union
        (Some(PropertyValue::Array(arr)), Cardinality::Many) => {
            let mut new_arr = arr.clone();
            extend_with_unique(&mut new_arr, &patch.value);
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(new_arr));
        }
        // Scalar + Many → convert to array with union
        (Some(scalar), Cardinality::Many) => {
            let mut new_arr = vec![(*scalar).clone()];
            extend_with_unique(&mut new_arr, &patch.value);
            block.properties.insert(key_owned.into_string(), PropertyValue::Array(new_arr));
        }
        // Any + One → overwrite
        (_, Cardinality::One) => {
            block.properties.insert(key_owned.into_string(), patch.value.clone());
        }
    }
    Ok(())
}

fn apply_reject_on_conflict(
    patch: &PropertyPatch,
    block: &mut Block,
    outcome: &mut PatchOutcome,
    existing: Option<PropertyValue>,
) -> Result<PatchOutcome, DomainError> {
    let key_owned = patch.key.clone();

    if let Some(existing_val) = existing {
        if existing_val == patch.value {
            // No-op: values match → skip
            let mut result = PatchOutcome::unchanged(block.clone());
            result.skipped.push(key_owned);
            return Ok(result);
        }
        // Values differ → reject
        return Err(DomainError::MergeConflict {
            key: key_owned.into_string(),
            existing: existing_val,
            attempted: patch.value.clone(),
        });
    }

    // No existing value → write
    if patch.provenance == crate::canonicalization::PropertyPatchProvenance::Derived {
        outcome.derived_materialized.push(key_owned.clone());
    }
    block.properties.insert(key_owned.into_string(), patch.value.clone());
    Ok(PatchOutcome::unchanged(block.clone()))
}

fn apply_ask_on_conflict(
    patch: &PropertyPatch,
    block: &mut Block,
    outcome: &mut PatchOutcome,
    existing: Option<PropertyValue>,
) -> Result<PatchOutcome, DomainError> {
    use crate::properties::types::MergePolicy;

    let key_owned = patch.key.clone();

    if let Some(existing_val) = existing {
        if existing_val == patch.value {
            // No-op: values match → skip
            let mut result = PatchOutcome::unchanged(block.clone());
            result.skipped.push(key_owned);
            return Ok(result);
        }
        // Values differ → surface conflict, don't modify block
        let conflict = crate::canonicalization::ProjectionConflict {
            property: key_owned.clone(),
            existing_value: existing_val,
            attempted_value: patch.value.clone(),
            policy: MergePolicy::AskOnConflict,
            reason: "conflicting values require user resolution".into(),
        };
        outcome.conflicts.push(conflict);
        return Ok(PatchOutcome {
            block: block.clone(),
            conflicts: std::mem::take(&mut outcome.conflicts),
            derived_materialized: std::mem::take(&mut outcome.derived_materialized),
            skipped: std::mem::take(&mut outcome.skipped),
        });
    }

    // No existing value → write
    if patch.provenance == crate::canonicalization::PropertyPatchProvenance::Derived {
        outcome.derived_materialized.push(key_owned.clone());
    }
    block.properties.insert(key_owned.into_string(), patch.value.clone());
    Ok(PatchOutcome::unchanged(block.clone()))
}

// ── Array helpers ─────────────────────────────────────────────────────────────

/// Push a value into an array, deduplicating by equality.
fn push_dedup(arr: &mut Vec<PropertyValue>, value: &PropertyValue) {
    if !arr.contains(value) {
        arr.push(value.clone());
    }
}

/// Extend an array with unique values from another value (scalar or array).
fn extend_with_unique(target: &mut Vec<PropertyValue>, value: &PropertyValue) {
    if let PropertyValue::Array(items) = value {
        for item in items {
            if !target.contains(item) {
                target.push(item.clone());
            }
        }
    } else if !target.contains(value) {
        target.push(value.clone());
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonicalization::{CanonicalInput, PropertyPatchProvenance, PropertyPatch};
    use crate::entities::{Block, BlockCreate};
    use crate::properties::types::{Cardinality, MergePolicy, PropertyMutability, PropertyType, PropertyVisibility};
    use crate::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
    use std::collections::HashMap;

    fn make_key(s: &str) -> crate::entities::PropertyKey {
        crate::entities::PropertyKey::new(s).expect("valid key")
    }

    fn make_empty_block() -> Block {
        Block::new(BlockCreate {
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

    fn make_def(db_ident: &str, merge_policy: MergePolicy, mutability: PropertyMutability) -> crate::properties::PropertyDefinition {
        use crate::properties::types::ViewContext;
        crate::properties::PropertyDefinition {
            id: Uuid::new_v4(),
            db_ident: db_ident.to_string(),
            title: db_ident.to_string(),
            property_type: PropertyType::Text,
            cardinality: Cardinality::One,
            closed_values: Vec::new(),
            view_context: ViewContext::default(),
            public: false,
            queryable: false,
            hidden: false,
            attribute: None,
            read_only: false,
            status: crate::properties::types::PropertyStatus::Active,
            derived_from: None,
            visibility: PropertyVisibility::default(),
            mutability,
            merge_policy,
            alias_of: None,
            block_count: 0,
            page_count: 0,
            first_seen_at: None,
            last_seen_at: None,
        }
    }

    fn build_registry(defs: &[crate::properties::PropertyDefinition]) -> PropertyDefinitionRegistry {
        PropertyDefinitionRegistry::from_definitions(defs.iter().cloned())
    }

    // ── Forbidden keys ─────────────────────────────────────────────

    #[test]
    fn apply_to_rejects_content_key() {
        let patch = PropertyPatch::explicit(make_key("content"), PropertyValue::text("nope"));
        let mut block = make_empty_block();
        let defs = build_registry(&[]);
        let result = patch.apply_to(&mut block, &defs);
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "content"));
        // Block is not modified
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_to_rejects_text_key() {
        let patch = PropertyPatch::explicit(make_key("text"), PropertyValue::text("nope"));
        let mut block = make_empty_block();
        let defs = build_registry(&[]);
        let result = patch.apply_to(&mut block, &defs);
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "text"));
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_to_rejects_children_key() {
        let patch = PropertyPatch::explicit(make_key("children"), PropertyValue::text("nope"));
        let mut block = make_empty_block();
        let defs = build_registry(&[]);
        let result = patch.apply_to(&mut block, &defs);
        assert!(matches!(result, Err(DomainError::ForbiddenPatchKey(k)) if k == "children"));
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_to_skips_unknown_key() {
        let patch = PropertyPatch::explicit(make_key("unknown-prop"), PropertyValue::text("val"));
        let mut block = make_empty_block();
        let defs = build_registry(&[]);
        let result = patch.apply_to(&mut block, &defs).expect("no error");
        assert!(result.skipped.iter().any(|k| k.as_str() == "unknown-prop"));
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_to_rejects_explicit_patch_to_immutable_property() {
        let def = make_def("heading-level", MergePolicy::SetIfMissing, PropertyMutability::Immutable);
        let patch = PropertyPatch::explicit(make_key("heading-level"), PropertyValue::text("1"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs);
        assert!(matches!(result, Err(DomainError::ImmutableProperty(k)) if k == "heading-level"));
        assert!(block.properties.is_empty());
    }

    #[test]
    fn apply_to_block_content_is_preserved_on_error() {
        let def = make_def("heading-level", MergePolicy::SetIfMissing, PropertyMutability::Immutable);
        let patch = PropertyPatch::explicit(make_key("heading-level"), PropertyValue::text("1"));
        let mut block = make_empty_block();
        block.content = "original content".to_string();
        let original_content = block.content.clone();
        let _ = patch.apply_to(&mut block, &build_registry(&[def]));
        assert_eq!(block.content, original_content);
        assert!(block.properties.is_empty());
    }

    // ── SetIfMissing ─────────────────────────────────────────────

    #[test]
    fn apply_set_if_missing_writes_when_no_value() {
        let def = make_def("status", MergePolicy::SetIfMissing, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn apply_set_if_missing_skips_when_value_exists() {
        let def = make_def("status", MergePolicy::SetIfMissing, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("done")));
        assert!(result.skipped.iter().any(|k| k.as_str() == "status"));
    }

    #[test]
    fn apply_set_if_missing_never_errors() {
        let def = make_def("status", MergePolicy::SetIfMissing, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        patch.apply_to(&mut block, &defs).expect("never errors");
    }

    // ── Overwrite ─────────────────────────────────────────────────

    #[test]
    fn apply_overwrite_replaces_existing() {
        let def = make_def("status", MergePolicy::Overwrite, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn apply_overwrite_writes_when_no_value() {
        let def = make_def("status", MergePolicy::Overwrite, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
        assert!(result.skipped.is_empty());
    }

    #[test]
    fn apply_overwrite_never_errors() {
        let def = make_def("status", MergePolicy::Overwrite, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        patch.apply_to(&mut block, &defs).expect("never errors");
    }

    // ── Append ────────────────────────────────────────────────────

    #[test]
    fn apply_append_extends_existing_array() {
        let def = make_def("tags", MergePolicy::Append, PropertyMutability::Mutable);
        let def = crate::properties::PropertyDefinition {
            cardinality: Cardinality::Many,
            ..def
        };
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("new-tag"));
        let mut block = make_empty_block();
        block.properties.insert("tags".into(), PropertyValue::Array(vec![PropertyValue::text("existing")]));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        let arr = match block.properties.get("tags") {
            Some(PropertyValue::Array(a)) => a,
            other => panic!("expected Array, got {:?}", other),
        };
        assert_eq!(arr.len(), 2);
        assert!(arr.contains(&PropertyValue::text("existing")));
        assert!(arr.contains(&PropertyValue::text("new-tag")));
        assert!(result.derived_materialized.is_empty());
    }

    #[test]
    fn apply_append_seeds_new_array_when_no_value() {
        let mut def = make_def("tags", MergePolicy::Append, PropertyMutability::Mutable);
        def.cardinality = Cardinality::Many;
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("first"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        let arr = match block.properties.get("tags") {
            Some(PropertyValue::Array(a)) => a,
            other => panic!("expected Array, got {:?}", other),
        };
        assert_eq!(arr.len(), 1);
        assert!(arr.contains(&PropertyValue::text("first")));
    }

    #[test]
    fn apply_append_acts_as_overwrite_on_scalar_cardinality_one() {
        let def = make_def("status", MergePolicy::Append, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
    }

    #[test]
    fn apply_append_never_errors() {
        let mut def = make_def("tags", MergePolicy::Append, PropertyMutability::Mutable);
        def.cardinality = Cardinality::Many;
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("new-tag"));
        let mut block = make_empty_block();
        block.properties.insert("tags".into(), PropertyValue::text("existing"));
        let defs = build_registry(&[def]);
        patch.apply_to(&mut block, &defs).expect("never errors");
    }

    // ── Union ────────────────────────────────────────────────────

    #[test]
    fn apply_union_merges_removing_duplicates() {
        let mut def = make_def("tags", MergePolicy::Union, PropertyMutability::Mutable);
        def.cardinality = Cardinality::Many;
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("existing"));
        let mut block = make_empty_block();
        block.properties.insert("tags".into(), PropertyValue::Array(vec![PropertyValue::text("existing"), PropertyValue::text("other")]));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        let arr = match block.properties.get("tags") {
            Some(PropertyValue::Array(a)) => a,
            other => panic!("expected Array, got {:?}", other),
        };
        assert_eq!(arr.len(), 2); // no dup added
    }

    #[test]
    fn apply_union_extends_with_new_values() {
        let mut def = make_def("tags", MergePolicy::Union, PropertyMutability::Mutable);
        def.cardinality = Cardinality::Many;
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("new-tag"));
        let mut block = make_empty_block();
        block.properties.insert("tags".into(), PropertyValue::Array(vec![PropertyValue::text("existing")]));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        let arr = match block.properties.get("tags") {
            Some(PropertyValue::Array(a)) => a,
            other => panic!("expected Array, got {:?}", other),
        };
        assert_eq!(arr.len(), 2);
        assert!(arr.contains(&PropertyValue::text("existing")));
        assert!(arr.contains(&PropertyValue::text("new-tag")));
    }

    #[test]
    fn apply_union_acts_as_overwrite_on_scalar_cardinality_one() {
        let def = make_def("status", MergePolicy::Union, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
    }

    #[test]
    fn apply_union_never_errors() {
        let mut def = make_def("tags", MergePolicy::Union, PropertyMutability::Mutable);
        def.cardinality = Cardinality::Many;
        let patch = PropertyPatch::explicit(make_key("tags"), PropertyValue::text("new-tag"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        patch.apply_to(&mut block, &defs).expect("never errors");
    }

    // ── RejectOnConflict ─────────────────────────────────────────

    #[test]
    fn apply_reject_on_conflict_rejects_when_differ() {
        let def = make_def("status", MergePolicy::RejectOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs);
        assert!(matches!(result, Err(DomainError::MergeConflict { key, .. }) if key == "status"));
        // Block unchanged
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("done")));
    }

    #[test]
    fn apply_reject_on_conflict_accepts_when_match() {
        let def = make_def("status", MergePolicy::RejectOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("done"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert!(result.skipped.contains(&make_key("status")));
    }

    #[test]
    fn apply_reject_on_conflict_writes_when_no_value() {
        let def = make_def("status", MergePolicy::RejectOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
        assert!(result.derived_materialized.is_empty());
    }

    // ── AskOnConflict ────────────────────────────────────────────

    #[test]
    fn apply_ask_on_conflict_defers_when_differ() {
        let def = make_def("status", MergePolicy::AskOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(result.conflicts.len(), 1);
        assert_eq!(result.conflicts[0].policy, MergePolicy::AskOnConflict);
        // Block unchanged
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("done")));
    }

    #[test]
    fn apply_ask_on_conflict_accepts_when_match() {
        let def = make_def("status", MergePolicy::AskOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("done"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("done"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert!(result.conflicts.is_empty());
        assert!(result.skipped.contains(&make_key("status")));
    }

    #[test]
    fn apply_ask_on_conflict_writes_when_no_value() {
        let def = make_def("status", MergePolicy::AskOnConflict, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("todo"));
        let mut block = make_empty_block();
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("todo")));
    }

    // ── Derived provenance tracking ──────────────────────────────

    #[test]
    fn apply_derived_materialized_only_for_derived_patches() {
        // Immutable definition + Explicit patch → Err (immutable means never writable by user)
        let def_immutable = make_def("heading-level", MergePolicy::SetIfMissing, PropertyMutability::Immutable);
        let explicit_patch = PropertyPatch::explicit(make_key("heading-level"), PropertyValue::text("1"));
        let mut block_explicit = make_empty_block();
        let defs_immutable = build_registry(&[def_immutable]);
        let result_explicit = explicit_patch.apply_to(&mut block_explicit, &defs_immutable);
        assert!(matches!(result_explicit, Err(DomainError::ImmutableProperty(k)) if k == "heading-level"));

        // Mutable definition + Derived patch → writes and tracks derived_materialized
        let def_mutable = make_def("heading-level", MergePolicy::SetIfMissing, PropertyMutability::Mutable);
        let derived_patch = PropertyPatch::derived(make_key("heading-level"), PropertyValue::text("2"));
        let mut block_derived = make_empty_block();
        let defs_mutable = build_registry(&[def_mutable]);
        let result_derived = derived_patch.apply_to(&mut block_derived, &defs_mutable).expect("ok");
        assert!(result_derived.derived_materialized.iter().any(|k| k.as_str() == "heading-level"));
    }

    #[test]
    fn apply_skipped_reports_set_if_missing_kept_value() {
        let def = make_def("status", MergePolicy::SetIfMissing, PropertyMutability::Mutable);
        let patch = PropertyPatch::explicit(make_key("status"), PropertyValue::text("new"));
        let mut block = make_empty_block();
        block.properties.insert("status".into(), PropertyValue::text("existing"));
        let defs = build_registry(&[def]);
        let result = patch.apply_to(&mut block, &defs).expect("ok");
        assert!(result.skipped.iter().any(|k| k.as_str() == "status"));
        assert_eq!(block.properties.get("status"), Some(&PropertyValue::text("existing")));
    }

    // ── Per-policy matrix (T13) ───────────────────────────────────

    #[test]
    fn per_policy_outcome_matrix() {
        // (policy, pre_state, patch_value) → expected_outcome
        // Policies: SetIfMissing, Overwrite, Append, Union, RejectOnConflict, AskOnConflict
        // Pre states: None, Scalar(s), Array([s])
        // Patch: p
        // Expected: Write | Skip | Err | Conflict

        let policies = [
            MergePolicy::SetIfMissing,
            MergePolicy::Overwrite,
            MergePolicy::Append,
            MergePolicy::Union,
            MergePolicy::RejectOnConflict,
            MergePolicy::AskOnConflict,
        ];

        let patch = PropertyPatch::explicit(make_key("prop"), PropertyValue::text("p"));

        for policy in policies {
            let mut def = make_def("prop", policy, PropertyMutability::Mutable);
            def.cardinality = Cardinality::One;

            // Case: no pre-existing value → always write
            {
                let mut block = make_empty_block();
                let defs = build_registry(&[def.clone()]);
                let result = patch.apply_to(&mut block, &defs);
                assert!(result.is_ok(), "policy {:?} should not error with no pre-value", policy);
                assert!(block.properties.contains_key("prop"), "policy {:?} should write with no pre-value", policy);
            }

            // Case: same value → Skip (SetIfMissing, Reject, Ask) or Write (Overwrite, Append, Union)
            {
                let mut block = make_empty_block();
                block.properties.insert("prop".into(), PropertyValue::text("p"));
                let defs = build_registry(&[def.clone()]);
                let result = patch.apply_to(&mut block, &defs).unwrap();
                match policy {
                    MergePolicy::SetIfMissing
                    | MergePolicy::RejectOnConflict
                    | MergePolicy::AskOnConflict => {
                        assert!(result.skipped.iter().any(|k| k.as_str() == "prop"), "policy {:?} should skip when values match", policy);
                    }
                    _ => {
                        assert!(block.properties.get("prop") == Some(&PropertyValue::text("p")), "policy {:?} should keep value when values match", policy);
                    }
                }
            }

            // Case: different value + One cardinality
            {
                let mut block = make_empty_block();
                block.properties.insert("prop".into(), PropertyValue::text("existing"));
                let defs = build_registry(&[def.clone()]);
                let result = patch.apply_to(&mut block, &defs);
                match policy {
                    MergePolicy::RejectOnConflict => {
                        assert!(matches!(result, Err(DomainError::MergeConflict { .. })), "policy {:?} should error on conflict", policy);
                    }
                    MergePolicy::AskOnConflict => {
                        let outcome = result.expect("ok");
                        assert!(!outcome.conflicts.is_empty(), "policy {:?} should surface conflict", policy);
                    }
                    _ => {
                        let outcome = result.expect("ok");
                        assert!(outcome.conflicts.is_empty(), "policy {:?} should not conflict", policy);
                    }
                }
            }
        }
    }
}
