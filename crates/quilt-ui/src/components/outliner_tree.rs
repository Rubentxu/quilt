//! OutlinerTree component - displays hierarchical blocks
//!
//! This component takes a list of blocks and renders them as a tree
//! with proper indentation, expand/collapse, and keyboard navigation.

use crate::bridge::BlockDto;
use crate::components::slash_command::{SlashCommand, SlashCommandPalette};
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

    // Track expanded state for each block
    let expanded_map = RwSignal::new(HashMap::<String, bool>::new());

    // Slash command palette state
    let slash_open = RwSignal::new(false);
    let slash_query = RwSignal::new(String::new());
    let _slash_block_id = RwSignal::new(String::new()); // Track which block opened the palette

    // Handle slash command trigger
    let on_slash_command = move |query: String| {
        slash_query.set(query);
        slash_open.set(true);
    };

    // Handle command selection
    let on_slash_select = move |cmd: SlashCommand| {
        // TODO: Insert command.template into the block that triggered it
        // For now, just log the selection
        log::info!(
            "Slash command selected: {} with template: {:?}",
            cmd.id,
            cmd.template
        );
    };

    // Close slash palette
    let on_slash_close = move |_| {
        slash_open.set(false);
        slash_query.set(String::new());
    };

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

    // Get flat block IDs for keyboard navigation
    let flat_block_ids = Signal::derive(move || {
        flattened_blocks
            .get()
            .iter()
            .map(|t| t.block.id.clone())
            .collect::<Vec<_>>()
    });

    // Get expanded state for a block
    let get_expanded =
        move |block_id: &str| -> bool { expanded_map.get().get(block_id).copied().unwrap_or(true) };

    // Toggle expanded state
    let toggle_expanded = move |block_id: String| {
        expanded_map.update(|map| {
            let current = map.get(&block_id).copied().unwrap_or(true);
            map.insert(block_id, !current);
        });
    };

    // Focus next block
    let focus_next = move |current_id: String| {
        let ids = flat_block_ids.get();
        if let Some(pos) = ids.iter().position(|id| id == &current_id) {
            if pos + 1 < ids.len() {
                let next_id = &ids[pos + 1];
                if let Ok(Some(el)) =
                    document().query_selector(&format!("[data-block-id=\"{}\"]", next_id))
                {
                    use wasm_bindgen::JsCast;
                    if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                        let _ = html_el.focus();
                    }
                }
            }
        }
    };

    // Focus previous block
    let focus_prev = move |current_id: String| {
        let ids = flat_block_ids.get();
        if let Some(pos) = ids.iter().position(|id| id == &current_id) {
            if pos > 0 {
                let prev_id = &ids[pos - 1];
                if let Ok(Some(el)) =
                    document().query_selector(&format!("[data-block-id=\"{}\"]", prev_id))
                {
                    use wasm_bindgen::JsCast;
                    if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                        let _ = html_el.focus();
                    }
                }
            }
        }
    };

    view! {
        <div class="outliner-tree" data-block-tree>
            <For each={move || flattened_blocks.get()} key=|t| t.block.id.clone() let:item>
                {let item_id = item.block.id.clone(); let item_id2 = item_id.clone(); let item_id3 = item_id.clone(); let _item_id4 = item_id.clone(); let item_id5 = item_id.clone(); let item_id6 = item_id.clone(); view! {
                    <div
                        class="outliner-node"
                        data-block-id={item_id}
                    >
                        <OutlinerBlock
                            block={item.block.clone()}
                            has_children={!item.children.is_empty()}
                            expanded={RwSignal::new(get_expanded(&item_id2))}
                            on_collapse={Some(Callback::new(move |_| {
                                let id = item_id3.clone();
                                toggle_expanded(id);
                            }))}
                            on_focus_next={Some(Callback::new(move |_| {
                                let id = item_id5.clone();
                                focus_next(id);
                            }))}
                            on_focus_prev={Some(Callback::new(move |_| {
                                let id = item_id6.clone();
                                focus_prev(id);
                            }))}
                            on_slash_command={Some(Callback::new(on_slash_command))}
                        />
                    </div>
                }}
            </For>

            {/* Slash Command Palette */}
            <SlashCommandPalette
                is_open={slash_open}
                query={slash_query}
                on_select={Callback::new(on_slash_select)}
                on_close={Callback::new(on_slash_close)}
            />
        </div>
    }
}
