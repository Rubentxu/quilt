use crate::bridge;
use crate::bridge::BlockDto;
use crate::components::block::Block;
use crate::components::loading::Loading;
use crate::outliner::history::OutlinerCommand;
use crate::outliner::page::PageOutliner;
use crate::outliner::selection::SelectionState;
use crate::outliner::tree::apply_structural_mutation;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use web_sys::KeyboardEvent;

#[component]
pub fn PageView() -> impl IntoView {
    let params = use_params_map();
    let page_name = move || {
        params
            .get()
            .get("name")
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let (blocks, set_blocks) = signal(Vec::<crate::bridge::BlockDto>::new());
    let (loading, set_loading) = signal(true);

    // Fetch page names for autocomplete (page ref suggestions).
    let page_names = RwSignal::new(Vec::<String>::new());
    provide_context(page_names);

    // Create SelectionState for keyboard-first navigation and provide as context.
    let selection_state = SelectionState::new();
    provide_context(selection_state);

    // Create the PageOutliner coordinator with both a content-applier callback
    // and a structural-applier callback. Both update the blocks signal.
    let page_outliner = {
        let set_blocks_a = set_blocks;
        let set_blocks_b = set_blocks;
        let apply = move |block_id: &str, content: &str| {
            let id = block_id.to_string();
            let c = content.to_string();
            set_blocks_a.update(|blocks_mut| {
                if let Some(idx) = blocks_mut.iter().position(|b| b.id == id) {
                    blocks_mut[idx].content = c;
                }
            });
        };
        let structural_apply = move |cmd: &OutlinerCommand| {
            set_blocks_b.update(|blocks_mut| {
                apply_structural_mutation(blocks_mut, cmd);
            });
        };
        PageOutliner::new_with_structural(100, apply, structural_apply)
    };
    provide_context(page_outliner);

    // ── Data loading effect ──
    Effect::new(move || {
        let name = page_name();
        let pn = page_names;
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            match bridge::get_page_blocks(&name).await {
                Ok(b) => set_blocks.set(b),
                Err(_) => set_blocks.set(vec![]),
            }
            if let Ok(pages) = bridge::list_pages().await {
                let names: Vec<String> = pages.into_iter().map(|p| p.name).collect();
                pn.set(names);
            }
            set_loading.set(false);
        });
    });

    // ── Keyboard handler for block navigation ──
    // NOTE: When a CM6 editor is active, Mod+{key} shortcuts work globally.
    // Plain arrow keys and Enter only work when no block is being edited.
    let on_page_keydown = {
        move |ev: KeyboardEvent| {
            let is_editing = selection_state.editing_block_id.get_untracked().is_some();
            let meta = ev.meta_key() || ev.ctrl_key();

            // ── Mod+ shortcuts: work even when editing ──
            if meta {
                match ev.key().as_str() {
                    "Enter" => {
                        ev.prevent_default();
                        cycle_block_marker(&selection_state, &blocks, &set_blocks);
                        return;
                    }
                    "ArrowUp" => {
                        ev.prevent_default();
                        collapse_selected_block(&selection_state, &blocks);
                        return;
                    }
                    "ArrowDown" => {
                        ev.prevent_default();
                        expand_selected_block(&selection_state, &blocks);
                        return;
                    }
                    ";" => {
                        ev.prevent_default();
                        toggle_collapse_selected_block(&selection_state, &blocks);
                        return;
                    }
                    _ => {}
                }
                return;
            }

            // ── Plain keys: only when NOT editing ──
            // CM6 handles its own arrow keys, Enter, Escape, etc.
            if is_editing {
                return;
            }

            match ev.key().as_str() {
                "ArrowUp" => {
                    ev.prevent_default();
                    navigate_selection(&selection_state, &blocks, -1);
                }
                "ArrowDown" => {
                    ev.prevent_default();
                    navigate_selection(&selection_state, &blocks, 1);
                }
                "ArrowLeft" => {
                    ev.prevent_default();
                    collapse_or_parent(&selection_state, &blocks, &set_blocks);
                }
                "ArrowRight" => {
                    ev.prevent_default();
                    expand_or_child(&selection_state, &blocks);
                }
                "Enter" => {
                    ev.prevent_default();
                    if let Some(ref sel_id) = selection_state.selected_block_id.get_untracked() {
                        selection_state.request_edit(sel_id);
                    }
                }
                "Escape" => {
                    selection_state.deselect();
                }
                _ => {}
            }
        }
    };

    view! {
        <div class="page-view">
            <h1 class="text-2xl font-bold mb-6">
                {move || page_name()}
            </h1>

            <Show when=move || loading.get()>
                <Loading />
            </Show>

            <Show
                when=move || !loading.get() && !blocks.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !loading.get()>
                        <div class="text-text-muted text-sm py-4">
                            "This page is empty. Start writing..."
                        </div>
                    </Show>
                }
            >
                <div
                    class="outliner"
                    tabindex="0"
                    on:keydown=on_page_keydown
                >
                    <For each=move || blocks.get() key=|b| b.id.clone() let:block>
                        <Block block=Signal::derive(move || block.clone()) blocks=blocks set_blocks=set_blocks children=vec![] />
                    </For>
                </div>
            </Show>
        </div>
    }
}

// ── Keyboard navigation helpers ──

/// Move the selection up or down by `delta` in the flat block list.
fn navigate_selection(sel: &SelectionState, blocks: &ReadSignal<Vec<BlockDto>>, delta: isize) {
    let list = blocks.get_untracked();
    if list.is_empty() {
        return;
    }
    let current = sel.selected_block_id.get_untracked();
    let new_idx = match current {
        None => {
            if delta > 0 {
                0
            } else {
                list.len() - 1
            }
        }
        Some(ref id) => {
            let idx = list.iter().position(|b| b.id == *id);
            match idx {
                Some(i) => {
                    let next = i as isize + delta;
                    if next < 0 {
                        0
                    } else if next >= list.len() as isize {
                        list.len() - 1
                    } else {
                        next as usize
                    }
                }
                None => 0,
            }
        }
    };
    sel.select(&list[new_idx].id);
}

/// Collapse the selected block, or if already collapsed / no children,
/// move selection to its parent.
fn collapse_or_parent(
    sel: &SelectionState,
    blocks: &ReadSignal<Vec<BlockDto>>,
    set_blocks: &WriteSignal<Vec<BlockDto>>,
) {
    let list = blocks.get_untracked();
    let current = sel.selected_block_id.get_untracked();
    let id = match current {
        Some(ref id) => id.clone(),
        None => return,
    };

    // Check if this block has children
    let has_children = list.iter().any(|b| b.parent_id.as_deref() == Some(&id));

    if has_children {
        // Collapse the block
        set_blocks.update(|b| {
            if let Some(block) = b.iter_mut().find(|blk| blk.id == id) {
                block.collapsed = true;
            }
        });
    } else {
        // Go to parent
        if let Some(block) = list.iter().find(|b| b.id == id) {
            if let Some(ref parent_id) = block.parent_id {
                sel.select(parent_id);
            }
        }
    }
}

/// Expand the selected block (show children), or if already expanded
/// and has children, select the first child.
fn expand_or_child(sel: &SelectionState, blocks: &ReadSignal<Vec<BlockDto>>) {
    let list = blocks.get_untracked();
    let current = sel.selected_block_id.get_untracked();
    let id = match current {
        Some(ref id) => id.clone(),
        None => return,
    };

    let block = match list.iter().find(|b| b.id == id) {
        Some(b) => b,
        None => return,
    };

    let children: Vec<&BlockDto> = list
        .iter()
        .filter(|b| b.parent_id.as_deref() == Some(&id))
        .collect();

    if children.is_empty() {
        return;
    }

    if block.collapsed {
        sel.request_collapse(&id, false);
    } else {
        sel.select(&children[0].id);
    }
}

/// Cycle the marker of the selected block: None → todo → doing → done → None.
fn cycle_block_marker(
    sel: &SelectionState,
    blocks: &ReadSignal<Vec<BlockDto>>,
    set_blocks: &WriteSignal<Vec<BlockDto>>,
) {
    let current = sel.selected_block_id.get_untracked();
    let id = match current {
        Some(ref id) => id.clone(),
        None => return,
    };

    let list = blocks.get_untracked();
    let block = match list.iter().find(|b| b.id == id) {
        Some(b) => b,
        None => return,
    };
    let next = next_marker(block.marker.as_deref());

    set_blocks.update(|b| {
        if let Some(blk) = b.iter_mut().find(|blk| blk.id == id) {
            blk.marker = next;
        }
    });
}

/// Return the next marker in the cycle:
/// None → "todo" → "doing" → "done" → None
fn next_marker(current: Option<&str>) -> Option<String> {
    match current {
        None | Some("cancelled") => Some("todo".to_string()),
        Some("todo") => Some("doing".to_string()),
        Some("doing") | Some("now") => Some("done".to_string()),
        Some("done") => None,
        _ => Some("todo".to_string()),
    }
}

/// Collapse the selected block's children (Mod+ArrowUp).
fn collapse_selected_block(sel: &SelectionState, _blocks: &ReadSignal<Vec<BlockDto>>) {
    let current = sel.selected_block_id.get_untracked();
    let id = match current {
        Some(ref id) => id.clone(),
        None => return,
    };
    sel.request_collapse(&id, true);
}

/// Expand the selected block's children (Mod+ArrowDown).
fn expand_selected_block(sel: &SelectionState, _blocks: &ReadSignal<Vec<BlockDto>>) {
    let current = sel.selected_block_id.get_untracked();
    let id = match current {
        Some(ref id) => id.clone(),
        None => return,
    };
    sel.request_collapse(&id, false);
}

/// Toggle collapse on the selected block (Mod+;).
fn toggle_collapse_selected_block(sel: &SelectionState, blocks: &ReadSignal<Vec<BlockDto>>) {
    let current = sel.selected_block_id.get_untracked();
    let (id, currently_collapsed) = match current {
        Some(ref id) => {
            let list = blocks.get_untracked();
            let collapsed = list
                .iter()
                .find(|b| b.id == *id)
                .map(|b| b.collapsed)
                .unwrap_or(false);
            (id.clone(), collapsed)
        }
        None => return,
    };
    sel.request_collapse(&id, !currently_collapsed);
}
