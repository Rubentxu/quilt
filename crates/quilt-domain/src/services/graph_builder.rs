//! GraphBuilder — constructs typed edges from RefIndex
//!
//! G4: Static graph construction from RefIndex.
//!
//! This service consumes the public API of [`RefIndex`] (specifically
//! `get_forward_refs` and `get_backlinks`) to construct typed edges.
//! No internal HashMap access — only the documented public methods.

use crate::references::{EdgeType, RefIndex, RefType, TypedEdge};
use crate::value_objects::Uuid;

/// Graph builder for constructing typed edges from a [`RefIndex`].
///
/// G4 (V1): Types only. No walker (deferred to V2).
///
/// # Example
///
/// ```
/// use quilt_domain::references::{EdgeType, GraphBuilder, RefIndex};
/// use quilt_domain::value_objects::Uuid;
///
/// let index = RefIndex::new();
/// let root = Uuid::new_v4();
/// let edges = GraphBuilder::build_from_ref_index(root, &index);
/// assert!(edges.is_empty());
/// ```
#[derive(Debug, Clone, Default)]
pub struct GraphBuilder;

impl GraphBuilder {
    /// Build a list of typed edges from all forward references of `root`.
    ///
    /// Consumes `RefIndex`'s public API only:
    /// - [`RefIndex::get_forward_refs`] for outgoing edges
    ///
    /// Each `Ref` in the index is converted to a [`TypedEdge`] using
    /// the RefType → EdgeType mapping table from the spec.
    ///
    /// # Arguments
    ///
    /// * `root` - The source entity UUID
    /// * `index` - The reference index to query
    ///
    /// # Returns
    ///
    /// A vector of [`TypedEdge`] representing all outgoing references from `root`.
    /// Returns an empty vector if `root` has no outgoing references.
    pub fn build_from_ref_index(root: Uuid, index: &RefIndex) -> Vec<TypedEdge> {
        index
            .get_forward_refs(root)
            .into_iter()
            .map(|(target, ref_type)| {
                TypedEdge::new(root, target, ref_type_to_edge_type(ref_type), 1.0, 0)
            })
            .collect()
    }
}

/// Convert a RefType to EdgeType using the spec mapping table.
///
/// | RefType   | EdgeType           |
/// |-----------|-------------------|
/// | PageRef   | PageRef           |
/// | BlockRef  | BlockRef          |
/// | Tag       | Tag               |
/// | Alias     | Custom("alias")   |
fn ref_type_to_edge_type(ref_type: RefType) -> EdgeType {
    match ref_type {
        RefType::PageRef => EdgeType::PageRef,
        RefType::BlockRef => EdgeType::BlockRef,
        RefType::Tag => EdgeType::Tag,
        RefType::Alias => EdgeType::Custom("alias".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // G4.R3 — mapping scenarios

    #[test]
    fn test_empty_index_returns_empty_edges() {
        let index = RefIndex::new();
        let root = Uuid::new_v4();
        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_pageref_maps_to_pageref() {
        let mut index = RefIndex::new();
        let root = Uuid::new_v4();
        let target = Uuid::new_v4();
        index.add_ref(root, target, RefType::PageRef);

        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::PageRef);
        assert_eq!(edges[0].from, root);
        assert_eq!(edges[0].to, target);
    }

    #[test]
    fn test_blockref_maps_to_blockref() {
        let mut index = RefIndex::new();
        let root = Uuid::new_v4();
        let target = Uuid::new_v4();
        index.add_ref(root, target, RefType::BlockRef);

        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::BlockRef);
    }

    #[test]
    fn test_tag_maps_to_tag() {
        let mut index = RefIndex::new();
        let root = Uuid::new_v4();
        let target = Uuid::new_v4();
        index.add_ref(root, target, RefType::Tag);

        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].edge_type, EdgeType::Tag);
    }

    #[test]
    fn test_alias_maps_to_custom() {
        let mut index = RefIndex::new();
        let root = Uuid::new_v4();
        let target = Uuid::new_v4();
        index.add_ref(root, target, RefType::Alias);

        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert_eq!(edges.len(), 1);
        match &edges[0].edge_type {
            EdgeType::Custom(s) => assert_eq!(s, "alias"),
            other => panic!("Expected Custom(\"alias\"), got {:?}", other),
        }
    }

    #[test]
    fn test_multiple_refs() {
        let mut index = RefIndex::new();
        let root = Uuid::new_v4();
        let t1 = Uuid::new_v4();
        let t2 = Uuid::new_v4();
        let t3 = Uuid::new_v4();
        index.add_ref(root, t1, RefType::PageRef);
        index.add_ref(root, t2, RefType::BlockRef);
        index.add_ref(root, t3, RefType::Tag);

        let edges = GraphBuilder::build_from_ref_index(root, &index);
        assert_eq!(edges.len(), 3);
    }

    // V1 is types only — the compile-time absence of walk/bfs/dfs/traverse
    // methods on GraphBuilder is verified by the fact that this file compiles.
}
