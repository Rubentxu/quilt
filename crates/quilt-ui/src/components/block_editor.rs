//! Block editor component with autocomplete support.
//!
//! Provides a contenteditable-based editor for individual blocks.
//! Autocomplete is integrated via trigger detection on input events
//! and keyboard interception when the dropdown is active.

use crate::bridge::BlockDto;
use crate::components::autocomplete_dropdown::{AutocompleteDropdown, DropdownController};
use crate::outliner::page::PageOutliner;
use crate::parser::autocomplete::AutocompleteTrigger;
use crate::parser::autocomplete_pipeline::autocomplete_at_cursor_with_service;
use crate::parser::autocomplete_pipeline::compute_insertion;
use crate::parser::providers::create_default_service;
use leptos::prelude::*;
use std::sync::Arc;

/// Convert an `AutocompleteTrigger` to a human-readable kind string
/// for history recording (e.g., `"page"`, `"tag"`, `"property"`, `"block"`).
fn ac_trigger_to_kind(trigger: &AutocompleteTrigger) -> Option<String> {
    match trigger {
        AutocompleteTrigger::PageRef { .. } => Some("page".into()),
        AutocompleteTrigger::Tag { .. } => Some("tag".into()),
        AutocompleteTrigger::PropertyValue { .. } => Some("property".into()),
        AutocompleteTrigger::BlockRef { .. } => Some("block".into()),
    }
}

pub struct TreeOps {
    pub on_indent: Arc<dyn Fn() + Send + Sync>,
    pub on_outdent: Arc<dyn Fn() + Send + Sync>,
    pub on_split: Arc<dyn Fn(u32) + Send + Sync>,
    pub on_merge_next: Arc<dyn Fn() + Send + Sync>,
}

impl Default for TreeOps {
    fn default() -> Self {
        Self {
            on_indent: Arc::new(|| {}),
            on_outdent: Arc::new(|| {}),
            on_split: Arc::new(|_| {}),
            on_merge_next: Arc::new(|| {}),
        }
    }
}

impl Clone for TreeOps {
    fn clone(&self) -> Self {
        Self {
            on_indent: self.on_indent.clone(),
            on_outdent: self.on_outdent.clone(),
            on_split: self.on_split.clone(),
            on_merge_next: self.on_merge_next.clone(),
        }
    }
}

/// Cloneable wrapper for the autocomplete service.
#[derive(Clone)]
struct AcService(Arc<crate::parser::autocomplete::AutocompleteService>);

#[component]
pub fn BlockEditor(
    #[prop(into)] block: Signal<BlockDto>,
    /// Called with (text_content, trigger) when content is saved.
    /// `trigger`: `None` for manual edits, `Some("page"|"tag"|"property")` for
    /// autocomplete insertions.
    on_save: impl Fn(String, Option<String>) + Clone + Send + Sync + 'static,
    on_cancel: impl Fn() + Clone + Send + Sync + 'static,
    #[prop(optional)] tree_ops: Option<TreeOps>,
    /// List of page names for autocomplete (from bridge).
    /// Pass empty vec if page data is not yet loaded.
    #[prop(optional)]
    page_names: Vec<String>,
) -> impl IntoView {
    let content = RwSignal::new(block.get().content.clone());
    let el_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let is_focused = RwSignal::new(false);
    let on_save_blur = on_save.clone();
    let tree_ops = tree_ops.unwrap_or_default();

    // Page-level outliner coordinator (optional — degrades gracefully
    // if no context is provided, e.g., in tests or split-pane views).
    let page_outliner: Option<PageOutliner> = use_context();

    // Autocomplete state
    let ac_service = AcService(Arc::new(create_default_service(page_names)));
    let ac_svc = ac_service.0.clone(); // cloned Arc for closures
    let ac_svc_for_kd = ac_svc.clone(); // for handle_keydown
    let ac_svc_for_sel = ac_svc.clone(); // for handle_ac_select
    let ac_items = RwSignal::new(Vec::new());
    let ac_visible = RwSignal::new(false);
    let ac_selected = RwSignal::new(0usize);
    let ac_trigger_kind: RwSignal<Option<String>> = RwSignal::new(None);
    let ac_controller = RwSignal::new(None);

    Effect::new(move || {
        if let Some(el) = el_ref.get() {
            let _ = el.focus();
        }
    });

    // ── Editor resync: When the block content changes externally (e.g.,
    //    undo/redo via PageOutliner), update the local editing buffer
    //    and the DOM element to keep them consistent.
    //    We don't touch the local signal while the user is actively
    //    focused/typing to avoid clobbering in-progress edits.
    Effect::new(move || {
        let saved = block.get().content;
        if !is_focused.get_untracked() {
            content.set(saved.clone());
            if let Some(el) = el_ref.get_untracked() {
                let current = el.text_content().unwrap_or_default();
                if current != saved {
                    el.set_text_content(Some(&saved));
                }
            }
        }
    });

    let handle_input = {
        let ac_svc = ac_svc.clone();
        move |_ev: leptos::ev::Event| {
            if let Some(el) = el_ref.get() {
                let text = el.text_content().unwrap_or_default();
                content.set(text.clone());

                let cursor = crate::components::keyboard_handlers::get_cursor_offset(&el);
                let cursor_us = cursor as usize;

                let (trigger, result) =
                    autocomplete_at_cursor_with_service(&text, cursor_us, &ac_svc);

                if let Some(t) = trigger {
                    if !result.is_empty() {
                        let count = result.items.len();
                        ac_trigger_kind.set(ac_trigger_to_kind(&t));
                        ac_items.set(result.items);
                        ac_selected.set(0);
                        ac_controller.set(Some(DropdownController::new(count)));
                        ac_visible.set(true);
                        return;
                    }
                }

                ac_visible.set(false);
                ac_items.set(Vec::new());
                ac_trigger_kind.set(None);
                ac_controller.set(None);
            }
        }
    };

    let on_save_kd = on_save.clone();
    let on_save_ac = on_save.clone();

    let handle_keydown = {
        move |ev: leptos::ev::KeyboardEvent| {
            let key = ev.key();
            let ctrl = ev.ctrl_key() || ev.meta_key();
            let shift = ev.shift_key();

            if ac_visible.get_untracked() {
                match (key.as_str(), shift, ctrl) {
                    ("ArrowDown", false, false) => {
                        ev.prevent_default();
                        ac_controller.update(|c| {
                            if let Some(ref mut ctrl) = c {
                                ctrl.move_down();
                                ac_selected.set(ctrl.selected());
                            }
                        });
                        return;
                    }
                    ("ArrowUp", false, false) => {
                        ev.prevent_default();
                        ac_controller.update(|c| {
                            if let Some(ref mut ctrl) = c {
                                ctrl.move_up();
                                ac_selected.set(ctrl.selected());
                            }
                        });
                        return;
                    }
                    ("Enter", false, false) => {
                        ev.prevent_default();
                        let current = content.get_untracked();
                        let trigger_kind = ac_trigger_kind.get_untracked();
                        let entry = {
                            // Re-detect the trigger for computing insertion
                            let cursor = crate::components::keyboard_handlers::get_cursor_offset(
                                &el_ref.get().unwrap(),
                            ) as usize;
                            let (trigger, _) = autocomplete_at_cursor_with_service(&current, cursor, &ac_svc_for_kd);
                            let items = ac_items.get_untracked();
                            let idx = ac_selected.get_untracked();
                            trigger.and_then(|t| {
                                items
                                    .get(idx)
                                    .and_then(|item| compute_insertion(&current, &t, item))
                            })
                        };
                        if let Some(ir) = entry {
                            let new_text = ir.new_content;
                            content.set(new_text.clone());
                            ac_visible.set(false);
                            ac_items.set(Vec::new());
                            ac_controller.set(None);
                            (on_save_kd.clone())(new_text, trigger_kind);
                        }
                        return;
                    }
                    ("Escape", _, _) => {
                        ev.prevent_default();
                        ac_visible.set(false);
                        ac_items.set(Vec::new());
                        ac_trigger_kind.set(None);
                        ac_controller.set(None);
                        return;
                    }
                    // Silently consume undo/redo shortcuts when dropdown is active
                    // to prevent them from interfering with autocomplete navigation.
                    ("z", false, true) | ("z", true, true) | ("y", false, true) => {
                        ev.prevent_default();
                        return;
                    }
                    _ => {}
                }
            }

            match (key.as_str(), shift, ctrl) {
                ("Enter", false, false) => {
                    ev.prevent_default();
                    let offset = crate::components::keyboard_handlers::get_cursor_offset(
                        &el_ref.get().unwrap(),
                    );
                    (tree_ops.on_split)(offset);
                    (on_save_kd.clone())(content.get_untracked(), None);
                }
                ("Enter", true, false) => {}
                ("Escape", _, _) => {
                    ev.prevent_default();
                    content.set(block.get().content.clone());
                    on_cancel.clone()();
                }
                ("Tab", false, false) => {
                    ev.prevent_default();
                    (tree_ops.on_indent)();
                }
                ("Tab", true, false) => {
                    ev.prevent_default();
                    (tree_ops.on_outdent)();
                }
                ("Backspace", false, true) => {
                    ev.prevent_default();
                    (tree_ops.on_merge_next)();
                }
                ("Backspace", false, false) => {}
                // Undo/Redo: Mod+Z = Undo, Mod+Shift+Z / Mod+Y = Redo
                ("z", false, true) => {
                    ev.prevent_default();
                    if let Some(ref p) = page_outliner {
                        p.undo();
                    }
                }
                ("z", true, true) => {
                    ev.prevent_default();
                    if let Some(ref p) = page_outliner {
                        p.redo();
                    }
                }
                ("y", false, true) => {
                    ev.prevent_default();
                    if let Some(ref p) = page_outliner {
                        p.redo();
                    }
                }
                _ => {}
            }
        }
    };

    // Create the autocomplete select callback.
    // This is called when the user clicks an item in the dropdown.
    let handle_ac_select = move |idx: usize| {
        let current = content.get_untracked();
        let trigger_kind = ac_trigger_kind.get_untracked();
        let entry = {
            let cursor = content.get_untracked().len();
            let (trigger, _) = autocomplete_at_cursor_with_service(&current, cursor, &ac_svc_for_sel);
            let items = ac_items.get_untracked();
            trigger.and_then(|t| {
                items
                    .get(idx)
                    .and_then(|item| compute_insertion(&current, &t, item))
            })
        };
        if let Some(ir) = entry {
            let new_text = ir.new_content;
            content.set(new_text.clone());
            ac_visible.set(false);
            ac_items.set(Vec::new());
            ac_trigger_kind.set(None);
            ac_controller.set(None);
            (on_save_ac.clone())(new_text, trigger_kind);
        }
    };

    view! {
        <div class="block-editor-wrapper relative">
            <div
                node_ref=el_ref
                class="flex-1 text-sm min-h-[1.5em] outline-none break-words border-l-2 border-accent pl-1"
                contenteditable="true"
                on:keydown=handle_keydown
                on:focus=move |_| is_focused.set(true)
                on:blur=move |_| {
                    is_focused.set(false);
                    on_save_blur.clone()(content.get_untracked(), None);
                }
                on:input=handle_input
            >
                {move || block.get().content.clone()}
            </div>

            <Show when=move || ac_visible.get()>
                {{
                    let items = ac_items.get();
                    let sel_idx = ac_selected.get();
                    let cb = handle_ac_select.clone();
                    if !items.is_empty() {
                        (view! {
                            <AutocompleteDropdown
                                items=items
                                selected_index=sel_idx
                                on_select=cb
                                visible=true
                            />
                        }).into_any()
                    } else {
                        (view! { <div></div> }).into_any()
                    }
                }}
            </Show>
        </div>
    }
}
