//! Block tree operations
//!
//! Converts flat block lists into hierarchical trees for rendering.
//! Handles ordering, parent-child relationships.

use crate::bridge::{BlockDto, BridgeError};
use crate::outliner::history::OutlinerCommand;
use std::fmt;

#[derive(Debug, Clone)]
pub enum TreeError {
    BlockNotFound,
    ParentNotFound,
    NoPreviousSibling,
    NoParent,
    NoNextSibling,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeError::BlockNotFound => write!(f, "Block not found"),
            TreeError::ParentNotFound => write!(f, "Parent not found"),
            TreeError::NoPreviousSibling => write!(f, "No previous sibling"),
            TreeError::NoParent => write!(f, "Block has no parent"),
            TreeError::NoNextSibling => write!(f, "No next sibling"),
        }
    }
}

impl std::error::Error for TreeError {}

impl From<TreeError> for BridgeError {
    fn from(err: TreeError) -> Self {
        match err {
            TreeError::BlockNotFound => BridgeError::BlockNotFound(String::new()),
            TreeError::ParentNotFound => BridgeError::BlockNotFound(String::new()),
            TreeError::NoPreviousSibling => BridgeError::BlockNotFound(String::new()),
            TreeError::NoParent => BridgeError::BlockNotFound(String::new()),
            TreeError::NoNextSibling => BridgeError::BlockNotFound(String::new()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub block: BlockDto,
    pub children: Vec<BlockNode>,
}

pub fn build_tree(blocks: &[BlockDto]) -> Vec<BlockNode> {
    if blocks.is_empty() {
        return vec![];
    }

    let mut nodes: Vec<BlockNode> = blocks
        .iter()
        .map(|b| BlockNode {
            block: b.clone(),
            children: vec![],
        })
        .collect();

    let mut root_indices: Vec<usize> = Vec::new();
    let mut parent_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for (i, node) in nodes.iter().enumerate() {
        parent_map.insert(node.block.id.clone(), i);
    }

    for i in (0..nodes.len()).rev() {
        if let Some(ref parent_id) = nodes[i].block.parent_id {
            if let Some(&parent_idx) = parent_map.get(parent_id) {
                if parent_idx != i {
                    let child = nodes.swap_remove(i);
                    if parent_idx > i {
                        nodes[parent_idx - 1].children.insert(0, child);
                    } else {
                        nodes[parent_idx].children.insert(0, child);
                    }
                    continue;
                }
            }
        }
        root_indices.push(i);
    }

    root_indices.sort();
    root_indices.into_iter().map(|i| nodes[i].clone()).collect()
}

pub fn flatten_tree(nodes: &[BlockNode]) -> Vec<&BlockDto> {
    let mut result = Vec::new();
    for node in nodes {
        result.push(&node.block);
        result.extend(flatten_tree(&node.children));
    }
    result
}

pub fn count_descendants(node: &BlockNode) -> usize {
    node.children.len() + node.children.iter().map(count_descendants).sum::<usize>()
}

fn find_previous_sibling(blocks: &[BlockDto], idx: usize) -> Option<usize> {
    if idx == 0 {
        return None;
    }
    let block = &blocks[idx];
    (0..idx)
        .rev()
        .find(|&i| blocks[i].parent_id == block.parent_id)
}

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

fn split_content(content: &str, cursor: usize) -> (String, String) {
    let mut chars = content.chars();
    let first: String = chars.by_ref().take(cursor).collect();
    let second: String = chars.collect();
    (first, second)
}

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

pub fn merge_content(
    blocks: &mut Vec<BlockDto>,
    target_id: &str,
    source_id: &str,
    _cursor_offset: u32,
) -> Result<BlockDto, TreeError> {
    let target_idx = blocks
        .iter()
        .position(|b| b.id == target_id)
        .ok_or(TreeError::BlockNotFound)?;
    let source_idx = blocks
        .iter()
        .position(|b| b.id == source_id)
        .ok_or(TreeError::BlockNotFound)?;

    let combined = format!(
        "{}{}",
        blocks[target_idx].content, blocks[source_idx].content
    );
    blocks[target_idx].content = combined;
    blocks.remove(source_idx);
    Ok(blocks[target_idx].clone())
}

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

/// Apply a structural `OutlinerCommand` to a mutable block list.
///
/// This is the mutation side of undo/redo for structural operations:
/// - `MergeBlock` → merge target+source content, remove source
/// - `SplitBlock` → split block content, insert new block
/// - `Indent` / `Outdent` → update block parent and order
///
/// Returns `true` if the command was recognized and applied.
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
                        // ti is now at ti - 1, but content is already set at ti
                        // which shifted to ti-1. This is correct because we
                        // set blocks[ti] before removal.
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

    // ── RED: MergeBlock removes source and merges content ──

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

    // ── RED: SplitBlock splits content and inserts new block ──

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

    // ── RED: Indent updates parent and order ──

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

    // ── RED: Outdent updates parent and order ──

    #[test]
    fn test_apply_outdent() {
        let mut blocks = vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", Some("b1"), "B", 2, 1.5),
        ];
        let cmd = OutlinerCommand::Outdent {
            block_id: "b2".into(),
            old_parent: None,
            old_order: 2.0,
            new_parent: Some("b1".into()),
            new_order: 1.5,
        };
        let ok = apply_structural_mutation(&mut blocks, &cmd);
        assert!(ok, "Outdent should succeed");
        assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        assert!((blocks[1].order - 1.5).abs() < f64::EPSILON);
    }

    // ── TRIANGULATE: Block not found returns false ──

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

    // ── TRIANGULATE: Non-structural command returns false ──

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

    // ── TRIANGULATE: Apply same command twice (idempotency concern) ──

    #[test]
    fn test_apply_merge_block_source_already_gone() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];
        // Source b2 was already removed — command should fail gracefully
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

    // ── TRIANGULATE: SplitBlock preserves block properties ──

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

    // ── TRIANGULATE: SplitBlock when original block missing ──

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

    // ── TRIANGULATE: Full round-trip: split → merge → split ──

    #[test]
    fn test_apply_split_then_merge_roundtrip() {
        let mut blocks = vec![make_block("b1", None, "Hello World", 1, 1.0)];

        // Split
        let split = OutlinerCommand::SplitBlock {
            block_id: "b1".into(),
            new_block_id: "b2".into(),
            first_part: "Hello".into(),
            second_part: " World".into(),
        };
        assert!(apply_structural_mutation(&mut blocks, &split));
        assert_eq!(blocks.len(), 2);

        // Merge back
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

    /// Helper: create a PageOutliner wired to apply_structural_mutation
    /// through a shared Vec<BlockDto>.
    fn make_mutation_outliner(shared: Arc<std::sync::Mutex<Vec<BlockDto>>>) -> PageOutliner {
        let sb = shared.clone();
        let apply = move |_: &str, _: &str| {
            // Content apply — not used in these structural tests
        };
        let sb2 = shared.clone();
        let structural_apply = move |cmd: &OutlinerCommand| {
            let mut blocks = sb2.lock().unwrap();
            apply_structural_mutation(&mut blocks, cmd);
        };
        PageOutliner::new_with_structural(100, apply, structural_apply)
    }

    // ── RED: SplitBlock undo merges blocks → redo splits again ──

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

        // Record split
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

        // Undo → merge back
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].content, "Hello World");
        }

        // Redo → split again
        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].content, "Hello");
            assert_eq!(blocks[1].id, "b2");
        }
    }

    // ── RED: Indent undo/redo restores parent and order ──

    #[test]
    fn test_structural_undo_redo_indent() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        // Record indent: b2 under b1
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

        // Undo → restore root
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
            assert!((blocks[1].order - 2.0).abs() < f64::EPSILON);
        }

        // Redo → indent again
        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        }
    }

    // ── RED: Outdent undo/redo restores parent and order ──

    #[test]
    fn test_structural_undo_redo_outdent() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", Some("b1"), "B", 2, 1.5),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        // Record outdent: b2 to root
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

        // Undo → restore parent
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
            assert!((blocks[1].order - 1.5).abs() < f64::EPSILON);
        }

        // Redo → outdent again
        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
        }
    }

    // ── TRIANGULATE: Content + structural interleaved undo/redo ──

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

        // 1. Content change
        outliner.record_content_change("b1", "Hello World", "Hello There", None);
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello There");
        }

        // 2. Split
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

        // 3. Undo structural (split → merge)
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].content, "Hello There");
        }

        // 4. Undo content
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello World");
        }

        // 5. Redo content
        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[0].content, "Hello There");
        }

        // 6. Redo structural
        assert!(outliner.redo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].content, "Hello");
        }
    }

    // ── TRIANGULATE: Undo/redo when nothing recorded ──

    #[test]
    fn test_structural_undo_redo_none() {
        let shared = Arc::new(std::sync::Mutex::new(vec![make_block(
            "b1", None, "A", 1, 1.0,
        )]));
        let outliner = make_mutation_outliner(shared.clone());
        assert!(!outliner.undo(), "no undo");
        assert!(!outliner.redo(), "no redo");
    }

    // ── TRIANGULATE: Multiple structural undos in reverse order ──

    #[test]
    fn test_multiple_structural_undos_reverse() {
        let shared = Arc::new(std::sync::Mutex::new(vec![
            make_block("b1", None, "A", 1, 1.0),
            make_block("b2", None, "B", 1, 2.0),
        ]));
        let outliner = make_mutation_outliner(shared.clone());

        // Indent b2 then outdent b2 back
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

        // Last was outdent: b2 at root
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
        }

        // Undo outdent → b2 under b1
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert_eq!(blocks[1].parent_id.as_deref(), Some("b1"));
        }

        // Undo indent → b2 at root, order=2.0
        assert!(outliner.undo());
        {
            let blocks = shared.lock().unwrap();
            assert!(blocks[1].parent_id.is_none());
            assert!((blocks[1].order - 2.0).abs() < f64::EPSILON);
        }
    }
}
