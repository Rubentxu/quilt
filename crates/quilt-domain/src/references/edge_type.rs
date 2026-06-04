//! EdgeType — the type of a typed edge in the Quilt graph
//!
//! EdgeType distinguishes between different kinds of links in the knowledge graph:
//! - `PageRef`: `[[Page Name]]` — link to a page
//! - `BlockRef`: `((block-uuid))` — link to a specific block
//! - `Tag`: `#tag` — a tag applied to content
//! - `Backlink`: bidirectional link (computed, not stored)
//! - `Namespace`: hierarchical namespace reference
//! - `Parent`: parent-child relationship
//! - `Custom(String)`: forward-compatibility for user-defined edge types

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of a typed edge in the Quilt graph.
///
/// Each variant corresponds to a different linking mechanism in Quilt.
/// These types are used to construct typed edges via [`TypedEdge`].
///
/// Unit variants (`PageRef`, `BlockRef`, `Tag`, `Backlink`, `Namespace`, `Parent`)
/// are `Copy`. The `Custom` variant is not `Copy` since it contains a `String`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Reference to a page via `[[Page Name]]`
    PageRef,
    /// Reference to a block via `((block-uuid))`
    BlockRef,
    /// A `#tag` applied to content
    Tag,
    /// A bidirectional backlink (computed, not stored as a separate type)
    Backlink,
    /// Hierarchical namespace reference
    Namespace,
    /// Parent-child relationship
    Parent,
    /// User-defined custom edge type for forward-compatibility
    Custom(String),
}

/// Custom edge type for user-defined edge semantics.
///
/// This is a separate type so that `EdgeType` can derive `Copy` for unit variants.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomEdgeType(pub String);

impl EdgeType {
    /// Returns true if this is a unit variant (not Custom).
    #[inline]
    pub fn is_unit(&self) -> bool {
        !matches!(self, EdgeType::Custom(_))
    }
}

impl fmt::Display for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EdgeType::PageRef => write!(f, "page_ref"),
            EdgeType::BlockRef => write!(f, "block_ref"),
            EdgeType::Tag => write!(f, "tag"),
            EdgeType::Backlink => write!(f, "backlink"),
            EdgeType::Namespace => write!(f, "namespace"),
            EdgeType::Parent => write!(f, "parent"),
            EdgeType::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TypedEdge
// ─────────────────────────────────────────────────────────────────────────────

/// A typed edge in the Quilt graph with weight and timestamp.
///
/// Represents a directed relationship from one block/page to another with:
/// - `from`: source entity UUID
/// - `to`: target entity UUID
/// - `edge_type`: the type of relationship ([`EdgeType`])
/// - `weight`: confidence score in range `[0.0, 1.0]`, clamped by [`TypedEdge::new`]
/// - `created_at`: unix epoch timestamp in seconds (i64 for WASM compatibility)
///
/// # Example
///
/// ```
/// use quilt_domain::references::{EdgeType, TypedEdge};
/// use quilt_domain::value_objects::Uuid;
///
/// let from = Uuid::new_v4();
/// let to = Uuid::new_v4();
/// let edge = TypedEdge::new(from, to, EdgeType::PageRef, 0.85, 1700000000);
/// assert_eq!(edge.edge_type, EdgeType::PageRef);
/// assert!((edge.weight - 0.85).abs() < f32::EPSILON);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedEdge {
    /// Source entity UUID
    pub from: Uuid,
    /// Target entity UUID
    pub to: Uuid,
    /// The type of edge
    pub edge_type: EdgeType,
    /// Confidence score in range `[0.0, 1.0]`
    pub weight: f32,
    /// Unix epoch timestamp in seconds
    pub created_at: i64,
}

impl TypedEdge {
    /// Creates a new `TypedEdge` with weight clamped to `[0.0, 1.0]`.
    ///
    /// If `weight` is greater than 1.0, it is clamped to 1.0.
    /// If `weight` is less than 0.0, it is clamped to 0.0.
    ///
    /// # Arguments
    ///
    /// * `from` - Source entity UUID
    /// * `to` - Target entity UUID
    /// * `edge_type` - The type of edge
    /// * `weight` - Confidence score (will be clamped to `[0.0, 1.0]`)
    /// * `created_at` - Unix epoch timestamp in seconds
    #[inline]
    pub fn new(from: Uuid, to: Uuid, edge_type: EdgeType, weight: f32, created_at: i64) -> Self {
        Self {
            from,
            to,
            edge_type,
            weight: weight.clamp(0.0, 1.0),
            created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // G4.R2 — weight clamping scenarios

    #[test]
    fn test_weight_clamp_above_1_0() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let edge = TypedEdge::new(from, to, EdgeType::PageRef, 1.5, 1700000000);
        assert!((edge.weight - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weight_clamp_negative() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let edge = TypedEdge::new(from, to, EdgeType::PageRef, -0.3, 1700000000);
        assert!((edge.weight - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weight_within_range_unchanged() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let edge = TypedEdge::new(from, to, EdgeType::PageRef, 0.75, 1700000000);
        assert!((edge.weight - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weight_exactly_0_unchanged() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let edge = TypedEdge::new(from, to, EdgeType::PageRef, 0.0, 1700000000);
        assert!((edge.weight - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_weight_exactly_1_unchanged() {
        let from = Uuid::new_v4();
        let to = Uuid::new_v4();
        let edge = TypedEdge::new(from, to, EdgeType::PageRef, 1.0, 1700000000);
        assert!((edge.weight - 1.0).abs() < f32::EPSILON);
    }
}
