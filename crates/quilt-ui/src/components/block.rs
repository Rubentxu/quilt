use crate::bridge::BlockDto;
use crate::components::block_editor::TreeOps;
use crate::components::cm6_block_editor::Cm6BlockEditor;
use crate::editor::decorations::{
    replace_property_value, DecorationManager, RenderItem,
};
use crate::outliner::history::OutlinerCommand;
use crate::outliner::page::PageOutliner;
use crate::outliner::selection::SelectionState;
use crate::outliner::tree::{indent, merge_with_next, outdent, split_block};
use crate::parser::InlineParser;
use leptos::prelude::*;
use log::warn;
use std::sync::Arc;

#[component]
pub fn Block(
    #[prop(into)] block: Signal<BlockDto>,
    #[prop(into)] blocks: Signal<Vec<BlockDto>>,
    #[prop(into)] set_blocks: WriteSignal<Vec<BlockDto>>,
    children: Vec<BlockDto>,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let (collapsed, set_collapsed) = signal(block.get().collapsed);
    let has_children = !children.is_empty();
    let children = RwSignal::new(children);

    // Retrieve the PageOutliner coordinator (optional — gracefully degrades
    // if no context is provided, e.g., in tests or future split views).
    let page_outliner: Option<PageOutliner> = use_context();

    // Retrieve page names for autocomplete (populated by PageView).
    // Falls back to empty vec if context is not provided.
    let page_names_signal =
        use_context::<RwSignal<Vec<String>>>().unwrap_or_else(|| RwSignal::new(Vec::new()));

    // Retrieve SelectionState for keyboard-first navigation (optional).
    let selection_state: Option<SelectionState> = use_context();

    // Determine if THIS block is currently selected.
    let is_selected = Signal::derive({
        let block_id = block.get().id.clone();
        move || {
            selection_state
                .as_ref()
                .and_then(|s| s.selected_block_id.get())
                .is_some_and(|id| id == block_id)
        }
    });

    // Watch for edit_request: if this block was requested to start editing,
    // set local editing to true and consume the request.
    let block_id_for_effect = block.get().id.clone();
    Effect::new({
        let block_id = block_id_for_effect.clone();
        let edit_req = selection_state;
        move || {
            if let Some(ref sel) = edit_req {
                if sel.edit_request.get().as_deref() == Some(&block_id) {
                    set_editing.set(true);
                    sel.edit_request.set(None);
                }
            }
        }
    });

    // When editing state changes locally, sync to SelectionState.
    Effect::new({
        let block_id = block_id_for_effect.clone();
        let sel = selection_state;
        move || {
            if editing.get() {
                if let Some(ref s) = sel {
                    s.set_editing(&block_id);
                }
            } else {
                if let Some(ref s) = sel {
                    if s.editing_block_id.get().as_deref() == Some(&block_id) {
                        s.clear_editing();
                    }
                }
            }
        }
    });

    // Watch for collapse_request from page-level keyboard handler.
    Effect::new({
        let block_id = block_id_for_effect.clone();
        let sel = selection_state;
        let sc = set_collapsed;
        move || {
            if let Some(ref s) = sel {
                if let Some((id, val)) = s.collapse_request.get() {
                    if id == block_id {
                        sc.set(val);
                        s.collapse_request.set(None);
                    }
                }
            }
        }
    });

    // Track the content at the START of the current edit session for undo/redo.
    // Updated when the user clicks to edit; not captured at component creation time.
    // This ensures each edit session records the correct "before" state even if
    // block content changed via undo/redo or structural operations between sessions.
    let before_content_for_undo = RwSignal::new(block.get().content.clone());

    let on_save = {
        let outliner = page_outliner.clone();
        let sel = selection_state;
        let block_id = block.get().id.clone();

        move |content: String, trigger: Option<String>| {
            let before = before_content_for_undo.get_untracked();
            if let Some(ref o) = outliner {
                o.record_content_change(&block_id, &before, &content, trigger.as_deref());
            }
            before_content_for_undo.set(content);
            set_editing.set(false);
            // After saving, keep the block selected (not editing).
            if let Some(ref s) = sel {
                s.select(&block_id);
                s.clear_editing();
            }
        }
    };

    let on_cancel = {
        let sel = selection_state;
        let block_id = block.get().id.clone();
        move || {
            set_editing.set(false);
            // Keep the block selected after cancelling.
            if let Some(ref s) = sel {
                s.select(&block_id);
                s.clear_editing();
            }
        }
    };

    let on_indent = {
        let outliner = page_outliner.clone();
        move || {
            let block_id = block.get().id.clone();
            let before = block.get();
            let old_parent = before.parent_id.clone();
            let old_order = before.order;

            set_blocks.update(|blocks_mut| {
                if let Err(e) = indent(blocks_mut, &block_id) {
                    warn!("Indent failed: {}", e);
                    return;
                }
                if let Some(after) = blocks_mut.iter().find(|b| b.id == block_id) {
                    if let Some(ref o) = outliner {
                        let cmd = OutlinerCommand::Indent {
                            block_id: block_id.clone(),
                            old_parent,
                            old_order,
                            new_parent: after.parent_id.clone(),
                            new_order: after.order,
                        };
                        o.record_structural(cmd);
                    }
                }
            });
        }
    };

    let on_outdent = {
        let outliner = page_outliner.clone();
        move || {
            let block_id = block.get().id.clone();
            let before = block.get();
            let old_parent = before.parent_id.clone();
            let old_order = before.order;

            set_blocks.update(|blocks_mut| {
                if let Err(e) = outdent(blocks_mut, &block_id) {
                    warn!("Outdent failed: {}", e);
                    return;
                }
                if let Some(after) = blocks_mut.iter().find(|b| b.id == block_id) {
                    if let Some(ref o) = outliner {
                        let cmd = OutlinerCommand::Outdent {
                            block_id: block_id.clone(),
                            old_parent,
                            old_order,
                            new_parent: after.parent_id.clone(),
                            new_order: after.order,
                        };
                        o.record_structural(cmd);
                    }
                }
            });
        }
    };

    let on_split = {
        let outliner = page_outliner.clone();
        move |cursor: u32| {
            let block_id = block.get().id.clone();
            set_blocks.update(|blocks_mut| {
                if let Ok((old_block, new_block)) = split_block(blocks_mut, &block_id, cursor) {
                    if let Some(ref o) = outliner {
                        let (first, second) =
                            (old_block.content.clone(), new_block.content.clone());
                        let cmd = crate::outliner::history::OutlinerCommand::SplitBlock {
                            block_id: block_id.clone(),
                            new_block_id: new_block.id.clone(),
                            first_part: first,
                            second_part: second,
                        };
                        o.record_structural(cmd);
                    }
                } else {
                    warn!("Split failed");
                }
            });
        }
    };

    let on_merge_next = {
        move || {
            let block_id = block.get().id.clone();
            set_blocks.update(|blocks_mut| {
                if let Err(e) = merge_with_next(blocks_mut, &block_id) {
                    warn!("Merge with next failed: {}", e);
                }
            });
        }
    };

    let tree_ops = TreeOps {
        on_indent: Arc::new(on_indent),
        on_outdent: Arc::new(on_outdent),
        on_split: Arc::new(on_split),
        on_merge_next: Arc::new(on_merge_next),
    };

    view! {
        <div class="block-group">
            <div class="flex items-start gap-1 py-0.5 group hover:bg-surface-hover rounded-sm transition-colors"
                 class:block-selected=is_selected
                 data-block-id={move || block.get().id.clone()}
                 style=move || format!(
                     "padding-left: {}px",
                     (block.get().level.saturating_sub(1)) * 24
                 )
            >
                <button
                    class="w-5 h-5 flex items-center justify-center text-text-muted hover:text-text shrink-0 mt-0.5 block-bullet"
                    on:click=move |_| {
                        if let Some(ref s) = selection_state {
                            s.select(&block.get().id);
                        }
                        if has_children {
                            set_collapsed.update(|c| *c = !*c);
                        }
                    }
                >
                    {move || if has_children {
                        if collapsed.get() { "▶" } else { "▼" }
                    } else {
                        "•"
                    }}
                </button>

                {move || if editing.get() {
                    let os = on_save.clone();
                    let oc = on_cancel.clone();
                    let page_names = page_names_signal.get();

                    // Callback to update block marker from slash commands
                    let on_set_marker_cb = {
                        let bid = block.get().id.clone();
                        let sb = set_blocks;
                        Arc::new(move |marker: String| {
                            sb.update(|blocks| {
                                if let Some(b) = blocks.iter_mut().find(|b| b.id == bid) {
                                    b.marker = Some(marker.clone());
                                }
                            });
                        }) as Arc<dyn Fn(String) + Send + Sync>
                    };

                    // Callback to update block priority from slash commands
                    let on_set_priority_cb = {
                        let bid = block.get().id.clone();
                        let sb = set_blocks;
                        Arc::new(move |priority: String| {
                            sb.update(|blocks| {
                                if let Some(b) = blocks.iter_mut().find(|b| b.id == bid) {
                                    b.priority = Some(priority.clone());
                                }
                            });
                        }) as Arc<dyn Fn(String) + Send + Sync>
                    };

                    vec![view! {
                        <Cm6BlockEditor
                            block=block
                            on_save=os
                            on_cancel=oc
                            tree_ops=tree_ops.clone()
                            page_names=page_names
                            on_set_marker=on_set_marker_cb
                            on_set_priority=on_set_priority_cb
                        />
                    }.into_any()]
                } else {
                    let b = block.get();
                    let content = b.content.clone();
                    let marker = b.marker.clone().unwrap_or_default();
                    let icon = match marker.as_str() {
                        "todo" => "○",
                        "now" | "doing" => "●",
                        "done" => "✓",
                        "cancelled" => "✕",
                        _ => "",
                    };

                    // Build property-aware render items from parsed content
                    let parser = InlineParser::default();
                    let parsed = parser.parse(&content);
                    let render_items =
                        DecorationManager::build_render_items(&content, &parsed);
                    let block_id = b.id.clone();
                    let block_content_for_undo = before_content_for_undo;

                    // Helper closures for property interactions
                    let on_property_status_click = {
                        let bid = block_id.clone();
                        let content_c = content.clone();
                        let ri = render_items.clone();
                        let sb = set_blocks;
                        move |idx: usize| {
                            if idx < ri.len() {
                                if let RenderItem::PropertyStatus {
                                    ref key,
                                    ref value,
                                    ref range,
                                    ..
                                } = ri[idx]
                                {
                                    let new_val = DecorationManager::cycle_status(value);
                                    let new_content =
                                        replace_property_value(&content_c, range, key, new_val);
                                    sb.update(|blocks| {
                                        if let Some(b) = blocks.iter_mut().find(|b| b.id == bid) {
                                            b.content = new_content;
                                        }
                                    });
                                }
                            }
                        }
                    };

                    let on_property_priority_click = {
                        let bid = block_id;
                        let content_c = content;
                        let ri = render_items.clone();
                        let sb = set_blocks;
                        move |idx: usize| {
                            if idx < ri.len() {
                                if let RenderItem::PropertyPriority {
                                    ref key,
                                    ref value,
                                    ref range,
                                    ..
                                } = ri[idx]
                                {
                                    let new_val = DecorationManager::cycle_priority(value);
                                    let new_content =
                                        replace_property_value(&content_c, range, key, new_val);
                                    sb.update(|blocks| {
                                        if let Some(b) = blocks.iter_mut().find(|b| b.id == bid) {
                                            b.content = new_content;
                                        }
                                    });
                                }
                            }
                        }
                    };

                    let icon_str: String = icon.to_string();
                    let on_start_edit_cb = {
                        let sel = selection_state.clone();
                        let bid = block.get().id.clone();
                        let bcu = block_content_for_undo;
                        let se = set_editing;
                        let block_signal = block;
                        move || {
                            if let Some(ref s) = sel {
                                s.select(&bid);
                            }
                            bcu.set(block_signal.get().content.clone());
                            se.set(true);
                        }
                    };

                    vec![view! {
                        <PropertyContentView
                            icon=icon_str
                            render_items=render_items
                            on_status_click=std::sync::Arc::new(on_property_status_click)
                            on_priority_click=std::sync::Arc::new(on_property_priority_click)
                            on_start_edit=Callback::from(on_start_edit_cb)
                        />
                    }.into_any()]
                }}
            </div>

            {move || if !collapsed.get() && has_children {
                view! {
                    <div class="block-children">
                        <For each=move || children.get() key=|b| b.id.clone() let:child>
                            <Block block=Signal::derive(move || child.clone()) blocks=blocks set_blocks=set_blocks children=vec![] />
                        </For>
                    </div>
                }.into_any()
            } else if collapsed.get() && has_children {
                view! {
                    <div class="text-xs text-text-muted pl-8 py-0.5">"hidden blocks"</div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}

/// Non-editing view of a block with property-aware rich rendering.
///
/// Extracted as a separate component to avoid deep brace nesting in the
/// main `view!` template, which can confuse the Leptos proc-macro.
#[component]
fn PropertyContentView(
    /// Icon character (○ ● ✓ ✕) for task marker
    icon: String,
    /// Property render items
    render_items: Vec<RenderItem>,
    /// Click handler for status properties (cycles TODO/DOING/DONE)
    on_status_click: std::sync::Arc<dyn Fn(usize) + Send + Sync + 'static>,
    /// Click handler for priority properties (cycles A/B/C)
    on_priority_click: std::sync::Arc<dyn Fn(usize) + Send + Sync + 'static>,
    /// Called when the block is clicked to start editing
    on_start_edit: leptos::prelude::Callback<(), ()>,
) -> impl IntoView {
    view! {
        <div
            class="flex-1 text-sm cursor-text min-h-[1.5em] break-words"
            on:click=move |_| { on_start_edit.run(()); }
        >
            {if icon.is_empty() { String::new() } else { format!("{} ", icon) }}
            {render_property_items(
                render_items,
                on_status_click,
                on_priority_click,
            )}
        </div>
    }
}

/// Render a list of `RenderItem`s into HTML view children with property-aware styling.
///
/// Plain text, tags, and refs get their standard decoration CSS classes.
/// Properties get rich visual treatment: status badges, priority pills,
/// date icons, tag pills, and generic property key/value styling.
fn render_property_items(
    items: Vec<RenderItem>,
    on_status_click: std::sync::Arc<dyn Fn(usize) + Send + Sync + 'static>,
    on_priority_click: std::sync::Arc<dyn Fn(usize) + Send + Sync + 'static>,
) -> Vec<leptos::prelude::AnyView> {
    items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| match item {
            RenderItem::DecoratedText {
                text,
                css_class,
                label: _,
            } => {
                if css_class.is_empty() {
                    view! { <span>{text}</span> }.into_any()
                } else {
                    view! { <span class={css_class}>{text}</span> }.into_any()
                }
            }
            RenderItem::PropertyStatus {
                full_text: _,
                key,
                value,
                valid: _,
                range: _,
            } => {
                let status_class = match value.to_uppercase().as_str() {
                    "TODO" => "prop-status-todo",
                    "DOING" => "prop-status-doing",
                    "DONE" => "prop-status-done",
                    _ => "prop-status-invalid",
                };
                let circle = match value.to_uppercase().as_str() {
                    "TODO" => "○",
                    "DOING" => "●",
                    "DONE" => "✓",
                    _ => "?",
                };
                let cb = on_status_click.clone();
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span
                        class={format!("prop-status {}", status_class)}
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb(idx);
                        }
                    >
                        <span class="prop-status-circle">{circle}</span>
                        <span class="prop-status-text">{value.clone()}</span>
                    </span>
                }.into_any()
            }
            RenderItem::PropertyPriority {
                full_text: _,
                key,
                value,
                valid: _,
                range: _,
            } => {
                let pill_class = match value.to_uppercase().as_str() {
                    "A" => "prop-priority-A",
                    "B" => "prop-priority-B",
                    "C" => "prop-priority-C",
                    _ => "prop-priority-invalid",
                };
                let cb = on_priority_click.clone();
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span
                        class={format!("prop-priority {}", pill_class)}
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb(idx);
                        }
                    >
                        <span class="prop-priority-letter">{value.clone()}</span>
                    </span>
                }.into_any()
            }
            RenderItem::PropertyScheduled {
                full_text: _,
                key,
                value,
                range: _,
            } => {
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span class="prop-date prop-date-scheduled" title={format!("Scheduled: {}", value)}>
                        <span class="prop-date-icon">"📅"</span>
                        <span class="prop-date-text">{value.clone()}</span>
                    </span>
                }.into_any()
            }
            RenderItem::PropertyDeadline {
                full_text: _,
                key,
                value,
                range: _,
            } => {
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span class="prop-date prop-date-deadline" title={format!("Deadline: {}", value)}>
                        <span class="prop-date-icon">"⏰"</span>
                        <span class="prop-date-text">{value.clone()}</span>
                    </span>
                }.into_any()
            }
            RenderItem::PropertyTags {
                full_text: _,
                key,
                value: _,
                tags,
                range: _,
            } => {
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span class="prop-tags">
                        {tags.into_iter().map(|tag| {
                            let colors = ["prop-tag-blue", "prop-tag-green", "prop-tag-orange", "prop-tag-purple", "prop-tag-teal"];
                            let color_idx = tag.len() % colors.len();
                            view! {
                                <span class={format!("prop-tag-pill {}", colors[color_idx])}>
                                    {tag}
                                </span>
                            }
                        }).collect::<Vec<_>>()}
                    </span>
                }.into_any()
            }
            RenderItem::PropertyGeneric { key, value, .. } => {
                view! {
                    <span class="prop-key">{key}:: </span>
                    <span class="prop-value">{value.clone()}</span>
                }.into_any()
            }
        })
        .collect()
}
