//! Preset registry port — maps [`PresetId`] to [`PropertyPreset`].
//!
//! The trait is object-safe (`Send + Sync`) so it can be wrapped in
//! `Arc<dyn PresetRegistry>` and shared across use cases.

use super::{PresetId, PropertyPreset};

/// Port for looking up property presets by id.
///
/// Implementors may be static (hard-coded V1), dynamic (loaded from DB),
/// or plugin-based. The V1 implementation is [`StaticPresetRegistry`]
/// in `quilt-application`.
pub trait PresetRegistry: Send + Sync {
    /// Look up a preset by its id.
    ///
    /// Returns `None` if no preset with this id exists.
    fn get(&self, id: &PresetId) -> Option<PropertyPreset>;

    /// List all preset ids in declaration order.
    fn list(&self) -> Vec<PresetId>;

    /// Number of presets in the registry.
    fn len(&self) -> usize;

    /// Returns `true` if the registry is empty.
    #[must_use]
    fn is_empty(&self) -> bool {
        self.len() == 0
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

    fn explicit_patch(key: &str, value: &str) -> crate::canonicalization::PropertyPatch {
        crate::canonicalization::PropertyPatch {
            key: make_key(key),
            value: PropertyValue::text(value),
            provenance: PropertyPatchProvenance::Explicit,
        }
    }

    /// A test registry holding a fixed map of presets.
    #[derive(Debug, Clone)]
    struct TestRegistry(HashMap<PresetId, PropertyPreset>);

    impl TestRegistry {
        fn new() -> Self {
            Self(HashMap::new())
        }

        fn insert(mut self, preset: PropertyPreset) -> Self {
            self.0.insert(preset.id.clone(), preset);
            self
        }
    }

    impl PresetRegistry for TestRegistry {
        fn get(&self, id: &PresetId) -> Option<PropertyPreset> {
            self.0.get(id).cloned()
        }

        fn list(&self) -> Vec<PresetId> {
            self.0.keys().cloned().collect()
        }

        fn len(&self) -> usize {
            self.0.len()
        }
    }

    fn make_preset(id: &str) -> PropertyPreset {
        PropertyPreset::new(
            PresetId::new(id).unwrap(),
            vec![explicit_patch("type", "task")],
            crate::canonicalization::PresetArgs::empty(),
            "test",
        )
        .unwrap()
    }

    #[test]
    fn dyn_trait_accepts_test_registry() {
        // Compile-time check: Box<dyn PresetRegistry> accepts TestRegistry
        let reg: Box<dyn PresetRegistry> =
            Box::new(TestRegistry::new().insert(make_preset("/TODO")));
        assert_eq!(reg.len(), 1);
        let got = reg.get(&PresetId::new("/TODO").unwrap());
        assert!(got.is_some());
    }

    #[test]
    fn list_returns_ids() {
        let reg = TestRegistry::new()
            .insert(make_preset("/A"))
            .insert(make_preset("/B"));
        let ids = reg.list();
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn len_matches_count() {
        let reg = TestRegistry::new()
            .insert(make_preset("/A"))
            .insert(make_preset("/B"))
            .insert(make_preset("/C"));
        assert_eq!(reg.len(), 3);
        assert!(!reg.is_empty());
    }

    #[test]
    fn unknown_id_returns_none() {
        let reg = TestRegistry::new().insert(make_preset("/TODO"));
        let got = reg.get(&PresetId::new("/NotAPreset").unwrap());
        assert!(got.is_none());
    }

    #[test]
    fn empty_registry_is_empty() {
        let reg = TestRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert!(reg.list().is_empty());
    }
}
