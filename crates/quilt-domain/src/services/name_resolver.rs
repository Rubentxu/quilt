//! NameResolver — trait for resolving names to page/block IDs
//!
//! G5: Named-thing retrieval foundation.
//!
//! This trait provides a dependency inversion boundary for fuzzy name resolution.
//! Implementations live in infrastructure (quilt-search), keeping the domain
//! crate free of search dependencies.
//!
//! # Why a trait?
//!
//! The domain crate (`quilt-domain`) MUST NOT depend on:
//! - `sqlx` (infrastructure)
//! - `quilt-infrastructure` (infrastructure)
//! - `quilt-search` (search implementation)
//!
//! By defining a trait in domain and implementing it in the appropriate crate,
//! we maintain clean architecture boundaries.

use serde::{Deserialize, Serialize};
use std::fmt;

/// The kind of entity a resolved name refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedKind {
    /// A page entity
    Page,
    /// A block entity
    Block,
}

impl fmt::Display for ResolvedKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolvedKind::Page => write!(f, "page"),
            ResolvedKind::Block => write!(f, "block"),
        }
    }
}

/// A resolved name with its ID, kind, and relevance score.
///
/// Returned by [`NameResolver::resolve_by_name`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedName {
    /// The UUID of the resolved entity
    pub id: String,
    /// The kind of entity (page or block)
    pub kind: ResolvedKind,
    /// The matched name (may be an alias or fuzzy match)
    pub name: String,
    /// Relevance score in range `[0.0, 1.0]`, higher is better
    pub score: f32,
}

impl ResolvedName {
    /// Create a new ResolvedName.
    pub fn new(id: String, kind: ResolvedKind, name: String, score: f32) -> Self {
        Self { id, kind, name, score }
    }
}

/// Trait for resolving names to page/block IDs via fuzzy matching.
///
/// G5 (Named-thing retrieval): This trait is the domain boundary for the
/// fuzzy name resolution capability. Implementations should use FTS5
/// with prefix-first strategy per the design spec.
///
/// The trait is object-safe (`Send + Sync`) so it can be used as
/// `Arc<dyn NameResolver>` in application services.
pub trait NameResolver: Send + Sync {
    /// Resolve a name to a list of matching pages/blocks.
    ///
    /// The implementation should use fuzzy matching (FTS5 prefix-first)
    /// and return results sorted by relevance score descending.
    ///
    /// # Arguments
    ///
    /// * `name` - The name or partial name to search for
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of resolved names sorted by score descending.
    fn resolve_by_name(&self, name: &str, limit: usize) -> Vec<ResolvedName>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_resolved_name_page_roundtrip() {
        let original = ResolvedName::new(
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
            ResolvedKind::Page,
            "My Page".to_string(),
            0.95,
        );
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ResolvedName = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_resolved_name_block_roundtrip() {
        let original = ResolvedName::new(
            "550e8400-e29b-41d4-a716-446655440001".to_string(),
            ResolvedKind::Block,
            "My Block".to_string(),
            0.85,
        );
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ResolvedName = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    // Proptest: 100 random ResolvedName instances round-trip correctly
    proptest! {
        #[test]
        fn test_resolved_name_proptest_roundtrip(
            id in "[a-f0-9]{36}",
            kind in prop_oneof![Just(ResolvedKind::Page), Just(ResolvedKind::Block)],
            name in "[a-zA-Z0-9 ]{1,100}",
            score in 0.0f32..1.0f32,
        ) {
            let original = ResolvedName::new(id, kind, name, score);
            let json = serde_json::to_string(&original).unwrap();
            let parsed: ResolvedName = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(original, parsed);
        }
    }
}
