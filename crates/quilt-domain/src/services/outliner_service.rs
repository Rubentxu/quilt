//! OutlinerService - domain service for outliner logic

use crate::content::BlockContent;
use crate::entities::Block;
use crate::errors::DomainError;
use crate::value_objects::{BlockFormat, Uuid};
use tracing::instrument;

/// Result of an indent/dedent operation containing the new parent and order.
#[derive(Debug, Clone)]
pub struct MoveCalculation {
    /// The new parent ID (None means root level)
    pub new_parent_id: Option<Uuid>,
    /// The new order value
    pub new_order: f64,
    /// The new level
    pub new_level: u8,
}

/// Result of a subtree move operation.
#[derive(Debug, Clone)]
pub struct SubtreeMove {
    /// The block being moved
    pub block_id: Uuid,
    /// The new parent ID (None means root level)
    pub new_parent_id: Option<Uuid>,
    /// The new order value
    pub new_order: f64,
    /// The new level
    pub new_level: u8,
}

/// The type of backspace action to take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackspaceAction {
    /// Cursor at start of content block - try to dedent
    Dedent,
    /// Empty block with children - promote children
    PromoteChildren,
    /// Empty block without children - merge with previous sibling
    MergeWithPrevious,
    /// Cursor in middle of text - normal backspace (handled by UI, not here)
    NormalBackspace,
}

/// OutlinerService provides domain logic for the outliner (block tree).
///
/// This service handles:
/// - Lexicographic order calculation for sibling blocks
/// - Rebalancing children after moves
/// - Circular reference validation
/// - Breadcrumb/path calculation
/// - Indent/dedent operations
/// - Backspace priority handling
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

    /// Calculate the new position for indenting a block.
    ///
    /// Tab pressed: Move block (and its entire subtree) to become the last child
    /// of its previous sibling.
    ///
    /// Returns `None` if the block cannot be indented (no previous sibling).
    #[instrument(skip(all_blocks))]
    pub fn calculate_indent(
        block: &Block,
        all_blocks: &[Block],
    ) -> Option<MoveCalculation> {
        // Find previous sibling
        let siblings = all_blocks
            .iter()
            .filter(|b| b.page_id == block.page_id && b.parent_id == block.parent_id && b.id != block.id)
            .collect::<Vec<_>>();

        // Get previous sibling (immediate left neighbor)
        let previous_sibling = siblings
            .iter()
            .filter(|b| b.order < block.order)
            .max_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(std::cmp::Ordering::Equal))?;

        // New parent is the previous sibling
        let new_parent_id = Some(previous_sibling.id);

        // New order: be the last child of the new parent
        let children_of_new_parent = all_blocks
            .iter()
            .filter(|b| b.page_id == block.page_id && b.parent_id == new_parent_id)
            .collect::<Vec<_>>();

        let new_order = if children_of_new_parent.is_empty() {
            100.0
        } else {
            let max_order = children_of_new_parent
                .iter()
                .map(|b| b.order)
                .fold(0.0f64, |max, o| if o > max { o } else { max });
            max_order + 100.0
        };

        // New level is parent's level + 1
        let parent_level = all_blocks
            .iter()
            .find(|b| b.id == previous_sibling.id)
            .map(|b| b.level)
            .unwrap_or(1);

        Some(MoveCalculation {
            new_parent_id,
            new_order,
            new_level: parent_level + 1,
        })
    }

    /// Calculate the new position for dedenting a block.
    ///
    /// Shift+Tab pressed: Move block (and its entire subtree) to become a sibling
    /// of its parent, positioned after the parent.
    ///
    /// Returns `None` if the block cannot be dedented (is at root level).
    #[instrument(skip(all_blocks))]
    pub fn calculate_dedent(
        block: &Block,
        all_blocks: &[Block],
    ) -> Option<MoveCalculation> {
        // Find the parent block
        let parent_id = block.parent_id?;
        let parent = all_blocks
            .iter()
            .find(|b| b.id == parent_id)?;

        // New parent is the grandparent (or None if parent is at root)
        let new_parent_id = parent.parent_id;

        // Find all siblings of the parent (including parent's siblings)
        let parent_siblings = all_blocks
            .iter()
            .filter(|b| b.page_id == block.page_id && b.parent_id == new_parent_id && b.id != parent.id)
            .collect::<Vec<_>>();

        // New order: be positioned after the parent among its siblings
        let new_order = if parent_siblings.is_empty() {
            // No siblings, just use parent's order + small increment
            parent.order + 50.0
        } else {
            // Find siblings with order > parent.order
            let greater_siblings = parent_siblings
                .iter()
                .filter(|b| b.order > parent.order)
                .collect::<Vec<_>>();

            if greater_siblings.is_empty() {
                // No greater siblings, put after parent
                parent.order + 100.0
            } else {
                // Put between parent and the next sibling
                let next_sibling_order = greater_siblings
                    .iter()
                    .map(|b| b.order)
                    .fold(f64::MAX, |min, o| if o < min { o } else { min });
                (parent.order + next_sibling_order) / 2.0
            }
        };

        Some(MoveCalculation {
            new_parent_id,
            new_order,
            new_level: parent.level,
        })
    }

    /// Determine what backspace action to take for a block.
    ///
    /// Per ADR 0004, priority is:
    /// 1. Block with content, cursor at start → dedent
    /// 2. Empty block with children → promote children
    /// 3. Empty block without children → merge with previous sibling
    #[instrument(skip(all_blocks))]
    pub fn determine_backspace_action(
        block: &Block,
        cursor_at_start: bool,
        all_blocks: &[Block],
    ) -> BackspaceAction {
        // Case 1: Cursor at start of block with content → try dedent
        if cursor_at_start && !block.content.is_empty() {
            // Can we dedent? Only if parent is not root-level with no parent
            if block.parent_id.is_some() {
                return BackspaceAction::Dedent;
            }
        }

        // Case 2: Empty block with children → promote children
        if block.content.is_empty() {
            let has_children = all_blocks
                .iter()
                .any(|b| b.page_id == block.page_id && b.parent_id == Some(block.id));

            if has_children {
                return BackspaceAction::PromoteChildren;
            }

            // Case 3: Empty block without children → merge with previous
            return BackspaceAction::MergeWithPrevious;
        }

        // Default: normal backspace (handled by UI)
        BackspaceAction::NormalBackspace
    }

    /// Calculate moves for promoting children when an empty block is deleted.
    ///
    /// When a block with children is deleted, its children become children of
    /// the deleted block's parent, positioned where the deleted block was.
    #[instrument(skip(all_blocks))]
    pub fn calculate_promote_children(
        block: &Block,
        all_blocks: &[Block],
    ) -> Vec<SubtreeMove> {
        let parent_id = block.parent_id;

        // Get children of this block
        let children = all_blocks
            .iter()
            .filter(|b| b.page_id == block.page_id && b.parent_id == Some(block.id))
            .collect::<Vec<_>>();

        children
            .into_iter()
            .map(|child| {
                // Update children to have the same parent as the deleted block
                SubtreeMove {
                    block_id: child.id,
                    new_parent_id: parent_id,
                    new_order: child.order, // Keep same order initially
                    new_level: block.level, // Level up to match deleted block
                }
            })
            .collect()
    }

    /// Calculate move for merging a block with its previous sibling.
    ///
    /// When an empty block without children is deleted, its content (if any)
    /// is merged into the previous sibling.
    #[instrument(skip(all_blocks))]
    pub fn calculate_merge_with_previous(
        block: &Block,
        all_blocks: &[Block],
    ) -> Option<MergeCalculation> {
        // Find previous sibling
        let previous_sibling = all_blocks
            .iter()
            .filter(|b| b.page_id == block.page_id && b.parent_id == block.parent_id && b.order < block.order)
            .max_by(|a, b| a.order.partial_cmp(&b.order).unwrap_or(std::cmp::Ordering::Equal))?;

        Some(MergeCalculation {
            target_block_id: previous_sibling.id,
            deleted_block_id: block.id,
            content_to_append: block.content.clone(),
        })
    }

    /// Get all block IDs in the subtree rooted at the given block.
    #[instrument(skip(all_blocks))]
    pub fn get_subtree_ids(block_id: Uuid, all_blocks: &[Block]) -> Vec<Uuid> {
        let mut result = vec![block_id];

        // Recursively find all descendants
        fn collect_descendants(
            parent_id: Uuid,
            all_blocks: &[Block],
            result: &mut Vec<Uuid>,
        ) {
            for block in all_blocks {
                if block.parent_id == Some(parent_id) {
                    result.push(block.id);
                    collect_descendants(block.id, all_blocks, result);
                }
            }
        }

        collect_descendants(block_id, all_blocks, &mut result);
        result
    }
}

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub struct MergeCalculation {
    /// The block that will receive the merged content
    pub target_block_id: Uuid,
    /// The block being deleted
    pub deleted_block_id: Uuid,
    /// The content to append to the target block
    pub content_to_append: BlockContent,
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
            content: BlockContent::empty(),
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

    #[test]
    fn test_get_subtree_ids() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let child1_id = Uuid::new_v4();
        let child2_id = Uuid::new_v4();
        let grandchild_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: parent_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::from_text("Parent"),
                ..Default::default()
            },
            Block {
                id: child1_id,
                page_id,
                parent_id: Some(parent_id),
                order: 100.0,
                level: 2,
                content: BlockContent::from_text("Child 1"),
                ..Default::default()
            },
            Block {
                id: child2_id,
                page_id,
                parent_id: Some(parent_id),
                order: 200.0,
                level: 2,
                content: BlockContent::from_text("Child 2"),
                ..Default::default()
            },
            Block {
                id: grandchild_id,
                page_id,
                parent_id: Some(child1_id),
                order: 100.0,
                level: 3,
                content: BlockContent::from_text("Grandchild"),
                ..Default::default()
            },
        ];

        let subtree = OutlinerService::get_subtree_ids(parent_id, &blocks);
        assert_eq!(subtree.len(), 4);
        assert!(subtree.contains(&parent_id));
        assert!(subtree.contains(&child1_id));
        assert!(subtree.contains(&child2_id));
        assert!(subtree.contains(&grandchild_id));
    }

    #[test]
    fn test_calculate_indent() {
        let page_id = Uuid::new_v4();
        let block_a_id = Uuid::new_v4();
        let block_b_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: block_a_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::from_text("A"),
                ..Default::default()
            },
            Block {
                id: block_b_id,
                page_id,
                parent_id: None,
                order: 200.0,
                level: 1,
                content: BlockContent::from_text("B"),
                ..Default::default()
            },
        ];

        let block_b = blocks.iter().find(|b| b.id == block_b_id).unwrap();
        let result = OutlinerService::calculate_indent(block_b, &blocks);

        assert!(result.is_some());
        let move_calc = result.unwrap();
        assert_eq!(move_calc.new_parent_id, Some(block_a_id));
        assert_eq!(move_calc.new_level, 2);
    }

    #[test]
    fn test_calculate_indent_no_previous_sibling() {
        let page_id = Uuid::new_v4();
        let block_a_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: block_a_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::from_text("A"),
                ..Default::default()
            },
        ];

        let block_a = blocks.iter().find(|b| b.id == block_a_id).unwrap();
        let result = OutlinerService::calculate_indent(block_a, &blocks);

        // Cannot indent first block (no previous sibling)
        assert!(result.is_none());
    }

    #[test]
    fn test_calculate_dedent() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: parent_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::from_text("Parent"),
                ..Default::default()
            },
            Block {
                id: child_id,
                page_id,
                parent_id: Some(parent_id),
                order: 100.0,
                level: 2,
                content: BlockContent::from_text("Child"),
                ..Default::default()
            },
        ];

        let child = blocks.iter().find(|b| b.id == child_id).unwrap();
        let result = OutlinerService::calculate_dedent(child, &blocks);

        assert!(result.is_some());
        let move_calc = result.unwrap();
        assert_eq!(move_calc.new_parent_id, None); // Becomes root level
        assert_eq!(move_calc.new_level, 1);
    }

    #[test]
    fn test_determine_backspace_action_dedent() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let block_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: parent_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::from_text("Parent"),
                ..Default::default()
            },
            Block {
                id: block_id,
                page_id,
                parent_id: Some(parent_id),
                order: 100.0,
                level: 2,
                content: BlockContent::from_text("Child with content"),
                ..Default::default()
            },
        ];

        let block = blocks.iter().find(|b| b.id == block_id).unwrap();
        let action = OutlinerService::determine_backspace_action(block, true, &blocks);

        assert_eq!(action, BackspaceAction::Dedent);
    }

    #[test]
    fn test_determine_backspace_action_promote_children() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: parent_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::empty(), // Empty!
                ..Default::default()
            },
            Block {
                id: child_id,
                page_id,
                parent_id: Some(parent_id),
                order: 100.0,
                level: 2,
                content: BlockContent::from_text("Child 1"),
                ..Default::default()
            },
        ];

        let empty_block = blocks.iter().find(|b| b.id == parent_id).unwrap();
        let action = OutlinerService::determine_backspace_action(empty_block, false, &blocks);

        // Verify the block is actually empty
        assert!(empty_block.content.is_empty(), "Block should be empty");

        // Verify the block has children
        let has_children = blocks.iter().any(|b| b.page_id == page_id && b.parent_id == Some(parent_id));
        assert!(has_children, "Block should have children");

        assert_eq!(action, BackspaceAction::PromoteChildren);
    }

    #[test]
    fn test_calculate_promote_children() {
        let page_id = Uuid::new_v4();
        let parent_id = Uuid::new_v4();
        let child1_id = Uuid::new_v4();
        let child2_id = Uuid::new_v4();

        let blocks = vec![
            Block {
                id: parent_id,
                page_id,
                parent_id: None,
                order: 100.0,
                level: 1,
                content: BlockContent::empty(),
                ..Default::default()
            },
            Block {
                id: child1_id,
                page_id,
                parent_id: Some(parent_id),
                order: 100.0,
                level: 2,
                content: BlockContent::from_text("Child 1"),
                ..Default::default()
            },
            Block {
                id: child2_id,
                page_id,
                parent_id: Some(parent_id),
                order: 200.0,
                level: 2,
                content: BlockContent::from_text("Child 2"),
                ..Default::default()
            },
        ];

        let parent = blocks.iter().find(|b| b.id == parent_id).unwrap();
        let moves = OutlinerService::calculate_promote_children(parent, &blocks);

        assert_eq!(moves.len(), 2);
        // Children should now be at level 1 (same as deleted block)
        assert_eq!(moves[0].new_level, 1);
        assert_eq!(moves[1].new_level, 1);
        // Parent should be None (root level)
        assert_eq!(moves[0].new_parent_id, None);
        assert_eq!(moves[1].new_parent_id, None);
    }
}
