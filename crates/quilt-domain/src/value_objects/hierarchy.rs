//! Hierarchy value object - represents block hierarchy information
//!
//! This value object encapsulates the structural relationship of a block
//! within the page hierarchy: its parent, position among siblings, and
//! indentation level.

use crate::value_objects::Uuid;

/// Hierarchy represents the structural position of a block in the hierarchy.
///
/// It captures:
/// - `page_id`: The page this block belongs to
/// - `parent_id`: The parent block (None for top-level blocks)
/// - `order`: Lexicographic order among siblings (fractional indexing)
/// - `level`: Indentation level (1-indexed)
///
/// # Invariants
///
/// - `level` is always >= 1
/// - If `parent_id` is `Some`, then `level` must be >= 2
/// - If `parent_id` is `None`, then `level` must be == 1 (root level)
#[derive(Debug, Clone, PartialEq)]
pub struct Hierarchy {
    /// The page this block belongs to
    pub page_id: Uuid,
    /// Parent block (None for top-level blocks on a page)
    pub parent_id: Option<Uuid>,
    /// Lexicographic order among siblings (fractional indexing)
    pub order: f64,
    /// Indentation level (1-indexed, root is 1)
    pub level: u8,
}

impl Hierarchy {
    /// Create a new root-level hierarchy (no parent).
    ///
    /// Root blocks have `level = 1` and `parent_id = None`.
    pub fn root(page_id: Uuid, order: f64) -> Self {
        Self {
            page_id,
            parent_id: None,
            order,
            level: 1,
        }
    }

    /// Create a new child hierarchy.
    ///
    /// Child blocks have `level = parent_level + 1` (minimum 2).
    pub fn child(page_id: Uuid, parent_id: Uuid, parent_level: u8, order: f64) -> Self {
        Self {
            page_id,
            parent_id: Some(parent_id),
            order,
            level: parent_level.saturating_add(1).max(2),
        }
    }

    /// Check if this is a root-level block (no parent).
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this is a child block (has parent).
    pub fn is_child(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Get the expected level for a child of this hierarchy.
    ///
    /// Returns `self.level + 1`, with a minimum of 2.
    pub fn child_level(&self) -> u8 {
        self.level.saturating_add(1).max(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_level_is_1() {
        let page_id = Uuid::new_v4();
        let hierarchy = Hierarchy::root(page_id, 1.0);
        
        assert!(hierarchy.is_root());
        assert!(hierarchy.parent_id.is_none());
        assert_eq!(hierarchy.level, 1);
    }

    #[test]
    fn test_child_level_increments() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let parent = Hierarchy::root(page_id, 1.0);
        
        let child = Hierarchy::child(page_id, parent_id, parent.level, 1.0);
        
        assert!(child.is_child());
        assert_eq!(child.parent_id, Some(parent_id));
        assert_eq!(child.level, 2);
    }

    #[test]
    fn test_nested_child_level() {
        let page_id = Uuid::new_v4();
        let grandchild_id = Uuid::new_v4();
        
        let parent = Hierarchy::root(page_id, 1.0);
        let child = Hierarchy::child(page_id, Uuid::new_v4(), parent.level, 1.0);
        let grandchild = Hierarchy::child(page_id, grandchild_id, child.level, 1.0);
        
        assert_eq!(grandchild.level, 3);
    }

    #[test]
    fn test_child_level_minimum_is_2() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        
        // Even if parent level is 0 (which shouldn't happen), child should be at least 2
        let child = Hierarchy::child(page_id, parent_id, 0, 1.0);
        
        assert_eq!(child.level, 2);
    }
}
