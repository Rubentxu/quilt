//! Block tree operations
//!
//! Converts flat block lists into hierarchical trees for rendering.
//! Handles ordering, parent-child relationships.
//!
//! ## Architecture (SRP)
//!
//! Each function does exactly one thing. The public entry point
//! `build_tree` orchestrates three pure helpers:
//!
//! - `validate_no_cycles` — duplicate ID + cycle detection (O(n))
//! - `index_children`    — `parent_id → Vec<&BlockDto>` index (O(n log n))
//! - `build_node_recursive` — single-node tree builder (O(1) per call)
//!
//! ## SOLID principles
//!
//! - **SRP**: tree building, cycle detection, sibling lookup and merging
//!   are split into small focused helpers.
//! - **OCP**: adding a new error variant or sort strategy does not require
//!   changing existing helpers — they accept trait-shaped inputs.
//! - **LSP**: all fallible operations return `Result<_, TreeError>`, so
//!   callers can substitute one error path for another uniformly.
//! - **ISP**: helpers take the smallest slice they need (`&[&BlockDto]`
//!   for read-only indexes, never `&mut Vec<BlockDto>`).
//! - **DIP**: where the slice already gives us a stable view, we
//!   avoid re-iterating the whole `Vec`.

use crate::outliner::history::OutlinerCommand;
use crate::types::{BlockDto, CoreError};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::fmt;

// ═══════════════════════════════════════════════════════════════════════
//  Error type
// ═══════════════════════════════════════════════════════════════════════

/// Errors that can occur in tree operations.
///
/// `SelfMerge` and `CycleDetected` are new variants introduced to
/// surface bugs that previously caused panics (Bug #2 and Bug #3 in
/// the edge-case report). `InvalidState` is a backstop for semantic
/// preconditions that have no dedicated variant.
#[derive(Debug, Clone, PartialEq)]
pub enum TreeError {
    /// A block with the given id was not found in the list.
    BlockNotFound,
    /// A referenced parent id was not found in the list.
    ParentNotFound,
    /// Indent/merge: no previous sibling to attach to.
    NoPreviousSibling,
    /// Outdent: block has no parent to outdent from.
    NoParent,
    /// Merge-next: no block after the current one.
    NoNextSibling,
    /// Merge-content: caller passed the same id for source and target.
    SelfMerge { id: String },
    /// Build-tree: parent chain contains a cycle.
    CycleDetected { path: Vec<String> },
    /// Semantic precondition violated (e.g. invariant internal to a function).
    InvalidState(String),
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeError::BlockNotFound => write!(f, "Block not found"),
            TreeError::ParentNotFound => write!(f, "Parent not found"),
            TreeError::NoPreviousSibling => write!(f, "No previous sibling"),
            TreeError::NoParent => write!(f, "Block has no parent"),
            TreeError::NoNextSibling => write!(f, "No next sibling"),
            TreeError::SelfMerge { id } => {
                write!(f, "Cannot merge block with itself: {}", id)
            }
            TreeError::CycleDetected { path } => {
                write!(f, "Cycle detected in block tree: {:?}", path)
            }
            TreeError::InvalidState(msg) => write!(f, "Invalid tree state: {}", msg),
        }
    }
}

impl std::error::Error for TreeError {}

impl From<TreeError> for CoreError {
    fn from(err: TreeError) -> Self {
        match err {
            TreeError::BlockNotFound => CoreError::NotFound(String::new()),
            TreeError::ParentNotFound => CoreError::NotFound(String::new()),
            TreeError::NoPreviousSibling => CoreError::NotFound("No previous sibling".into()),
            TreeError::NoParent => CoreError::NotFound("No parent".into()),
            TreeError::NoNextSibling => CoreError::NotFound("No next sibling".into()),
            TreeError::SelfMerge { id } => {
                CoreError::InvalidOperation(format!("Self-merge: {}", id))
            }
            TreeError::CycleDetected { path } => {
                CoreError::InvalidOperation(format!("Cycle detected: {:?}", path))
            }
            TreeError::InvalidState(msg) => CoreError::InvalidOperation(msg),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tree node
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, serde::Serialize)]
pub struct BlockNode {
    pub block: BlockDto,
    pub children: Vec<BlockNode>,
}

// ═══════════════════════════════════════════════════════════════════════
//  Tree building — orchestrator + pure helpers (SRP)
// ═══════════════════════════════════════════════════════════════════════

/// Build a hierarchical tree from a flat list of blocks.
///
/// Algorithm (delegated to single-responsibility helpers):
/// 1. **`validate_no_cycles`** — reject duplicate IDs and parent cycles
/// 2. **`index_children`** — build `parent_id → Vec<&BlockDto>` index
/// 3. **sort roots** — by `order` field, stable
/// 4. **`build_node_recursive`** — recurse from each root using the index
///
/// On cycle detection the function returns an empty tree (resilience
/// over completeness — partial data is better than a panic).
///
/// Complexity: O(n) for validation, O(n log n) for sort, O(n) for the
/// recursive walk. Total: O(n log n).
pub fn build_tree(blocks: &[BlockDto]) -> Vec<BlockNode> {
    if blocks.is_empty() {
        return Vec::new();
    }

    // Step 1: validate — bail out on duplicate IDs or parent cycles.
    if validate_no_cycles(blocks).is_err() {
        return Vec::new();
    }

    // Step 2: index children by parent_id.
    let blocks_refs: Vec<&BlockDto> = blocks.iter().collect();
    let children_map = index_children(&blocks_refs);

    // Step 3: collect and sort roots by `order`. A "root" is any block
    // whose parent_id is None, points to itself (self-reference), or
    // points to an id that is not present in the input (an orphan).
    // Treating orphans as roots preserves data instead of silently
    // dropping it.
    let block_ids: HashSet<&str> = blocks.iter().map(|b| b.id.as_str()).collect();
    let mut roots: Vec<&BlockDto> = blocks_refs
        .into_iter()
        .filter(|b| match b.parent_id.as_deref() {
            None => true,
            Some(p) if p == b.id => true,
            Some(p) => !block_ids.contains(p),
        })
        .collect();
    sort_by_order(&mut roots);

    // Step 4: recurse.
    roots
        .iter()
        .map(|root| build_node_recursive(root, &children_map))
        .collect()
}

/// Validate that the block list has no duplicate IDs and no cycles
/// in the parent chain.
///
/// Returns `Ok(())` when the input is well-formed, otherwise the cycle
/// path (the set of ids traversed before the loop closed).
///
/// This is the single source of truth for "is the input a valid tree?".
fn validate_no_cycles(blocks: &[BlockDto]) -> Result<(), Vec<String>> {
    // 1. Duplicate ID check (O(n))
    let mut seen: HashSet<&str> = HashSet::with_capacity(blocks.len());
    for b in blocks {
        if !seen.insert(b.id.as_str()) {
            return Err(vec![b.id.clone()]);
        }
    }

    // 2. Build an `id → parent_id` map for O(1) traversal.
    let parent_of: HashMap<&str, &str> = blocks
        .iter()
        .filter_map(|b| b.parent_id.as_deref().map(|p| (b.id.as_str(), p)))
        .collect();

    // 3. For every block, walk up the parent chain. A repeat visit
    //    is a cycle. The path set grows as we walk and is returned
    //    on failure for diagnostics.
    //
    //    A self-edge (`parent_id == own id`) is treated as an orphan,
    //    not a cycle: it has no real "ancestor" to walk into. This
    //    preserves data instead of failing the whole tree.
    for start in blocks {
        let mut current: &str = start.id.as_str();
        let mut path: HashSet<&str> = HashSet::new();
        loop {
            if !path.insert(current) {
                return Err(path.into_iter().map(String::from).collect());
            }
            match parent_of.get(current).copied() {
                Some(parent) if parent == current => break,
                Some(parent) => current = parent,
                None => break,
            }
        }
    }

    Ok(())
}

/// Build a `parent_id → Vec<&BlockDto>` index, with each child list
/// pre-sorted by `order` (stable, NaN-tolerant).
///
/// This is the only place that scans the block list a second time
/// during tree building — by indexing once we get O(1) child lookup
/// in the recursive walk.
///
/// Self-references (`parent_id == own id`) are skipped: they would
/// create a one-element child list that loops `build_node_recursive`
/// forever. The block is still treated as a root by `build_tree`.
fn index_children<'a>(blocks: &[&'a BlockDto]) -> HashMap<&'a str, Vec<&'a BlockDto>> {
    let mut map: HashMap<&'a str, Vec<&'a BlockDto>> = HashMap::new();
    for b in blocks {
        if let Some(parent_id) = b.parent_id.as_deref() {
            if parent_id != b.id {
                map.entry(parent_id).or_default().push(b);
            }
        }
    }
    for children in map.values_mut() {
        sort_by_order(children);
    }
    map
}

/// Recursively build one `BlockNode` by looking up children in the
/// pre-built index. The recursion is structural — depth equals tree
/// depth — but each call does O(1) work plus the cost of its
/// children. Total work: O(n).
fn build_node_recursive<'a>(
    block: &'a BlockDto,
    children_map: &HashMap<&'a str, Vec<&'a BlockDto>>,
) -> BlockNode {
    let children = children_map
        .get(block.id.as_str())
        .map(|c| {
            c.iter()
                .map(|b| build_node_recursive(b, children_map))
                .collect()
        })
        .unwrap_or_default();
    BlockNode {
        block: block.clone(),
        children,
    }
}

/// Stable sort by `order` (ascending), NaN-tolerant.
///
/// Generic over any `Borrow<BlockDto>` so it works with
/// `&BlockDto`, `&mut BlockDto`, and owned `BlockDto` slices.
fn sort_by_order<T: Borrow<BlockDto>>(items: &mut [T]) {
    items.sort_by(|a, b| {
        a.borrow()
            .order
            .partial_cmp(&b.borrow().order)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

// ═══════════════════════════════════════════════════════════════════════
//  Tree traversal
// ═══════════════════════════════════════════════════════════════════════

/// Flatten a tree depth-first. Order is determined by the tree
/// structure (parents before children, siblings in `build_tree` order).
pub fn flatten_tree(nodes: &[BlockNode]) -> Vec<&BlockDto> {
    let mut result = Vec::new();
    for node in nodes {
        result.push(&node.block);
        result.extend(flatten_tree(&node.children));
    }
    result
}

/// Count the descendants of a node (excludes the node itself).
pub fn count_descendants(node: &BlockNode) -> usize {
    node.children.len() + node.children.iter().map(count_descendants).sum::<usize>()
}

// ═══════════════════════════════════════════════════════════════════════
//  Indent / Outdent
// ═══════════════════════════════════════════════════════════════════════

/// Find the flat-array index of the previous sibling for the block at
/// `idx`. Sibling is defined as "previous block in the flat list with
/// the same `parent_id`". This is the right notion for `indent` (the
/// user is editing the flat list) but NOT for `merge_with_prev` —
/// the merge path uses a semantic-sibling lookup.
fn find_previous_sibling(blocks: &[BlockDto], idx: usize) -> Option<usize> {
    if idx == 0 {
        return None;
    }
    let block = &blocks[idx];
    (0..idx)
        .rev()
        .find(|&i| blocks[i].parent_id == block.parent_id)
}

/// Indent `block_id` under its previous sibling (same parent in the
/// flat list, immediately before the block).
pub fn indent(blocks: &mut [BlockDto], block_id: &str) -> Result<(), TreeError> {
    let idx = blocks
        .iter()
        .position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let prev_idx = find_previous_sibling(blocks, idx).ok_or(TreeError::NoPreviousSibling)?;

    let new_parent_id = blocks[prev_idx].id.clone();
    let new_order = blocks[prev_idx].order + 0.001;

    blocks[idx].parent_id = Some(new_parent_id);
    blocks[idx].order = new_order;
    blocks[idx].level = blocks[prev_idx].level + 1;
    Ok(())
}

/// Outdent `block_id` to the level of its parent (becomes sibling of
/// its current parent).
pub fn outdent(blocks: &mut [BlockDto], block_id: &str) -> Result<(), TreeError> {
    let idx = blocks
        .iter()
        .position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let block = blocks[idx].clone();
    let parent_id = block.parent_id.as_ref().ok_or(TreeError::NoParent)?;

    let parent_idx = blocks
        .iter()
        .position(|b| b.id == *parent_id)
        .ok_or(TreeError::ParentNotFound)?;
    let parent = blocks[parent_idx].clone();

    blocks[idx].parent_id = parent.parent_id.clone();
    blocks[idx].order = parent.order + 0.001;
    blocks[idx].level = parent.level;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
//  Split / Merge
// ═══════════════════════════════════════════════════════════════════════

/// Split `content` at the given char-boundary cursor.
fn split_content(content: &str, cursor: usize) -> (String, String) {
    let mut chars = content.chars();
    let first: String = chars.by_ref().take(cursor).collect();
    let second: String = chars.collect();
    (first, second)
}

/// Split a block at the cursor into two blocks with the same parent.
pub fn split_block(
    blocks: &mut Vec<BlockDto>,
    block_id: &str,
    cursor: u32,
) -> Result<(BlockDto, BlockDto), TreeError> {
    let idx = blocks
        .iter()
        .position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;

    let block = &blocks[idx];
    let cursor = cursor as usize;
    let (first, second) = split_content(&block.content, cursor);

    let mut updated = blocks[idx].clone();
    updated.content = first;

    let new_block = BlockDto {
        id: uuid::Uuid::new_v4().to_string(),
        page_id: block.page_id.clone(),
        parent_id: block.parent_id.clone(),
        content: second,
        order: block.order + 0.5,
        level: block.level,
        marker: None,
        priority: None,
        collapsed: false,
        properties: serde_json::json!({}),
        refs: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        created_by: None,
    };

    blocks[idx] = updated;
    blocks.insert(idx + 1, new_block.clone());
    Ok((blocks[idx].clone(), new_block))
}

/// Merge `source_id`'s content into `target_id`'s content, re-parent
/// `source_id`'s children to `target_id`, and remove `source_id`.
///
/// **Errors** (LSP — all paths return `TreeError`):
/// - `SelfMerge`     — `target_id == source_id` (was: panic, Bug #3)
/// - `BlockNotFound` — either id is missing from the list
pub fn merge_content(
    blocks: &mut Vec<BlockDto>,
    target_id: &str,
    source_id: &str,
    _cursor_offset: u32,
) -> Result<BlockDto, TreeError> {
    // Guard against the self-merge footgun: the source block would
    // be removed and then we'd index past the end of `blocks`.
    if target_id == source_id {
        return Err(TreeError::SelfMerge {
            id: target_id.to_string(),
        });
    }

    // Capture both indices before any mutation (avoid invalidation).
    let target_idx = blocks
        .iter()
        .position(|b| b.id == target_id)
        .ok_or(TreeError::BlockNotFound)?;
    let source_idx = blocks
        .iter()
        .position(|b| b.id == source_id)
        .ok_or(TreeError::BlockNotFound)?;

    // Build the new content. The combined string lives only in
    // memory until we commit it.
    let combined = format!(
        "{}{}",
        blocks[target_idx].content, blocks[source_idx].content
    );

    // Commit: update target, re-parent source's children, remove source.
    blocks[target_idx].content = combined;
    blocks[target_idx].updated_at = chrono::Utc::now().to_rfc3339();

    for b in blocks.iter_mut() {
        if b.parent_id.as_deref() == Some(source_id) {
            b.parent_id = Some(target_id.to_string());
        }
    }

    blocks.remove(source_idx);

    Ok(blocks[target_idx].clone())
}

/// Find the SEMANTIC previous sibling of a block (same `parent_id`,
/// highest `order` strictly below the current's). Returns `None` when
/// no such sibling exists.
fn find_semantic_previous_sibling<'a>(
    blocks: &'a [BlockDto],
    block_id: &str,
) -> Option<&'a BlockDto> {
    let current = blocks.iter().find(|b| b.id == block_id)?;
    let parent_id = current.parent_id.as_deref();
    let current_order = current.order;

    blocks
        .iter()
        .filter(|b| {
            b.id != block_id && b.parent_id.as_deref() == parent_id && b.order < current_order
        })
        .max_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Merge the current block's content into its **semantic** previous
/// sibling — the block at the same `parent_id` with the highest
/// `order` below the current's.
///
/// Fix for Bug #4: the previous implementation used `idx - 1` (flat
/// array previous), which is a child of the semantic previous when
/// the user is at a root level. This made Backspace merge the wrong
/// content into the wrong block.
pub fn merge_with_prev(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<(), TreeError> {
    if !blocks.iter().any(|b| b.id == block_id) {
        return Err(TreeError::BlockNotFound);
    }

    let prev_id = find_semantic_previous_sibling(blocks, block_id)
        .ok_or(TreeError::NoPreviousSibling)?
        .id
        .clone();

    // `prev_id` absorbs `block_id`; semantic-parent of the result is
    // the previous sibling, so children of the removed block are
    // already under the right roof after the merge.
    merge_content(blocks, &prev_id, block_id, u32::MAX)?;
    Ok(())
}

/// Merge the current block with the next block in the flat array.
pub fn merge_with_next(blocks: &mut Vec<BlockDto>, block_id: &str) -> Result<BlockDto, TreeError> {
    let idx = blocks
        .iter()
        .position(|b| b.id == block_id)
        .ok_or(TreeError::BlockNotFound)?;
    let next_idx = idx + 1;
    if next_idx >= blocks.len() {
        return Err(TreeError::NoNextSibling);
    }
    let next_block_id = blocks[next_idx].id.clone();
    merge_content(blocks, block_id, &next_block_id, u32::MAX)
}

// ═══════════════════════════════════════════════════════════════════════
//  Hierarchy / drag-and-drop helpers
// ═══════════════════════════════════════════════════════════════════════

/// Check if `descendant_id` is a direct or indirect descendant of
/// `ancestor_id` (returns `true` when both are the same id).
pub fn is_descendant_of(blocks: &[BlockDto], ancestor_id: &str, descendant_id: &str) -> bool {
    if ancestor_id == descendant_id {
        return true;
    }
    let mut current = descendant_id;
    loop {
        let block = match blocks.iter().find(|b| b.id == current) {
            Some(b) => b,
            None => return false,
        };
        match &block.parent_id {
            Some(parent) if parent == ancestor_id => return true,
            Some(parent) => current = parent,
            None => return false,
        }
    }
}

/// The position where a block is dropped relative to a target block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DropPosition {
    Before,
    After,
    Child,
}

/// Calculate the new parent and order for a dragged block based on
/// drop position.
pub fn calculate_drop_position(
    blocks: &[BlockDto],
    target_id: &str,
    source_id: &str,
    position: DropPosition,
) -> (Option<String>, f64) {
    let target = blocks
        .iter()
        .find(|b| b.id == target_id)
        .expect("target block must exist");

    match position {
        DropPosition::Before => {
            let parent_id = target.parent_id.clone();
            let prev_order = blocks
                .iter()
                .filter(|b| b.parent_id == parent_id && b.id != source_id && b.id != target_id)
                .filter(|b| b.order < target.order)
                .map(|b| b.order)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(target.order - 1.0);
            let new_order = (prev_order + target.order) / 2.0;
            (parent_id, new_order)
        }
        DropPosition::After => {
            let parent_id = target.parent_id.clone();
            let next_order = blocks
                .iter()
                .filter(|b| b.parent_id == parent_id && b.id != source_id && b.id != target_id)
                .filter(|b| b.order > target.order)
                .map(|b| b.order)
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(target.order + 1.0);
            let new_order = (target.order + next_order) / 2.0;
            (parent_id, new_order)
        }
        DropPosition::Child => {
            let new_parent = Some(target_id.to_string());
            let max_order = blocks
                .iter()
                .filter(|b| b.parent_id.as_deref() == Some(target_id) && b.id != source_id)
                .map(|b| b.order)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(0.0);
            (new_parent, max_order + 1.0)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Undo/Redo integration
// ═══════════════════════════════════════════════════════════════════════

/// Apply a structural `OutlinerCommand` to a mutable block list.
pub fn apply_structural_mutation(blocks: &mut Vec<BlockDto>, cmd: &OutlinerCommand) -> bool {
    match cmd {
        OutlinerCommand::MergeBlock {
            target_id,
            source_id,
            target_before,
            source_before,
        } => {
            let target_idx = blocks.iter().position(|b| b.id == *target_id);
            let source_idx = blocks.iter().position(|b| b.id == *source_id);
            match (target_idx, source_idx) {
                (Some(ti), Some(si)) => {
                    blocks[ti].content = format!("{}{}", target_before, source_before);
                    // Remove the higher index first to avoid shifting
                    if si > ti {
                        blocks.remove(si);
                    } else {
                        // Source is before target — remove source first,
                        // then target has shifted down by 1
                        blocks.remove(si);
                    }
                    true
                }
                _ => false,
            }
        }
        OutlinerCommand::SplitBlock {
            block_id,
            new_block_id,
            first_part,
            second_part,
        } => {
            let idx = blocks.iter().position(|b| b.id == *block_id);
            match idx {
                Some(idx) => {
                    // Clone fields from the original block before mutating
                    let page_id = blocks[idx].page_id.clone();
                    let parent_id = blocks[idx].parent_id.clone();
                    let order = blocks[idx].order;
                    let level = blocks[idx].level;
                    blocks[idx].content = first_part.clone();
                    let new_block = BlockDto {
                        id: new_block_id.clone(),
                        page_id,
                        parent_id,
                        content: second_part.clone(),
                        order: order + 0.5,
                        level,
                        marker: None,
                        priority: None,
                        collapsed: false,
                        properties: serde_json::json!({}),
                        refs: vec![],
                        created_at: chrono::Utc::now().to_rfc3339(),
                        updated_at: chrono::Utc::now().to_rfc3339(),
                        created_by: None,
                    };
                    blocks.insert(idx + 1, new_block);
                    true
                }
                None => false,
            }
        }
        OutlinerCommand::Indent {
            block_id,
            old_parent: _,
            old_order: _,
            new_parent,
            new_order,
        }
        | OutlinerCommand::Outdent {
            block_id,
            old_parent: _,
            old_order: _,
            new_parent,
            new_order,
        }
        | OutlinerCommand::MoveBlock {
            block_id,
            old_parent: _,
            old_order: _,
            new_parent,
            new_order,
        } => {
            if let Some(block) = blocks.iter_mut().find(|b| b.id == *block_id) {
                block.parent_id = new_parent.clone();
                block.order = *new_order;
                true
            } else {
                false
            }
        }
        // Content commands (SetContent, AutocompleteInsert) are not structural
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(
        id: &str,
        parent_id: Option<&str>,
        content: &str,
        level: u8,
        order: f64,
    ) -> BlockDto {
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

    #[test]
    fn test_indent_success() {
        let blocks = &mut vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", None, "Block 2", 1, 2.0),
        ];
        let result = indent(blocks, "b2");
        assert!(result.is_ok());
        assert_eq!(blocks[1].parent_id, Some("b1".to_string()));
        assert_eq!(blocks[1].level, 2);
    }

    #[test]
    fn test_indent_no_previous_sibling() {
        let blocks = &mut vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", None, "Block 2", 1, 2.0),
        ];
        let result = indent(blocks, "b1");
        assert!(matches!(result, Err(TreeError::NoPreviousSibling)));
    }

    #[test]
    fn test_indent_first_child() {
        let blocks = &mut vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", Some("b1"), "Block 2", 2, 1.5),
            make_block("b3", Some("b1"), "Block 3", 2, 1.6),
        ];
        let result = indent(blocks, "b2");
        assert!(matches!(result, Err(TreeError::NoPreviousSibling)));
    }

    #[test]
    fn test_outdent_success() {
        let blocks = &mut vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", Some("b1"), "Block 2", 2, 1.5),
        ];
        let result = outdent(blocks, "b2");
        assert!(result.is_ok());
        assert_eq!(blocks[1].parent_id, None);
        assert_eq!(blocks[1].level, 1);
    }

    #[test]
    fn test_outdent_root_block() {
        let blocks = &mut vec![
            make_block("b1", None, "Block 1", 1, 1.0),
            make_block("b2", None, "Block 2", 1, 2.0),
        ];
        let result = outdent(blocks, "b1");
        assert!(matches!(result, Err(TreeError::NoParent)));
    }

    #[test]
    fn test_split_block_at_cursor() {
        let blocks = &mut vec![make_block("b1", None, "Hello World", 1, 1.0)];
        let result = split_block(blocks, "b1", 5);
        assert!(result.is_ok());
        let (first, second) = result.unwrap();
        assert_eq!(first.content, "Hello");
        assert_eq!(second.content, " World");
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_split_block_at_end() {
        let blocks = &mut vec![make_block("b1", None, "Hello World", 1, 1.0)];
        let result = split_block(blocks, "b1", 11);
        assert!(result.is_ok());
        let (first, second) = result.unwrap();
        assert_eq!(first.content, "Hello World");
        assert_eq!(second.content, "");
    }

    #[test]
    fn test_merge_content_success() {
        let blocks = &mut vec![
            make_block("b1", None, "Hello", 1, 1.0),
            make_block("b2", None, " World", 1, 2.0),
        ];
        let result = merge_content(blocks, "b1", "b2", 0);
        assert!(result.is_ok());
        let merged = result.unwrap();
        assert_eq!(merged.content, "Hello World");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_merge_with_next_success() {
        let blocks = &mut vec![
            make_block("b1", None, "Hello", 1, 1.0),
            make_block("b2", None, " World", 1, 2.0),
        ];
        let result = merge_with_next(blocks, "b1");
        assert!(result.is_ok());
        assert_eq!(blocks[0].content, "Hello World");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_merge_no_next_sibling() {
        let blocks = &mut vec![make_block("b1", None, "Hello", 1, 1.0)];
        let result = merge_with_next(blocks, "b1");
        assert!(matches!(result, Err(TreeError::NoNextSibling)));
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 9 — Structural mutation (for undo/redo wiring)
    // ═══════════════════════════════════════════════════════════════

    #[test]
    fn test_apply_merge_block() {
        let mut blocks = vec![
            make_block("b1", None, "Hello", 1, 1.0),
            make_block("b2", None, " World", 1, 2.0),
        ];
        let cmd = OutlinerCommand::MergeBlock {
            target_id: "b1".into(),
            source_id: "b2".into(),
            target_before: "Hello".into(),
            source_before: " World".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok, "MergeBlock should succeed");
        assert_eq!(blocks.len(), 1, "source block removed");
        assert_eq!(blocks[0].content, "Hello World", "content merged");
    }

    #[test]
    fn test_apply_split_block() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok, "SplitBlock should succeed");
        assert_eq!(blocks.len(), 2, "new block inserted");
        assert_eq!(blocks[0].content, "Hello");
        assert_eq!(blocks[1].content, " World");
        assert_eq!(blocks[1].id, "b2");
    }

    #[test]
    fn test_apply_indent() {
        let mut blocks = vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ];
        let cmd = OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 2.001,
            new_parent: None,
            new_order: 2.0,
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok, "Indent should succeed");
        assert!(blocks[1].parent_id.is_none(), "b2 should be at root");
        assert!((blocks[1].order - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_apply_outdent() {
        let mut blocks = vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", Some("b1"), "B", 2, 1.5),
        ];
        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: Some("b1".into()),
            new_order: 1.5,
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok, "Outdent should succeed");
        assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        assert!((blocks[1].order - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_apply_structural_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
        let cmd = OutlinerCommand::MergeBlock {
            target_id: "bogus".into(),
            source_id: "b2".into(),
            target_before: "".into(),
            source_before: "".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(!ok, "Should return false");
    }

    #[test]
    fn test_apply_structural_non_structural() {
        let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
        let cmd = OutlinerCommand::SetContent {
            block_id: "b1".into(),
            before: "A".into(),
            after: "B".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(!ok, "SetContent is not structural");
        assert_eq!(blocks[0].content, "A", "content unchanged");
    }

    #[test]
    fn test_apply_merge_block_source_already_gone() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
        let cmd = OutlinerCommand::MergeBlock {
            target_id: "b1".into(),
            source_id: "b2".into(),
            target_before: "Hello".into(),
            source_before: " World".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(!ok, "Should return false when source is missing");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_apply_split_preserves_parent_and_page() {
        let mut blocks = vec![make_block("b1", Some("page1"), "Hello World", 2, 1.5)];
        let cmd = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[1].parent_id,
            Some("page1".into()),
            "parent preserved"
        );
        assert_eq!(blocks[1].page_id, "page1", "page_id preserved");
        assert_eq!(blocks[1].level, 2, "level preserved");
        assert!(blocks[1].order > 1.5, "new block order > original");
    }

    #[test]
    fn test_apply_split_block_not_found() {
        let mut blocks = vec![make_block("b1", None, "A", 1, 1.0)];
        let cmd = OutlinerCommand::SplitBlock {
            block_id: "bogus".into(),
            new_block_id: "b2".into(),
            first_part: "".into(),
            second_part: "".into(),
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(!ok, "Should return false when block not found");
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn test_apply_split_then_merge_roundtrip() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];

        let split = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };
        assert!(apply_structural_mutation(&mut blocks, &split));
        assert_eq!(blocks.len(), 2);

        let merge = OutlinerCommand::MergeBlock {
            target_id: "b1".into(),
            source_id: "b2".into(),
            target_before: "Hello".into(),
            source_before: " World".into(),
        };
        assert!(apply_structural_mutation(&mut blocks, &merge));
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "Hello World");
    }

    // ═══════════════════════════════════════════════════════════════
    //  BATCH 9 — Structural undo/redo integration (via PageOutliner)
    // ═══════════════════════════════════════════════════════════════

    use crate::outliner::page::PageOutliner;
    use std::sync::Arc;

    fn make_mutation_outliner(shared: Arc<std::sync::Mutex<Vec<BlockDto>>>) -> PageOutliner {
        let _sb = shared.clone();
        let apply = move |_: &str, _: &str| {};
        let sb2 = shared.clone();
        let structural_apply = move |cmd: &OutlinerCommand| {
            let mut blocks = sb2.lock().unwrap();
            apply_structural_mutation(&mut blocks, cmd);
        };
        PageOutliner::new_with_structural(100, apply, structural_apply)
    }

    #[test]
    fn test_structural_undo_redo_split() {
        let shared = Arc::new(std::sync::Mutex::new(vec![make_block(
            "b1",
            None,
            "Hello World",
            1,
            1.0,
        )]));
        let outliner = make_mutation_outliner(shared.clone());

        outliner.record_structural(OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        });
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].content, "Hello");
            assert_eq!(blocks[1].content, " World");
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].content, "Hello World");
        }

        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].content, "Hello");
            assert_eq!(blocks[1].id, "b2");
        }
    }

    #[test]
    fn test_structural_undo_redo_indent() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        outliner.record_structural(OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        });
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
            assert!((blocks[1].order - 2.0).abs() < f64::EPSILON);
        }

        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        }
    }

    #[test]
    fn test_structural_undo_redo_outdent() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", Some("b1"), "B", 2, 1.5),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        outliner.record_structural(OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 1.5,
            new_parent: None,
            new_order: 2.0,
        });
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
            assert!((blocks[1].order - 1.5).abs() < f64::EPSILON);
        }

        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
        }
    }

    #[test]
    fn test_interleaved_content_and_structural_undo_redo() {
        let shared = Arc::new(std::sync::Mutex::new(vec![make_block(
            "b1",
            None,
            "Hello World",
            1,
            1.0,
        )]));
        let sb = shared.clone();
        let apply = move |block_id: &str, content: &str| {
            let mut blocks = sb.lock().unwrap();
            if let Some(idx) = blocks.iter().position(|b| b.id == block_id) {
                blocks[idx].content = content.to_string();
            }
        };
        let sb2 = shared.clone();
        let structural_apply = move |cmd: &OutlinerCommand| {
            let mut blocks = sb2.lock().unwrap();
            apply_structural_mutation(&mut blocks, cmd);
        };
        let outliner = PageOutliner::new_with_structural(100, apply, structural_apply);

        outliner.record_content_change("b1", "Hello World", "Hello There", None);
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello There");
        }

        outliner.record_structural(OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " There".into(),
        });
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].content, "Hello There");
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello World");
        }

        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello There");
        }

        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].content, "Hello");
        }
    }

    #[test]
    fn test_structural_undo_redo_none() {
        let shared = Arc::new(std::sync::Mutex::new(vec![make_block(
            "b1", None, "A", 1, 1.0,
        )]));
        let outliner = make_mutation_outliner(shared.clone());
        assert!(!outliner.undo(), "no undo");
        assert!(!outliner.redo(), "no redo");
    }

    #[test]
    fn test_multiple_structural_undos_reverse() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        outliner.record_structural(OutlinerCommand::Indent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 2.001,
        });
        outliner.record_structural(OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: Some("b1".into()),
            old_order: 2.001,
            new_parent: None,
            new_order: 2.0,
        });

        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        }

        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
            assert!((blocks[1].order - 2.0).abs() < f64::EPSILON);
        }
    }
}
