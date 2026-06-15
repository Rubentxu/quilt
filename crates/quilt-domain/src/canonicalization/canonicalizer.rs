//! Canonicalizer trait and PropertyDefinitionRegistry.

use crate::canonicalization::{CanonicalInput, CanonicalizationResult};
use crate::properties::PropertyDefinition;
use std::collections::HashMap;
use std::sync::Arc;

/// Canonicalizer port — object-safe, synchronous, pure-function trait.
///
/// Implementors transform [`CanonicalInput`] into [`CanonicalizationResult`]
/// without side effects. The V1 implementation is [`MarkdownCanonicalizer`]
/// in `quilt-application`.
pub trait Canonicalizer: Send + Sync {
    /// Canonicalize the input text, producing structured content and derived patches.
    fn canonicalize(&self, input: CanonicalInput) -> CanonicalizationResult;
}

/// Thin wrapper around a HashMap of property definitions, keyed by `db_ident`.
///
/// Mirrors the [`PropertyKeyResolver`] Arc-wrapper pattern. `Arc` avoids
/// cloning the (large) definition struct on every [`PropertyPatch::apply_to`] call.
#[derive(Debug, Clone)]
pub struct PropertyDefinitionRegistry(HashMap<String, Arc<PropertyDefinition>>);

impl PropertyDefinitionRegistry {
    /// Build a registry from an iterator of definitions.
    #[must_use]
    pub fn from_definitions<I: IntoIterator<Item = PropertyDefinition>>(iter: I) -> Self {
        let mut map = HashMap::new();
        for def in iter {
            map.insert(def.db_ident.clone(), Arc::new(def));
        }
        Self(map)
    }

    /// Look up a definition by its database identifier.
    #[must_use]
    pub fn get(&self, db_ident: &str) -> Option<Arc<PropertyDefinition>> {
        self.0.get(db_ident).cloned()
    }

    /// Number of definitions in the registry.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonicalization::Canonicalizer;
    use crate::properties::types::{Cardinality, PropertyType, PropertyVisibility};
    use std::sync::Arc;

    #[allow(deprecated)]
    fn make_def(db_ident: &str) -> PropertyDefinition {
        use crate::properties::types::ViewContext;
        PropertyDefinition {
            id: crate::value_objects::Uuid::new_v4(),
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
            mutability: crate::properties::types::PropertyMutability::Mutable,
            merge_policy: crate::properties::types::MergePolicy::SetIfMissing,
            alias_of: None,
            block_count: 0,
            page_count: 0,
            first_seen_at: None,
            last_seen_at: None,
        }
    }

    #[test]
    fn registry_from_iter_stores_by_db_ident() {
        let defs = vec![
            make_def("heading-level"),
            make_def("block-role"),
            make_def("type"),
        ];
        let reg = PropertyDefinitionRegistry::from_definitions(defs);
        assert_eq!(reg.len(), 3);
        let found = reg.get("heading-level").expect("found");
        assert_eq!(found.db_ident, "heading-level");
    }

    #[test]
    fn registry_get_missing_key_returns_none() {
        let reg = PropertyDefinitionRegistry::from_definitions(std::iter::empty());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn registry_arc_lookup_cheap() {
        let def = make_def("heading-level");
        let reg = PropertyDefinitionRegistry::from_definitions([def]);
        // 1000 lookups — sanity check it doesn't panic
        for _ in 0..1000 {
            let _ = reg.get("heading-level");
        }
    }

    #[test]
    fn canonicalizer_trait_is_object_safe() {
        // A dummy canonicalizer that does nothing
        struct DummyCanonicalizer;
        impl Canonicalizer for DummyCanonicalizer {
            fn canonicalize(&self, _input: CanonicalInput) -> CanonicalizationResult {
                CanonicalizationResult::empty(crate::content::BlockContent::empty())
            }
        }
        // Compile-time check: Box<dyn Canonicalizer> must be constructible
        let _: Box<dyn Canonicalizer> = Box::new(DummyCanonicalizer);
    }
}
