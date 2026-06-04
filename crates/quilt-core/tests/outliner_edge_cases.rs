//! Edge case tests for outliner tree operations.
//!
//! These tests cover corner cases that are CRITICAL because all outliner
//! operations depend on them, and the React UI + WASM bridge rely on them.

use quilt_core::outliner::history::OutlinerCommand;
use quilt_core::outliner::tree::{
    BlockNode, DropPosition, TreeError, apply_structural_mutation, build_tree,
    calculate_drop_position, count_descendants, flatten_tree, indent, is_descendant_of,
    merge_content, merge_with_next, merge_with_prev, outdent, split_block,
};
use quilt_core::types::BlockDto;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_block(id: &str, parent_id: Option<&str>, content: &str, level: u8, order: f64) -> BlockDto {
    BlockDto {
        id: id.to_string(),
        page_id: "page1".to_string(),
        parent_id: parent_id.map(String::from),
        content: content.to_string(),
        order,
        level,
        marker: None,
        priority: None,
        collapsed: false,
        properties: serde_json::json!({}),
        refs: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        created_by: None,
    }
}

fn flatten_ids(tree: &[BlockNode]) -> Vec<String> {
    flatten_tree(tree)
        .into_iter()
        .map(|b| b.id.clone())
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 1: build_tree — empty / edge input
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_tree_empty_blocks() {
    let tree = build_tree(&[]);
    assert!(tree.is_empty(), "empty input → empty tree");
}

#[test]
fn test_build_tree_single_root_block() {
    let blocks = vec![make_block("b1", None, "Root", 1, 1.0)];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].block.id, "b1");
    assert!(tree[0].children.is_empty(), "single root has no children");
}

#[test]
fn test_build_tree_orphaned_blocks() {
    // parent_id points to non-existent block → treated as root
    let blocks = vec![
        make_block("b1", Some("ghost"), "Orphan", 1, 1.0),
        make_block("b2", Some("ghost"), "Another orphan", 1, 2.0),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 2, "orphaned blocks become roots");
    assert_eq!(tree[0].block.id, "b1");
    assert_eq!(tree[1].block.id, "b2");
}

#[test]
fn test_build_tree_self_referencing_parent() {
    // Edge: self-referencing parent_id (parent_id == own id)
    let blocks = vec![make_block("b1", Some("b1"), "Self ref", 1, 1.0)];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1, "self-reference becomes root");
    assert_eq!(tree[0].block.id, "b1");
}

#[test]
fn test_build_tree_all_roots() {
    let blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
        make_block("b3", None, "C", 1, 3.0),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 3);
    assert_eq!(tree[0].block.id, "b1");
    assert_eq!(tree[1].block.id, "b2");
    assert_eq!(tree[2].block.id, "b3");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 2: build_tree — deeply nested structures
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_tree_deeply_nested() {
    // 10 levels of nesting: b1 → b2 → b3 → ... → b10
    let mut blocks = Vec::new();
    for i in 1..=10 {
        let parent = if i == 1 {
            None
        } else {
            Some(format!("b{}", i - 1))
        };
        blocks.push(BlockDto {
            id: format!("b{}", i),
            page_id: "page1".to_string(),
            parent_id: parent,
            content: format!("Level {}", i),
            order: 1.0,
            level: i as u8,
            marker: None,
            priority: None,
            collapsed: false,
            properties: serde_json::json!({}),
            refs: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            created_by: None,
        });
    }
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1, "single root in deep chain");
    assert_eq!(tree[0].block.id, "b1");

    // Navigate chain
    let mut current = &tree[0];
    for i in 2..=10 {
        assert_eq!(current.children.len(), 1, "each level has one child");
        current = &current.children[0];
        assert_eq!(current.block.id, format!("b{}", i));
    }
    assert!(current.children.is_empty(), "deepest node has no children");
}

#[test]
fn test_build_tree_single_root_with_children() {
    // Simple case: one root with multiple children — avoids the
    // stale root_indices bug that occurs with multiple roots.
    let blocks = vec![
        make_block("b1", None, "Root1", 1, 1.0),
        make_block("b2", Some("b1"), "R1C1", 2, 1.5),
        make_block("b3", Some("b1"), "R1C2", 2, 2.5),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1, "one root");
    assert_eq!(tree[0].block.id, "b1");
    assert_eq!(tree[0].children.len(), 2);
}

#[test]
fn test_build_tree_single_root_with_children_declared_after() {
    // Children declared after their parent
    let blocks = vec![
        make_block("b1", None, "Root", 1, 1.0),
        make_block("b2", Some("b1"), "C1", 2, 1.5),
        make_block("b3", Some("b1"), "C2", 2, 2.5),
        make_block("b4", Some("b1"), "C3", 2, 3.5),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].block.id, "b1");
    assert_eq!(tree[0].children.len(), 3);
}

#[test]
fn test_build_tree_sibling_order_preserved() {
    // build_tree sorts roots and children by the `order` field. Inputs
    // are emitted in the sorted order regardless of declaration order.
    let blocks = vec![
        make_block("b3", None, "Third", 1, 3.0),
        make_block("b1", None, "First", 1, 1.0),
        make_block("b2", None, "Second", 1, 2.0),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 3);
    assert_eq!(tree[0].block.id, "b1", "first by order (1.0)");
    assert_eq!(tree[1].block.id, "b2", "second by order (2.0)");
    assert_eq!(tree[2].block.id, "b3", "third by order (3.0)");
}

#[test]
fn test_build_tree_sibling_order_by_order_field() {
    // build_tree now sorts children by `order` ascending.
    let blocks = vec![
        make_block("b1", None, "Root", 1, 1.0),
        make_block("b1a", Some("b1"), "A", 2, 1.0), // first child declared
        make_block("b2", Some("b1"), "B", 2, 2.0),  // second child declared
        make_block("b3", Some("b1"), "C", 2, 3.0),  // third child declared (order=3)
        make_block("b4", Some("b1"), "D", 2, 4.0),  // fourth child declared (order=4)
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1);
    let child_ids: Vec<&str> = tree[0]
        .children
        .iter()
        .map(|n| n.block.id.as_str())
        .collect();
    assert_eq!(
        child_ids,
        &["b1a", "b2", "b3", "b4"],
        "children sorted by `order` field ascending"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Cycle handling — must not panic, must return a sensible tree
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_tree_handles_three_node_cycle_gracefully() {
    // Three-node cycle: b1 → b3 → b2 → b1.
    // Old algorithm panicked with "index out of bounds" because
    // `swap_remove` shrank the array and `root_indices` held stale
    // positions. The new algorithm detects the cycle in
    // `validate_no_cycles` and returns an empty tree.
    let blocks = vec![
        make_block("b1", Some("b3"), "A", 1, 1.0),
        make_block("b2", Some("b1"), "B", 2, 1.5),
        make_block("b3", Some("b2"), "C", 3, 1.6),
    ];
    let tree = build_tree(&blocks);
    assert!(tree.is_empty(), "cycle detected → empty tree, no panic");
}

#[test]
fn test_build_tree_handles_root_indices_bug_gracefully() {
    // Three roots + children. Old algorithm panicked on this pattern
    // because `root_indices` could hold stale positions after
    // `swap_remove`. The new algorithm doesn't use `swap_remove` and
    // always returns the right tree.
    let blocks = vec![
        make_block("b1", None, "Root1", 1, 1.0),
        make_block("b2", Some("b1"), "C1", 2, 1.5),
        make_block("b3", Some("b1"), "C2", 2, 2.5),
        make_block("b4", None, "Root2", 1, 3.0),
        make_block("b5", Some("b4"), "C3", 2, 3.5),
        make_block("b6", Some("b4"), "C4", 2, 4.0),
        make_block("b7", Some("b4"), "C5", 2, 4.5),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 2, "two roots, no panic");
    assert_eq!(tree[0].block.id, "b1", "root sorted by order");
    assert_eq!(tree[1].block.id, "b4");
    assert_eq!(tree[0].children.len(), 2, "b1 has 2 children");
    assert_eq!(tree[1].children.len(), 3, "b4 has 3 children");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 3: indent — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_indent_first_sibling_fails() {
    let mut blocks = vec![
        make_block("b1", None, "First", 1, 1.0),
        make_block("b2", None, "Second", 1, 2.0),
    ];
    let result = indent(&mut blocks, "b1");
    assert!(matches!(result, Err(TreeError::NoPreviousSibling)));
}

#[test]
fn test_indent_deeply_nested_previous() {
    // b3 should indent under b1 (previous root-level sibling)
    let mut blocks = vec![
        make_block("b1", None, "Root", 1, 1.0),
        make_block("b2", Some("b1"), "Child", 2, 1.5),
        make_block("b3", None, "Target", 1, 2.0),
    ];
    let result = indent(&mut blocks, "b3");
    assert!(result.is_ok(), "indent should succeed");
    // find_previous_sibling looks for same parent (root/None) with order < 2.0
    // b1 is at root, order=1.0 < 2.0 → b3 becomes child of b1
    assert_eq!(
        blocks[2].parent_id,
        Some("b1".to_string()),
        "b3 should be child of b1 (prev root sibling)"
    );
    assert_eq!(blocks[2].level, 2, "level increased");
}

#[test]
fn test_indent_root_level_block() {
    let mut blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
    ];
    let result = indent(&mut blocks, "b2");
    assert!(result.is_ok(), "indent root block with prev sibling");
    assert_eq!(blocks[1].parent_id, Some("b1".to_string()));
}

#[test]
fn test_indent_preserves_existing_children() {
    let mut blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
        make_block("b3", Some("b2"), "B child", 2, 2.5),
    ];
    let result = indent(&mut blocks, "b2");
    assert!(result.is_ok());
    assert_eq!(
        blocks[1].parent_id,
        Some("b1".to_string()),
        "b2 becomes child of b1"
    );
    // b3's parent should still be b2
    assert_eq!(
        blocks[2].parent_id,
        Some("b2".to_string()),
        "b3 child of b2 preserved"
    );
}

#[test]
fn test_indent_nonexistent_block() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = indent(&mut blocks, "ghost");
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_indent_consecutive_multiple_times() {
    let mut blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
        make_block("b3", None, "C", 1, 3.0),
    ];
    // Indent b2 under b1
    assert!(indent(&mut blocks, "b2").is_ok());
    assert_eq!(blocks[1].parent_id, Some("b1".to_string()));
    assert_eq!(blocks[1].level, 2);
    // indent sets order = previous_sibling.order + 0.001 = 1.0 + 0.001
    assert!(
        (blocks[1].order - 1.001).abs() < f64::EPSILON,
        "expected order ~1.001, got {}",
        blocks[1].order
    );

    // Indent b3 under b1 (b2 is now a child of b1, so b3's prev root sibling is b1)
    let result = indent(&mut blocks, "b3");
    assert!(result.is_ok(), "second indent should succeed");
    assert_eq!(blocks[2].parent_id, Some("b1".to_string()), "b3 under b1");
    assert_eq!(blocks[2].level, 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 4: outdent — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_outdent_root_block_fails() {
    let mut blocks = vec![make_block("b1", None, "Root", 1, 1.0)];
    let result = outdent(&mut blocks, "b1");
    assert!(matches!(result, Err(TreeError::NoParent)));
}

#[test]
fn test_outdent_preserves_children() {
    let mut blocks = vec![
        make_block("b1", None, "Grandparent", 1, 1.0),
        make_block("b2", Some("b1"), "Parent", 2, 1.5),
        make_block("b3", Some("b2"), "Child", 3, 1.6),
    ];
    let result = outdent(&mut blocks, "b2");
    assert!(result.is_ok(), "outdent should succeed");
    assert_eq!(blocks[1].parent_id, None, "b2 becomes root");
    assert_eq!(blocks[1].level, 1, "b2 level back to root");
    // b3's parent is still b2
    assert_eq!(
        blocks[2].parent_id,
        Some("b2".to_string()),
        "b3 child of b2 preserved"
    );
}

#[test]
fn test_outdent_nonexistent_block() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = outdent(&mut blocks, "ghost");
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_outdent_missing_parent_block() {
    // b2's parent is b1, but b1 doesn't exist in the slice (orphan)
    let mut blocks = vec![make_block("b2", Some("ghost"), "Orphan", 2, 1.5)];
    let result = outdent(&mut blocks, "b2");
    assert!(matches!(result, Err(TreeError::ParentNotFound)));
}

#[test]
fn test_outdent_multiple_levels() {
    // Outdent multiple times to go up the hierarchy
    let mut blocks = vec![
        make_block("b1", None, "Root", 1, 1.0),
        make_block("b2", Some("b1"), "Level 2", 2, 1.5),
        make_block("b3", Some("b2"), "Level 3", 3, 1.6),
    ];
    // b3 outdent → parent becomes b1
    assert!(outdent(&mut blocks, "b3").is_ok());
    assert_eq!(blocks[2].parent_id, Some("b1".to_string()));
    assert_eq!(blocks[2].level, 2);

    // b3 outdent again → root
    assert!(outdent(&mut blocks, "b3").is_ok());
    assert_eq!(blocks[2].parent_id, None);
    assert_eq!(blocks[2].level, 1);

    // b3 outdent again → NoParent error (already root)
    assert!(matches!(
        outdent(&mut blocks, "b3"),
        Err(TreeError::NoParent)
    ));
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 5: split_block — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_split_block_at_position_zero() {
    let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
    let result = split_block(&mut blocks, "b1", 0);
    assert!(result.is_ok());
    let (first, second) = result.unwrap();
    assert_eq!(first.content, "", "first block empty");
    assert_eq!(second.content, "Hello World", "all content in new block");
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].content, "");
    assert_eq!(blocks[1].content, "Hello World");
}

#[test]
fn test_split_block_at_end() {
    let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
    let result = split_block(&mut blocks, "b1", 11);
    assert!(result.is_ok());
    let (first, second) = result.unwrap();
    assert_eq!(first.content, "Hello World", "content stays in original");
    assert_eq!(second.content, "", "new block is empty");
    assert_eq!(blocks.len(), 2);
}

#[test]
fn test_split_block_in_middle() {
    let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
    let result = split_block(&mut blocks, "b1", 5);
    assert!(result.is_ok());
    let (first, second) = result.unwrap();
    assert_eq!(first.content, "Hello");
    assert_eq!(second.content, " World");
}

#[test]
fn test_split_empty_block() {
    let mut blocks = vec![make_block("b1", None, "", 1, 1.0)];
    let result = split_block(&mut blocks, "b1", 0);
    assert!(result.is_ok());
    assert_eq!(blocks.len(), 2, "empty block creates two empty blocks");
    assert_eq!(blocks[0].content, "");
    assert_eq!(blocks[1].content, "");
}

#[test]
fn test_split_block_with_unicode() {
    // Multi-byte characters — split at char boundary, not byte boundary
    let mut blocks = vec![make_block("b1", None, "Hello 世界 World", 1, 1.0)];
    let result = split_block(&mut blocks, "b1", 7);
    assert!(result.is_ok());
    let (first, second) = result.unwrap();
    // "Hello " (6 chars) + "世" (1 char) = 7 chars
    assert_eq!(
        first.content, "Hello 世",
        "unicode split at correct char boundary"
    );
    assert_eq!(second.content, "界 World", "unicode remainder correct");
}

#[test]
fn test_split_block_nonexistent() {
    let mut blocks = vec![make_block("b1", None, "Hello", 1, 1.0)];
    let result = split_block(&mut blocks, "ghost", 0);
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_split_block_preserves_parent_and_order() {
    let mut blocks = vec![make_block("b1", Some("page1"), "Hello World", 2, 1.5)];
    let result = split_block(&mut blocks, "b1", 5);
    assert!(result.is_ok());
    assert_eq!(
        blocks[0].parent_id,
        Some("page1".to_string()),
        "original parent preserved"
    );
    assert_eq!(
        blocks[1].parent_id,
        Some("page1".to_string()),
        "new block inherits parent"
    );
    assert_eq!(blocks[1].level, 2, "new block inherits level");
    assert!(blocks[1].order > 1.5, "new block has higher order");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 6: merge_with_prev — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_merge_first_block_no_prev() {
    let mut blocks = vec![make_block("b1", None, "First", 1, 1.0)];
    let result = merge_with_prev(&mut blocks, "b1");
    assert!(result.is_err(), "no previous for first block");
}

#[test]
fn test_merge_with_prev_content_concatenation() {
    let mut blocks = vec![
        make_block("b1", None, "Hello ", 1, 1.0),
        make_block("b2", None, "World!", 1, 2.0),
    ];
    let result = merge_with_prev(&mut blocks, "b2");
    assert!(result.is_ok());
    assert_eq!(blocks.len(), 1, "merged into one");
    assert_eq!(blocks[0].id, "b1", "target block kept");
    assert_eq!(blocks[0].content, "Hello World!", "content concatenated");
}

#[test]
fn test_merge_with_prev_merges_into_semantic_sibling() {
    // When the current block and its flat-array previous share the
    // same parent, the merge target is the flat-previous AND the
    // semantic-previous — they coincide.
    let mut blocks = vec![
        make_block("b1", None, "Parent", 1, 1.0),
        make_block("b2", Some("b1"), "Child of b1", 2, 1.5),
        make_block("b3", Some("b1"), "Target", 2, 2.0),
    ];
    let result = merge_with_prev(&mut blocks, "b3");
    assert!(result.is_ok());
    assert_eq!(blocks.len(), 2, "target removed");
    assert_eq!(blocks[0].id, "b1", "b1 unchanged");
    assert_eq!(blocks[1].id, "b2", "b2 is the semantic prev sibling");
    assert_eq!(
        blocks[1].content, "Child of b1Target",
        "content merged into the previous sibling under the same parent"
    );
}

#[test]
fn test_merge_with_prev_nonexistent_block() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_with_prev(&mut blocks, "ghost");
    assert!(result.is_err());
}

#[test]
fn test_merge_with_prev_uses_semantic_sibling_not_flat_prev() {
    // merge_with_prev must use the SEMANTIC previous sibling (same
    // parent_id, highest order below the current's), not the flat
    // array previous. The old `idx - 1` logic merged b3 into b2
    // (its flat previous) even though b2 is a child of b1, not a
    // sibling of b3.
    let mut blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", Some("b1"), "B", 2, 1.5),
        make_block("b3", None, "C", 1, 2.0),
    ];
    let result = merge_with_prev(&mut blocks, "b3");
    assert!(result.is_ok(), "merge_with_prev should succeed");
    assert_eq!(blocks.len(), 2, "one block merged away");
    // Semantic previous sibling of b3 (parent=None, order=2.0) is
    // b1 (parent=None, order=1.0). b2 is a child of b1, not a sibling.
    assert_eq!(
        blocks[0].content, "AC",
        "content merged into semantic prev (b1)"
    );
    assert_eq!(blocks[1].content, "B", "b2 unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 7: merge_with_next — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_merge_last_block_no_next() {
    let mut blocks = vec![make_block("b1", None, "Only", 1, 1.0)];
    let result = merge_with_next(&mut blocks, "b1");
    assert!(matches!(result, Err(TreeError::NoNextSibling)));
}

#[test]
fn test_merge_with_next_content_concatenation() {
    let mut blocks = vec![
        make_block("b1", None, "Hello ", 1, 1.0),
        make_block("b2", None, "World!", 1, 2.0),
    ];
    let result = merge_with_next(&mut blocks, "b1");
    assert!(result.is_ok());
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "Hello World!");
}

#[test]
fn test_merge_with_next_reparents_children_to_target() {
    // merge_with_next calls merge_content which now re-parents the
    // removed block's children to the target. The previous behavior
    // (orphaning the children) was a footgun: blocks with a missing
    // parent disappear from the rendered tree.
    let mut blocks = vec![
        make_block("b1", None, "Target", 1, 1.0),
        make_block("b2", None, "Next", 1, 2.0),
        make_block("b3", Some("b2"), "ChildOfNext", 2, 2.5),
    ];
    let result = merge_with_next(&mut blocks, "b1");
    assert!(result.is_ok());
    assert_eq!(blocks[0].content, "TargetNext", "merged content");
    assert_eq!(blocks.len(), 2, "source (b2) removed");
    // b3 was a child of b2 — re-parented to b1 after the merge.
    assert_eq!(
        blocks[1].parent_id.as_deref(),
        Some("b1"),
        "b3 re-parented to the surviving block"
    );
}

#[test]
fn test_merge_with_next_nonexistent_block() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_with_next(&mut blocks, "ghost");
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_merge_with_next_nested_next() {
    let mut blocks = vec![
        make_block("b1", None, "Target", 1, 1.0),
        make_block("b2", Some("b1"), "Nested", 2, 1.5),
    ];
    let result = merge_with_next(&mut blocks, "b1");
    assert!(result.is_ok(), "merge with nested next");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "TargetNested");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 8: apply_structural_mutation — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_apply_unknown_command_noop() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let cmd = OutlinerCommand::SetContent {
        block_id: "b1".into(),
        before: "A".into(),
        after: "B".into(),
    };
    let ok = apply_structural_mutation(&mut blocks, &cmd);
    assert!(!ok, "SetContent is not a structural command");
    assert_eq!(blocks[0].content, "A", "content unchanged");
}

#[test]
fn test_apply_indent_invalid_block_noop() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let cmd = OutlinerCommand::Indent {
        block_id: "ghost".into(),
        old_parent: None,
        old_order: 0.0,
        new_parent: Some("b1".into()),
        new_order: 1.5,
    };
    let ok = apply_structural_mutation(&mut blocks, &cmd);
    assert!(!ok, "indent on invalid block should return false");
}

#[test]
fn test_apply_outdent_root_noop() {
    // apply_structural_mutation just sets parent/order values from the command.
    // It does not validate that outdent on root makes semantic sense.
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let cmd = OutlinerCommand::Outdent {
        block_id: "b1".into(),
        old_parent: None,
        old_order: 1.0,
        new_parent: None,
        new_order: 1.0,
    };
    let ok = apply_structural_mutation(&mut blocks, &cmd);
    assert!(ok, "command updates parent/order without validation");
    assert_eq!(blocks[0].parent_id, None);
}

#[test]
fn test_apply_merge_block_source_before_target() {
    // Edge: source comes before target in the array
    let mut blocks = vec![
        make_block("b2", None, " World", 1, 1.0),
        make_block("b1", None, "Hello", 1, 2.0),
    ];
    let cmd = OutlinerCommand::MergeBlock {
        target_id: "b1".into(),
        source_id: "b2".into(),
        target_before: "Hello".into(),
        source_before: " World".into(),
    };
    let ok = apply_structural_mutation(&mut blocks, &cmd);
    assert!(ok, "merge with source before target should work");
    assert_eq!(blocks.len(), 1, "one block remains");
    assert_eq!(blocks[0].content, "Hello World", "content merged correctly");
}

#[test]
fn test_apply_split_block_then_reverse_indent() {
    // Multiple mutations in sequence
    let mut blocks = vec![
        make_block("b1", None, "Hello World", 1, 1.0),
        make_block("b2", None, "Second", 1, 2.0),
    ];

    // 1. Split b1
    let split = OutlinerCommand::SplitBlock {
        block_id: "b1".into(),
        new_block_id: "b1_split".into(),
        first_part: "Hello".into(),
        second_part: " World".into(),
    };
    assert!(apply_structural_mutation(&mut blocks, &split));
    assert_eq!(blocks.len(), 3);

    // Verify split result
    assert_eq!(blocks[0].id, "b1");
    assert_eq!(blocks[0].content, "Hello");
    assert_eq!(blocks[1].id, "b1_split");
    assert_eq!(blocks[1].content, " World");

    // 2. Indent b2 under b1
    let indent_cmd = OutlinerCommand::Indent {
        block_id: "b2".into(),
        old_parent: None,
        old_order: 2.0,
        new_parent: Some("b1".into()),
        new_order: 2.001,
    };
    assert!(apply_structural_mutation(&mut blocks, &indent_cmd));
    let b2_idx = blocks.iter().position(|b| b.id == "b2").unwrap();
    assert_eq!(blocks[b2_idx].parent_id, Some("b1".to_string()));

    // 3. Outdent b2 back
    let outdent_cmd = OutlinerCommand::Outdent {
        block_id: "b2".into(),
        old_parent: Some("b1".into()),
        old_order: 2.001,
        new_parent: None,
        new_order: 2.0,
    };
    assert!(apply_structural_mutation(&mut blocks, &outdent_cmd));
    let b2_idx = blocks.iter().position(|b| b.id == "b2").unwrap();
    assert!(blocks[b2_idx].parent_id.is_none());
}

#[test]
fn test_apply_autocomplete_insert_non_structural() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let cmd = OutlinerCommand::AutocompleteInsert {
        block_id: "b1".into(),
        before: "A".into(),
        after: "A!".into(),
        trigger: "page".into(),
    };
    let ok = apply_structural_mutation(&mut blocks, &cmd);
    assert!(!ok, "AutocompleteInsert is not structural");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 9: flatten_tree — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_flatten_empty_tree() {
    let flat = flatten_tree(&[]);
    assert!(flat.is_empty(), "empty tree → empty list");
}

#[test]
fn test_flatten_single_node() {
    let tree = vec![BlockNode {
        block: make_block("b1", None, "A", 1, 1.0),
        children: vec![],
    }];
    let flat = flatten_tree(&tree);
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0].id, "b1");
}

#[test]
fn test_flatten_deeply_nested_depth_first() {
    // Build tree:
    // b1
    //   b2
    //     b3
    //   b4
    let tree = vec![BlockNode {
        block: make_block("b1", None, "Root", 1, 1.0),
        children: vec![
            BlockNode {
                block: make_block("b2", Some("b1"), "Child1", 2, 1.5),
                children: vec![BlockNode {
                    block: make_block("b3", Some("b2"), "Grandchild", 3, 1.6),
                    children: vec![],
                }],
            },
            BlockNode {
                block: make_block("b4", Some("b1"), "Child2", 2, 2.5),
                children: vec![],
            },
        ],
    }];
    let ids = flatten_ids(&tree);
    assert_eq!(ids, &["b1", "b2", "b3", "b4"], "depth-first order");
}

#[test]
fn test_flatten_preserves_sibling_order() {
    let tree = vec![
        BlockNode {
            block: make_block("b3", None, "C", 1, 3.0),
            children: vec![],
        },
        BlockNode {
            block: make_block("b1", None, "A", 1, 1.0),
            children: vec![],
        },
        BlockNode {
            block: make_block("b2", None, "B", 1, 2.0),
            children: vec![],
        },
    ];
    let ids = flatten_ids(&tree);
    // Flatten preserves tree structure as-is (build_tree sorts roots)
    assert_eq!(ids, &["b3", "b1", "b2"]);
}

#[test]
fn test_flatten_multiple_roots() {
    let tree = vec![
        BlockNode {
            block: make_block("b1", None, "Root1", 1, 1.0),
            children: vec![BlockNode {
                block: make_block("b2", Some("b1"), "R1C1", 2, 1.5),
                children: vec![],
            }],
        },
        BlockNode {
            block: make_block("b3", None, "Root2", 1, 2.0),
            children: vec![],
        },
    ];
    let ids = flatten_ids(&tree);
    assert_eq!(ids, &["b1", "b2", "b3"], "multiple roots depth-first");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 10: is_descendant_of — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_descendant_of_direct_parent() {
    let blocks = vec![
        make_block("b1", None, "Parent", 1, 1.0),
        make_block("b2", Some("b1"), "Child", 2, 1.5),
    ];
    assert!(is_descendant_of(&blocks, "b1", "b2"), "direct parent-child");
}

#[test]
fn test_is_descendant_of_grandparent() {
    let blocks = vec![
        make_block("b1", None, "GP", 1, 1.0),
        make_block("b2", Some("b1"), "Parent", 2, 1.5),
        make_block("b3", Some("b2"), "Child", 3, 1.6),
    ];
    assert!(
        is_descendant_of(&blocks, "b1", "b3"),
        "grandparent-grandchild"
    );
}

#[test]
fn test_is_descendant_of_unrelated() {
    let blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
    ];
    assert!(!is_descendant_of(&blocks, "b1", "b2"), "unrelated roots");
}

#[test]
fn test_is_descendant_of_self() {
    let blocks = vec![make_block("b1", None, "Self", 1, 1.0)];
    // Current implementation returns true for self
    assert!(
        is_descendant_of(&blocks, "b1", "b1"),
        "self is descendant of self"
    );
}

#[test]
fn test_is_descendant_of_nonexistent_ancestor() {
    let blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    assert!(
        !is_descendant_of(&blocks, "ghost", "b1"),
        "non-existent ancestor"
    );
}

#[test]
fn test_is_descendant_of_nonexistent_descendant() {
    let blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    assert!(
        !is_descendant_of(&blocks, "b1", "ghost"),
        "non-existent descendant"
    );
}

#[test]
fn test_is_descendant_of_deep_chain() {
    let mut blocks = Vec::new();
    for i in 1..=20 {
        let parent = if i == 1 {
            None
        } else {
            Some(format!("b{}", i - 1))
        };
        blocks.push(BlockDto {
            id: format!("b{}", i),
            page_id: "page1".to_string(),
            parent_id: parent,
            content: format!("Block {}", i),
            order: 1.0,
            level: i as u8,
            marker: None,
            priority: None,
            collapsed: false,
            properties: serde_json::json!({}),
            refs: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            created_by: None,
        });
    }
    assert!(
        is_descendant_of(&blocks, "b1", "b20"),
        "b20 is descendant of b1"
    );
    assert!(
        is_descendant_of(&blocks, "b5", "b15"),
        "b15 is descendant of b5"
    );
    assert!(
        !is_descendant_of(&blocks, "b10", "b5"),
        "b5 is NOT descendant of b10"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 11: Cycle handling
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_descendant_of_valid_non_cycle() {
    // Normal case: follows parent chain correctly
    let blocks = vec![
        make_block("b1", None, "Root", 1, 1.0),
        make_block("b2", Some("b1"), "Mid", 2, 1.5),
        make_block("b3", Some("b2"), "Leaf", 3, 1.6),
    ];
    assert!(is_descendant_of(&blocks, "b1", "b3"), "chain traversal");
    assert!(
        !is_descendant_of(&blocks, "b2", "b1"),
        "b1 is NOT descendant of b2"
    );
}

#[test]
fn test_is_descendant_of_direct_cycle() {
    // Simple cycle: b1 <-> b2
    let blocks = vec![
        make_block("b1", Some("b2"), "A", 1, 1.0),
        make_block("b2", Some("b1"), "B", 1, 2.0),
    ];
    // b2's parent is b1 → true
    let result = is_descendant_of(&blocks, "b1", "b2");
    assert!(result, "in direct cycle, b2's parent is b1 → true");

    // b1's parent is b2 → is_descendant_of(b2, b1) follows: b1's parent = b2 → true
    let result2 = is_descendant_of(&blocks, "b2", "b1");
    assert!(result2, "in direct cycle, b1's parent is b2 → true");

    // is_descendant_of(b1, b1): self-check returns true early
    let self_check = is_descendant_of(&blocks, "b1", "b1");
    assert!(self_check, "self is descendant of self");
}

// ═══════════════════════════════════════════════════════════════════════════
// TEST 12: count_descendants — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_count_descendants_leaf() {
    let node = BlockNode {
        block: make_block("b1", None, "Leaf", 1, 1.0),
        children: vec![],
    };
    assert_eq!(count_descendants(&node), 0, "leaf has 0 descendants");
}

#[test]
fn test_count_descendants_one_child() {
    let node = BlockNode {
        block: make_block("b1", None, "Parent", 1, 1.0),
        children: vec![BlockNode {
            block: make_block("b2", Some("b1"), "Child", 2, 1.5),
            children: vec![],
        }],
    };
    assert_eq!(count_descendants(&node), 1);
}

#[test]
fn test_count_descendants_deeply_nested() {
    // b1 → b2 → b3 (each level has own children)
    let node = BlockNode {
        block: make_block("b1", None, "Root", 1, 1.0),
        children: vec![
            BlockNode {
                block: make_block("b2", Some("b1"), "Middle", 2, 1.5),
                children: vec![BlockNode {
                    block: make_block("b3", Some("b2"), "Leaf", 3, 1.6),
                    children: vec![],
                }],
            },
            BlockNode {
                block: make_block("b4", Some("b1"), "Sibling", 2, 2.5),
                children: vec![],
            },
        ],
    };
    // Total: b2 (1) + b3 (1) + b4 (1) = 3 descendants
    assert_eq!(count_descendants(&node), 3);
}

#[test]
fn test_count_descendants_wide_tree() {
    // b1 has 10 children, each leaf
    let children: Vec<BlockNode> = (2..=11)
        .map(|i| BlockNode {
            block: make_block(
                &format!("b{}", i),
                Some("b1"),
                &format!("Child {}", i),
                2,
                i as f64,
            ),
            children: vec![],
        })
        .collect();
    let node = BlockNode {
        block: make_block("b1", None, "Root", 1, 1.0),
        children,
    };
    assert_eq!(count_descendants(&node), 10);
}

// ═══════════════════════════════════════════════════════════════════════════
// BONUS: merge_content — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_merge_content_target_not_found() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_content(&mut blocks, "ghost", "b1", 0);
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_merge_content_source_not_found() {
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_content(&mut blocks, "b1", "ghost", 0);
    assert!(matches!(result, Err(TreeError::BlockNotFound)));
}

#[test]
fn test_merge_content_rejects_self_merge() {
    // Old code panicked with "index out of bounds" when target_id ==
    // source_id: the source was removed, then `blocks[target_idx]`
    // was indexed on an empty array. The new code returns
    // `Err(TreeError::SelfMerge)` and leaves the list untouched.
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_content(&mut blocks, "b1", "b1", 0);
    assert!(matches!(result, Err(TreeError::SelfMerge { ref id }) if id == "b1"));
    // Blocks are unchanged
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "A");
}

// ═══════════════════════════════════════════════════════════════════════════
// BONUS: calculate_drop_position — edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_drop_position_before_first() {
    let blocks = vec![
        make_block("b1", None, "First", 1, 1.0),
        make_block("b2", None, "Second", 1, 2.0),
    ];
    let (_parent, order) = calculate_drop_position(&blocks, "b1", "b3", DropPosition::Before);
    // prev_order = max of null (b1.order - 1.0 = 0.0) = 0.0
    // new_order = (0.0 + 1.0) / 2.0 = 0.5
    assert!(
        (order - 0.5).abs() < f64::EPSILON,
        "expected order ~0.5, got {}",
        order
    );
}

#[test]
fn test_drop_position_after_last() {
    let blocks = vec![
        make_block("b1", None, "First", 1, 1.0),
        make_block("b2", None, "Second", 1, 2.0),
    ];
    let (_parent, order) = calculate_drop_position(&blocks, "b2", "b3", DropPosition::After);
    // next_order = min of null (b2.order + 1.0 = 3.0) = 3.0
    // new_order = (2.0 + 3.0) / 2.0 = 2.5
    assert!(
        (order - 2.5).abs() < f64::EPSILON,
        "expected order ~2.5, got {}",
        order
    );
}

#[test]
fn test_drop_position_child_of_leaf() {
    let blocks = vec![make_block("b1", None, "Target", 1, 1.0)];
    let (parent, order) = calculate_drop_position(&blocks, "b1", "b2", DropPosition::Child);
    assert_eq!(parent, Some("b1".to_string()), "child: parent is target");
    assert!(
        (order - 1.0).abs() < f64::EPSILON,
        "first child order starts at 1.0"
    );
}

#[test]
fn test_drop_position_child_with_existing_children() {
    let blocks = vec![
        make_block("b1", None, "Parent", 1, 1.0),
        make_block("b2", Some("b1"), "Existing child", 2, 1.5),
    ];
    let (parent, order) = calculate_drop_position(&blocks, "b1", "b3", DropPosition::Child);
    assert_eq!(parent, Some("b1".to_string()));
    // max_order among children is 1.5, so new order = 1.5 + 1.0 = 2.5
    assert!(
        (order - 2.5).abs() < f64::EPSILON,
        "expected order ~2.5, got {}",
        order
    );
}

#[test]
fn test_drop_position_before_mid() {
    let blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
        make_block("b3", None, "C", 1, 3.0),
    ];
    let (_parent, order) = calculate_drop_position(&blocks, "b2", "b4", DropPosition::Before);
    // prev_order = 1.0 (b1), target_order = 2.0 → (1 + 2) / 2 = 1.5
    assert!(
        (order - 1.5).abs() < f64::EPSILON,
        "expected order ~1.5, got {}",
        order
    );
}

#[test]
fn test_drop_position_after_mid() {
    let blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
        make_block("b3", None, "C", 1, 3.0),
    ];
    let (_parent, order) = calculate_drop_position(&blocks, "b2", "b4", DropPosition::After);
    // next_order = 3.0 (b3), target_order = 2.0 → (2 + 3) / 2 = 2.5
    assert!(
        (order - 2.5).abs() < f64::EPSILON,
        "expected order ~2.5, got {}",
        order
    );
}

#[test]
fn test_drop_position_before_filter_self() {
    // Dropped block should be excluded from order calculation
    let blocks = vec![
        make_block("b1", None, "A", 1, 1.0),
        make_block("b2", None, "B", 1, 2.0),
    ];
    // Drop b1 before b2 — source and target are different, but filter excludes source
    let (_parent, order) = calculate_drop_position(&blocks, "b2", "b1", DropPosition::Before);
    // prev_order among blocks where parent=None, id != "b1" and id != "b2":
    // only b1 qualifies? No, b1 is excluded as source_id. So no blocks.
    // prev_order = 2.0 - 1.0 = 1.0
    // new_order = (1.0 + 2.0) / 2.0 = 1.5
    assert!(
        (order - 1.5).abs() < f64::EPSILON,
        "expected order ~1.5, got {}",
        order
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// REGRESSION TESTS — bugs fixed by the SOLID refactor
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn regression_build_tree_handles_two_node_cycle() {
    // Smallest possible cycle: A → B → A.
    // Old algorithm panicked with "index out of bounds". The new
    // algorithm detects the cycle via `validate_no_cycles` and
    // returns an empty tree (resilience over completeness).
    let blocks = vec![
        make_block("a", Some("b"), "A", 1, 1.0),
        make_block("b", Some("a"), "B", 1, 2.0),
    ];
    let tree = build_tree(&blocks);
    assert!(tree.is_empty(), "two-node cycle → empty tree, no panic");
}

#[test]
fn regression_build_tree_handles_self_reference_as_orphan() {
    // Self-referencing parent_id is treated as an orphan (root),
    // not a cycle. This keeps the block visible in the tree.
    let blocks = vec![make_block("b1", Some("b1"), "Self ref", 1, 1.0)];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 1, "self-reference becomes a root");
    assert_eq!(tree[0].block.id, "b1");
    assert!(tree[0].children.is_empty(), "no self-loop in children");
}

#[test]
fn regression_build_tree_handles_orphan_parent() {
    // parent_id points to a non-existent id ("ghost"). The new code
    // treats this as an orphan root, preserving the block.
    let blocks = vec![
        make_block("b1", Some("ghost"), "Orphan", 1, 1.0),
        make_block("b2", Some("ghost"), "Another orphan", 1, 2.0),
    ];
    let tree = build_tree(&blocks);
    assert_eq!(tree.len(), 2, "orphans become roots");
    assert_eq!(tree[0].block.id, "b1");
    assert_eq!(tree[1].block.id, "b2");
}

#[test]
fn regression_merge_content_rejects_self_merge() {
    // Self-merge (target == source) returns SelfMerge error, never panics.
    let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
    let result = merge_content(&mut blocks, "b1", "b1", 0);
    assert!(matches!(result, Err(TreeError::SelfMerge { ref id }) if id == "b1"));
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].content, "A");
}

#[test]
fn regression_merge_with_prev_finds_semantic_sibling() {
    // When the current block's flat previous is in a different
    // sub-tree, the merge must go to the SEMANTIC previous sibling
    // (same parent_id), not the flat previous.
    let mut blocks = vec![
        make_block("a", None, "A", 1, 1.0),
        make_block("b", Some("a"), "B", 2, 1.5),
        make_block("c", None, "C", 1, 2.0),
    ];
    let result = merge_with_prev(&mut blocks, "c");
    assert!(result.is_ok());
    // c's semantic prev sibling is `a` (parent=None, order=1.0),
    // not `b` (parent=a, different sub-tree).
    assert_eq!(blocks[0].content, "AC", "merged into semantic prev (a)");
    assert_eq!(blocks[1].content, "B", "b unchanged");
}

#[test]
fn regression_build_tree_sorts_children_by_order_field() {
    // Children of a parent are sorted by `order` ascending.
    let blocks = vec![
        make_block("parent", None, "P", 1, 1.0),
        make_block("d", Some("parent"), "D", 2, 4.0),
        make_block("a", Some("parent"), "A", 2, 1.0),
        make_block("c", Some("parent"), "C", 2, 3.0),
        make_block("b", Some("parent"), "B", 2, 2.0),
    ];
    let tree = build_tree(&blocks);
    let child_ids: Vec<&str> = tree[0]
        .children
        .iter()
        .map(|n| n.block.id.as_str())
        .collect();
    assert_eq!(
        child_ids,
        vec!["a", "b", "c", "d"],
        "children sorted by `order` field, not declaration order"
    );
}
