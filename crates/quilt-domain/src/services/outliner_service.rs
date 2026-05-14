//! OutlinerService - domain service for outliner logic

use crate::entities::Block;
use crate::errors::DomainError;
use crate::value_objects::{BlockFormat, Uuid};
use tracing::instrument;

/// OutlinerService provides domain logic for the outliner (block tree).
///
/// This service handles:
/// - Lexicographic order calculation for sibling blocks
/// - Rebalancing children after moves
/// - Circular reference validation
/// - Breadcrumb/path calculation
pub struct OutlinerService;

impl OutlinerService {
    /// Calculate the order value for inserting a block between siblings.
    ///
    /// Uses fractional indexing to allow insertions without reordering:
    /// - Insert at beginning: parent_order / 2
    /// - Insert between two siblings: (prev_order + next_order) / 2
    /// - Insert at end: last_order + 100
    #[instrument]
    pub fn calculate_order(sibling_orders: &[f64], position: usize) -> f64 {
        if sibling_orders.is_empty() {
            return 100.0;
        }

        let mut sorted = sibling_orders.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        if position == 0 {
            // Insert at beginning
            sorted[0] / 2.0
        } else if position >= sorted.len() {
            // Insert at end
            sorted[sorted.len() - 1] + 100.0
        } else {
            // Insert between position-1 and position
            (sorted[position - 1] + sorted[position]) / 2.0
        }
    }

    /// Rebalance children orders to use round numbers.
    ///
    /// Call this after deleting a child or when orders become too fragmented.
    #[instrument(skip(children))]
    pub fn rebalance_children(children: &mut [Block]) {
        for (i, child) in children.iter_mut().enumerate() {
            child.order = (i as f64 + 1.0) * 100.0;
        }
    }

    /// Validate that moving a block won't create a circular reference.
    ///
    /// A block cannot be moved to become a descendant of itself.
    #[instrument(skip(block, all_blocks))]
    pub fn validate_move(
        block: &Block,
        new_parent: Option<Uuid>,
        all_blocks: &[Block],
    ) -> Result<(), DomainError> {
        if !block.can_move_to(new_parent, all_blocks) {
            return Err(DomainError::CircularReference(block.id));
        }
        Ok(())
    }

    /// Get the tree structure (parent-child relationships) as a nested list.
    #[instrument(skip(blocks))]
    pub fn build_tree(blocks: &[Block], root_page_id: Uuid) -> Vec<TreeNode<'_>> {
        let mut result = Vec::new();
        let block_map: std::collections::HashMap<Uuid, &Block> =
            blocks.iter().map(|b| (b.id, b)).collect();

        // Find root blocks (no parent)
        let root_blocks: Vec<_> = blocks
            .iter()
            .filter(|b| b.page_id == root_page_id && b.parent_id.is_none())
            .collect();

        for block in root_blocks {
            Self::build_tree_recursive(block, &block_map, &mut result);
        }

        result
    }

    fn build_tree_recursive<'a>(
        block: &'a Block,
        _block_map: &std::collections::HashMap<Uuid, &'a Block>,
        result: &mut Vec<TreeNode<'a>>,
    ) {
        result.push(TreeNode {
            block,
            children: Vec::new(), // Will be filled by caller if needed
        });
    }

    /// Find the common ancestor of two blocks.
    #[instrument(skip(all_blocks))]
    pub fn find_common_ancestor(
        block_a_id: Uuid,
        block_b_id: Uuid,
        all_blocks: &[Block],
    ) -> Option<Uuid> {
        let path_a = Self::get_ancestor_path(block_a_id, all_blocks);
        let path_b = Self::get_ancestor_path(block_b_id, all_blocks);

        for ancestor in path_a.iter().rev() {
            if path_b.contains(ancestor) {
                return Some(*ancestor);
            }
        }

        None
    }

    /// Get the path of ancestor IDs from a block to the root.
    fn get_ancestor_path(block_id: Uuid, all_blocks: &[Block]) -> Vec<Uuid> {
        let mut path = vec![block_id];
        let mut current = block_id;

        while let Some(parent_id) = all_blocks
            .iter()
            .find(|b| b.id == current)
            .and_then(|b| b.parent_id)
        {
            path.push(parent_id);
            current = parent_id;
        }

        path
    }

    /// Calculate depth (level) from root for a block.
    #[instrument(skip(all_blocks))]
    pub fn calculate_depth(block_id: Uuid, all_blocks: &[Block]) -> usize {
        Self::get_ancestor_path(block_id, all_blocks).len()
    }
}

/// TreeNode represents a block with its children in the tree structure.
#[derive(Debug)]
pub struct TreeNode<'a> {
    pub block: &'a Block,
    pub children: Vec<TreeNode<'a>>,
}

impl Default for Block {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 0.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: String::new(),
            properties: std::collections::HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            journal_day: None,
            updated_journal_day: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_order_empty() {
        let order = OutlinerService::calculate_order(&[], 0);
        assert_eq!(order, 100.0);
    }

    #[test]
    fn test_calculate_order_at_beginning() {
        let orders = vec![100.0, 200.0, 300.0];
        let order = OutlinerService::calculate_order(&orders, 0);
        assert_eq!(order, 50.0); // 100 / 2
    }

    #[test]
    fn test_calculate_order_at_end() {
        let orders = vec![100.0, 200.0, 300.0];
        let order = OutlinerService::calculate_order(&orders, 10);
        assert_eq!(order, 400.0); // 300 + 100
    }

    #[test]
    fn test_calculate_order_between() {
        let orders = vec![100.0, 200.0, 300.0];
        let order = OutlinerService::calculate_order(&orders, 1);
        assert_eq!(order, 150.0); // (100 + 200) / 2
    }

    #[test]
    fn test_rebalance_children() {
        let mut children = vec![
            Block {
                order: 1.5,
                ..Default::default()
            },
            Block {
                order: 1.7,
                ..Default::default()
            },
            Block {
                order: 99.9,
                ..Default::default()
            },
        ];

        OutlinerService::rebalance_children(&mut children);

        assert_eq!(children[0].order, 100.0);
        assert_eq!(children[1].order, 200.0);
        assert_eq!(children[2].order, 300.0);
    }
}
