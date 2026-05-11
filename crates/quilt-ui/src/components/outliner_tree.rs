//! OutlinerTree component - displays hierarchical blocks
//!
//! This component takes a list of blocks and renders them as a tree
//! with proper indentation and expand/collapse.

use crate::bridge::BlockDto;
use crate::components::OutlinerBlock;
use leptos::prelude::*;
use std::collections::HashMap;

/// Tree node representation for efficient rendering
#[derive(Debug, Clone)]
pub struct TreeBlock {
    pub block: BlockDto,
    pub children: Vec<TreeBlock>,
}

/// Build a tree from flat block list
pub fn build_tree(blocks: &[BlockDto]) -> Vec<TreeBlock> {
    let mut block_map: HashMap<String, TreeBlock> = HashMap::new();
    let mut roots: Vec<TreeBlock> = Vec::new();

    // First pass: create all tree nodes
    for block in blocks {
        block_map.insert(
            block.id.clone(),
            TreeBlock {
                block: block.clone(),
                children: Vec::new(),
            },
        );
    }

    // Second pass: build tree structure
    let mut to_remove: Vec<String> = Vec::new();

    for block in blocks {
        if let Some(parent_id) = &block.parent_id {
            if block_map.contains_key(parent_id) && parent_id != &block.id {
                to_remove.push(block.id.clone());
            }
        }
    }

    // Now process removals
    for block_id in to_remove {
        if let Some(tree_node) = block_map.remove(&block_id) {
            if let Some(parent_id) = &tree_node.block.parent_id {
                if let Some(parent) = block_map.get_mut(parent_id) {
                    parent.children.push(tree_node);
                }
            }
        }
    }

    // Remaining nodes without parent are roots
    for (_, tree_node) in block_map {
        roots.push(tree_node);
    }

    // Sort roots by order
    roots.sort_by(|a, b| a.block.order.partial_cmp(&b.block.order).unwrap());

    // Sort children at each level
    fn sort_children(node: &mut TreeBlock) {
        node.children
            .sort_by(|a, b| a.block.order.partial_cmp(&b.block.order).unwrap());
        for child in &mut node.children {
            sort_children(child);
        }
    }

    for root in &mut roots {
        sort_children(root);
    }

    roots
}

/// OutlinerTree component - renders the full outliner tree
#[component]
pub fn OutlinerTree(blocks: Vec<BlockDto>) -> impl IntoView {
    let tree = Signal::derive(move || build_tree(&blocks));

    // Flatten tree for simpler rendering first
    let flattened_blocks = Signal::derive(move || {
        let mut result: Vec<TreeBlock> = Vec::new();
        fn flatten_nodes(nodes: &[TreeBlock], result: &mut Vec<TreeBlock>) {
            for node in nodes {
                result.push(node.clone());
                flatten_nodes(&node.children, result);
            }
        }
        flatten_nodes(&tree.get(), &mut result);
        result
    });

    view! {
        <div class="outliner-tree" data-block-tree>
            <For each={move || flattened_blocks.get()} key=|t| t.block.id.clone() let:item>
                <div class="outliner-node" data-block-id={item.block.id.clone()}>
                    <OutlinerBlock
                        block={item.block.clone()}
                        has_children={!item.children.is_empty()}
                        expanded={true}
                    />
                </div>
            </For>
        </div>
    }
}
