//! CodeMirror 6 backed block editor component with autocomplete and decorations.
//!
//! Replaces the contenteditable-based `BlockEditor` with a CM6 editor.
//! Key differences:
//! - Undo/redo is owned by the Outliner/HistoryStack, NOT by CM6.
//! - Custom keybindings dispatch tree operations (Enter, Tab, etc.)
//!   to the Rust `TreeOps` and `PageOutliner`.
//! - Single-line editing only (CM6 configured without history).
//! - Autocomplete reuses the existing Quilt pipeline (detect_trigger +
//!   AutocompleteService + compute_insertion).
//! - Visual decorations (tags, page refs, properties) are computed via
//!   InlineParser + DecorationManager and pushed to CM6 as mark decorations.
//!
//! # Lifecycle
//!
//! 1. Component mounts -> creates a `<div>` with `NodeRef`.
//! 2. `Effect::new` runs when the div is available -> calls
//!    `Cm6Handle::create()` via `wasm-bindgen` JS interop.
//! 3. CM6 instance stays alive until the component unmounts.
//! 4. On unmount, `Cm6Handle::drop()` cleans up the JS instance.
//!
//! # Content sync
//!
//! - Typing -> CM6 fires `onChange` -> updates local `content` signal.
//! - External changes (undo/redo via Outliner) -> `Effect` detects
//!   new `block()` content and calls `Cm6Handle::set_content()`.
//! - The `on_save` callback fires on blur and Enter (split).
//!
//! # Autocomplete
//!
//! - On each content change (onChange), runs detect_trigger at cursor
//!   position via the Rust autocomplete pipeline.
//! - If a trigger is found with suggestions, shows an AutocompleteDropdown.
//! - Keyboard navigation (ArrowUp/Down, Enter, Escape) when dropdown is
//!   active is intercepted by CM6's keymap via `dropdownActive` flag.
//! - On selection, compute_insertion computes the new content and cursor,
//!   then updates the editor programmatically.
//!
//! # Decorations
//!
//! - On each content change, runs InlineParser + DecorationManager to
//!   compute visual decorations (tags, page refs, properties).
//! - Decorations are serialized to JSON and pushed to CM6 via the bridge
//!   `setDecorations` method, which applies them as CM6 mark decorations.
//!
//! # Thread safety note
//!
//! Leptos 0.8 view closures require `Send + Sync`. The CM6 handle lives
//! inside `Rc<RefCell<...>>` (single-threaded, for Effect closures only).
//! To bridge the gap, view-level closures use `RwSignal`s to communicate
//! with CM6 via dedicated Effects. This keeps the handle out of view
//! closures while maintaining correct behavior.

use crate::bridge::BlockDto;
use crate::components::autocomplete_dropdown::{AutocompleteDropdown, DropdownController};
use crate::editor::cm6_bridge::{self, Cm6Callbacks, Cm6Handle};
use crate::editor::decorations::DecorationManager;
use crate::outliner::page::PageOutliner;
use crate::parser::autocomplete::AutocompleteTrigger;
use crate::parser::autocomplete_pipeline::{
    autocomplete_at_cursor, autocomplete_at_cursor_with_service, compute_insertion,
};
use crate::parser::inline::InlineParser;
use crate::parser::providers::create_default_service;
use leptos::prelude::*;
use serde::Serialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// ── Helpers ──

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

/// Serializable decoration DTO for CM6 bridge.
#[derive(Serialize)]
struct Cm6DecorationDto {
    from: usize,
    to: usize,
    class: String,
}

/// Compute visual decorations from block content, serialized as JSON
/// suitable for CM6's setDecorations bridge method.
fn compute_decorations_json(content: &str) -> String {
    if content.is_empty() {
        return "[]".to_string();
    }
    let parser = InlineParser::default();
    let parsed = parser.parse(content);
    let decorations = DecorationManager::build_decorations(&parsed);

    let cm6_decos: Vec<Cm6DecorationDto> = decorations
        .iter()
        .map(|d| {
            let class = match &d.kind {
                crate::editor::decorations::DecorationKind::PageLink { .. } => {
                    "decoration-page-ref"
                }
                crate::editor::decorations::DecorationKind::BlockLink { .. } => {
                    "decoration-block-ref"
                }
                crate::editor::decorations::DecorationKind::Tag { .. } => "decoration-tag",
                crate::editor::decorations::DecorationKind::Property { .. } => {
                    "decoration-property"
                }
                crate::editor::decorations::DecorationKind::SearchMatch { .. } => {
                    "decoration-search"
                }
                crate::editor::decorations::DecorationKind::AutocompleteActive { .. } => {
                    "decoration-autocomplete"
                }
            };
            Cm6DecorationDto {
                from: d.range.start,
                to: d.range.end,
                class: class.to_string(),
            }
        })
        .collect();

    serde_json::to_string(&cm6_decos).unwrap_or_else(|_| "[]".to_string())
}

/// Cloneable wrapper for the autocomplete service.
#[derive(Clone)]
struct AcService(Arc<crate::parser::autocomplete::AutocompleteService>);

/// Try to create a CM6 editor, with one retry attempt for JS bundle race.
fn try_create_cm6(
    container: &web_sys::Element,
    content: &str,
    callbacks: &Cm6Callbacks,
) -> Result<Cm6Handle, String> {
    if !cm6_bridge::is_cm6_available() {
        // Brief yield to let the JS event loop process
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Cm6Handle::create(container, content, callbacks).map_err(|e| e.to_string())
}

#[component]
pub fn Cm6BlockEditor(
    #[prop(into)] block: Signal<BlockDto>,
    /// Called with (text_content, trigger) when content is saved.
    /// `trigger`: `None` for manual edits, `Some("page"|"tag"|"property")` for
    /// autocomplete insertions.
    on_save: impl Fn(String, Option<String>) + Clone + Send + Sync + 'static,
    on_cancel: impl Fn() + Clone + Send + Sync + 'static,
    #[prop(optional)] tree_ops: Option<super::block_editor::TreeOps>,
    /// List of page names for page ref autocomplete (from bridge).
    /// Pass empty vec if page data is not yet loaded.
    #[prop(optional)]
    page_names: Vec<String>,
) -> impl IntoView {
    let content = RwSignal::new(block.get().content.clone());
    let el_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let on_save_blur = on_save.clone();
    let tree_ops = tree_ops.unwrap_or_default();
    let cm6_handle: Rc<RefCell<Option<Cm6Handle>>> = Rc::new(RefCell::new(None));

    // Page-level outliner coordinator (optional)
    let page_outliner: Option<PageOutliner> = use_context();

    // ── Autocomplete state (signals for dropdown rendering) ──
    let ac_service = AcService(Arc::new(create_default_service(page_names)));
    let ac_svc = ac_service.0.clone();
    let ac_items: RwSignal<Vec<crate::parser::autocomplete::AutocompleteItem>> = RwSignal::new(Vec::new());
    let ac_visible: RwSignal<bool> = RwSignal::new(false);
    let ac_selected: RwSignal<usize> = RwSignal::new(0usize);
    let ac_trigger_kind: RwSignal<Option<String>> = RwSignal::new(None);
    let ac_controller: RwSignal<Option<DropdownController>> = RwSignal::new(None);

    // Signal bridge for view-layer closures to talk to CM6 (without
    // capturing Rc<RefCell<>>, which isn't Send+Sync).
    // When set, an Effect pushes the content+cursor to the editor.
    let ac_result: RwSignal<Option<(String, u32)>> = RwSignal::new(None);

    // ── Effect: push autocomplete results (content + cursor) to CM6 ──
    Effect::new({
        let cm6_handle = cm6_handle.clone();
        move || {
            if let Some((ref text, cursor)) = ac_result.get() {
                if let Some(ref handle) = *cm6_handle.borrow() {
                    let _ = handle.set_autocomplete_state(false);
                    let _ = handle.set_content_with_cursor(text, cursor);
                    let decos = compute_decorations_json(text);
                    let _ = handle.set_decorations(&decos);
                }
                ac_result.set(None);
            }
        }
    });

    // ── Effect: sync autocomplete visibility to CM6 keymap ──
    Effect::new({
        let cm6_handle = cm6_handle.clone();
        move || {
            let visible = ac_visible.get();
            if let Some(ref handle) = *cm6_handle.borrow() {
                let _ = handle.set_autocomplete_state(visible);
            }
        }
    });

    // ── Lifecycle: mount CM6 when the div is available ──
    Effect::new({
        let el_ref = el_ref.clone();
        let block_id = block.get().id.clone();
        let cm6_handle = cm6_handle.clone();
        let page_outliner = page_outliner.clone();
        let content_signal = content;
        let on_save_effect = on_save.clone();
        let tree_ops_effect = tree_ops.clone();
        let initial_block = block.get().content.clone();

        // Clones for autocomplete closures (move into Effect)
        let ac_svc_effect = ac_svc.clone();
        let ac_items_effect = ac_items;
        let ac_visible_effect = ac_visible;
        let ac_selected_effect = ac_selected;
        let ac_trigger_kind_effect = ac_trigger_kind;
        let ac_controller_effect = ac_controller;
        let ac_result_effect = ac_result;

        move || {
            let el = el_ref.get();
            if el.is_none() {
                return;
            }
            let el = el.unwrap();

            // Don't re-init if we already have a handle
            if cm6_handle.borrow().is_some() {
                return;
            }

            let initial_content = content_signal.get_untracked();
            let bid = block_id.clone();

            // ── Create JS Closures for CM6 callbacks ──
            // These live for the editor's lifetime via .forget()

            // --- on_change: content change + autocomplete + decorations ---
            let on_change_cb = {
                let c = content_signal;
                let cm6_handle_inner = cm6_handle.clone();
                let ac_svc_inner = ac_svc_effect.clone();
                let ac_items_inner = ac_items_effect;
                let ac_visible_inner = ac_visible_effect;
                let ac_selected_inner = ac_selected_effect;
                let ac_trigger_kind_inner = ac_trigger_kind_effect;
                let ac_controller_inner = ac_controller_effect;

                Closure::wrap(Box::new(move |text: String| {
                    c.set(text.clone());

                    // 1. Compute visual decorations from parser
                    let decos_json = compute_decorations_json(&text);
                    if let Some(ref handle) = *cm6_handle_inner.borrow() {
                        let _ = handle.set_decorations(&decos_json);
                    }

                    // 2. Autocomplete: detect trigger + get suggestions
                    let cursor = cm6_handle_inner
                        .borrow()
                        .as_ref()
                        .and_then(|h| h.cursor_offset().ok())
                        .unwrap_or(0) as usize;
                    let (trigger, result) =
                        autocomplete_at_cursor_with_service(&text, cursor, &ac_svc_inner);

                    if let Some(t) = trigger {
                        if !result.is_empty() {
                            let count = result.items.len();
                            ac_trigger_kind_inner.set(ac_trigger_to_kind(&t));
                            ac_items_inner.set(result.items);
                            ac_selected_inner.set(0);
                            ac_controller_inner.set(Some(DropdownController::new(count)));
                            ac_visible_inner.set(true);
                            return;
                        }
                    }

                    // No trigger or no items — hide dropdown
                    ac_visible_inner.set(false);
                    ac_items_inner.set(Vec::new());
                    ac_trigger_kind_inner.set(None);
                    ac_controller_inner.set(None);
                }) as Box<dyn Fn(String)>)
            };

            let on_enter_cb = {
                let tree_ops = tree_ops_effect.clone();
                let on_save_enter = on_save_effect.clone();
                let content_enter = content_signal;
                Closure::wrap(Box::new(move |offset: f64| {
                    let cursor = offset as u32;
                    (tree_ops.on_split)(cursor);
                    on_save_enter(content_enter.get_untracked(), None);
                }) as Box<dyn Fn(f64)>)
            };

            let on_tab_cb = {
                let tree_ops = tree_ops_effect.clone();
                Closure::wrap(Box::new(move || {
                    (tree_ops.on_indent)();
                }) as Box<dyn Fn()>)
            };

            let on_shift_tab_cb = {
                let tree_ops = tree_ops_effect.clone();
                Closure::wrap(Box::new(move || {
                    (tree_ops.on_outdent)();
                }) as Box<dyn Fn()>)
            };

            let on_escape_cb = {
                let on_cancel_esc = on_cancel.clone();
                let content_esc = content_signal;
                let initial_esc = initial_block.clone();
                Closure::wrap(Box::new(move || {
                    content_esc.set(initial_esc.clone());
                    on_cancel_esc();
                }) as Box<dyn Fn()>)
            };

            let on_backspace_cb = {
                let tree_ops = tree_ops_effect.clone();
                Closure::wrap(Box::new(move || {
                    (tree_ops.on_merge_next)();
                }) as Box<dyn Fn()>)
            };

            let on_ctrl_backspace_cb = {
                let tree_ops = tree_ops_effect.clone();
                Closure::wrap(Box::new(move || {
                    (tree_ops.on_merge_next)();
                }) as Box<dyn Fn()>)
            };

            let on_undo_cb = {
                let outliner = page_outliner.clone();
                Closure::wrap(Box::new(move || {
                    if let Some(ref p) = outliner {
                        p.undo();
                    }
                }) as Box<dyn Fn()>)
            };

            let on_redo_cb = {
                let outliner = page_outliner.clone();
                Closure::wrap(Box::new(move || {
                    if let Some(ref p) = outliner {
                        p.redo();
                    }
                }) as Box<dyn Fn()>)
            };

            // ── Autocomplete callbacks ──

            let on_ac_navigate_cb = {
                let ac_sel = ac_selected_effect;
                let ac_ctrl = ac_controller_effect;
                Closure::wrap(Box::new(move |direction: i32| {
                    ac_ctrl.update(|c| {
                        if let Some(ref mut ctrl) = c {
                            if direction > 0 {
                                ctrl.move_down();
                            } else {
                                ctrl.move_up();
                            }
                            ac_sel.set(ctrl.selected());
                        }
                    });
                }) as Box<dyn Fn(i32)>)
            };

            let on_ac_select_cb = {
                let c = content_signal;
                let ac_items_inner = ac_items_effect;
                let ac_selected_inner = ac_selected_effect;
                let ac_trigger_kind_inner = ac_trigger_kind_effect;
                let ac_visible_inner = ac_visible_effect;
                let ac_controller_inner = ac_controller_effect;
                let ac_result_inner = ac_result_effect;
                let on_save_ac = on_save_effect.clone();
                Closure::wrap(Box::new(move || {
                    let current = c.get_untracked();
                    let items = ac_items_inner.get_untracked();
                    let idx = ac_selected_inner.get_untracked();
                    let trigger_kind = ac_trigger_kind_inner.get_untracked();

                    // Re-detect trigger for computing insertion at end of content
                    let cursor = current.len();
                    let (trigger, _) = autocomplete_at_cursor(&current, cursor);
                    let entry = trigger.and_then(|t| {
                        items
                            .get(idx)
                            .and_then(|item| compute_insertion(&current, &t, item))
                    });

                    if let Some(ir) = entry {
                        let new_text = ir.new_content;
                        let new_cursor = ir.cursor_offset;
                        c.set(new_text.clone());

                        // Hide dropdown
                        ac_visible_inner.set(false);
                        ac_items_inner.set(Vec::new());
                        ac_trigger_kind_inner.set(None);
                        ac_controller_inner.set(None);

                        // Signal the CM6 update to the dedicated Effect
                        ac_result_inner.set(Some((new_text.clone(), new_cursor as u32)));

                        on_save_ac(new_text, trigger_kind);
                    }
                }) as Box<dyn Fn()>)
            };

            let on_ac_cancel_cb = {
                let ac_visible_inner = ac_visible_effect;
                let ac_items_inner = ac_items_effect;
                let ac_trigger_kind_inner = ac_trigger_kind_effect;
                let ac_controller_inner = ac_controller_effect;
                Closure::wrap(Box::new(move || {
                    ac_visible_inner.set(false);
                    ac_items_inner.set(Vec::new());
                    ac_trigger_kind_inner.set(None);
                    ac_controller_inner.set(None);
                }) as Box<dyn Fn()>)
            };

            // ── Build Cm6Callbacks ──
            let callbacks = Cm6Callbacks {
                on_change: Some(on_change_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_enter: Some(on_enter_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_tab: Some(on_tab_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_shift_tab: Some(on_shift_tab_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_escape: Some(on_escape_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_backspace: Some(on_backspace_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_ctrl_backspace: Some(on_ctrl_backspace_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_undo: Some(on_undo_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_redo: Some(on_redo_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_ac_navigate: Some(on_ac_navigate_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_ac_select: Some(on_ac_select_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
                on_ac_cancel: Some(on_ac_cancel_cb.as_ref().unchecked_ref::<js_sys::Function>().clone()),
            };

            // ── Create the editor (with retry for JS bundle race) ──
            match try_create_cm6(&el, &initial_content, &callbacks) {
                Ok(handle) => {
                    // Apply initial decorations immediately
                    let init_decos = compute_decorations_json(&initial_content);
                    let _ = handle.set_decorations(&init_decos);

                    cm6_handle.borrow_mut().replace(handle);

                    // Leak closures to keep them alive for editor lifetime
                    on_change_cb.forget();
                    on_enter_cb.forget();
                    on_tab_cb.forget();
                    on_shift_tab_cb.forget();
                    on_escape_cb.forget();
                    on_backspace_cb.forget();
                    on_ctrl_backspace_cb.forget();
                    on_undo_cb.forget();
                    on_redo_cb.forget();
                    on_ac_navigate_cb.forget();
                    on_ac_select_cb.forget();
                    on_ac_cancel_cb.forget();
                }
                Err(e) => {
                    log::warn!("CM6 init failed for block {}: {}", bid, e);
                }
            }
        }
    });

    // ── Sync external content changes into CM6 ──
    // When the block content changes externally (undo/redo via Outliner),
    // push the new content into the CM6 editor.
    Effect::new({
        let cm6_handle = cm6_handle.clone();
        move || {
            let saved = block.get().content;
            let content_val = content.get_untracked();
            if saved != content_val {
                content.set(saved.clone());
                if let Some(ref handle) = *cm6_handle.borrow() {
                    let _ = handle.set_content(&saved);
                    // Also recompute decorations
                    let decos = compute_decorations_json(&saved);
                    let _ = handle.set_decorations(&decos);
                }
            }
        }
    });

    // ── Blur handler: save on blur ──
    // NOTE: Does NOT touch cm6_handle directly (Rc<RefCell<>> is not
    // Send+Sync). The autocomplete visibility Effect handles syncing
    // state to CM6's keymap.
    let on_blur_cb = move |_| {
        ac_visible.set(false);
        on_save_blur.clone()(content.get_untracked(), None);
    };

    // ── Autocomplete select callback (mouse click) ──
    // NOTE: Does NOT touch cm6_handle directly. Uses ac_result signal
    // + dedicated Effect to push content+cursor+decorations to CM6.
    let handle_ac_select = {
        let content_signal = content;
        let ac_items = ac_items;
        let _ac_selected = ac_selected;
        let ac_trigger_kind = ac_trigger_kind;
        let ac_visible = ac_visible;
        let ac_controller = ac_controller;
        let ac_result = ac_result;
        let on_save = on_save;
        move |idx: usize| {
            let current = content_signal.get_untracked();
            let trigger_kind = ac_trigger_kind.get_untracked();
            let cursor = current.len();
            let (trigger, _) = autocomplete_at_cursor(&current, cursor);
            let items = ac_items.get_untracked();
            let entry = trigger.and_then(|t| {
                items
                    .get(idx)
                    .and_then(|item| compute_insertion(&current, &t, item))
            });
            if let Some(ir) = entry {
                let new_text = ir.new_content;
                let new_cursor = ir.cursor_offset;
                content_signal.set(new_text.clone());
                ac_visible.set(false);
                ac_items.set(Vec::new());
                ac_trigger_kind.set(None);
                ac_controller.set(None);
                // Signal the CM6 update via the dedicated Effect
                ac_result.set(Some((new_text.clone(), new_cursor as u32)));
                on_save(new_text, trigger_kind);
            }
        }
    };

    view! {
        <div class="block-editor-wrapper relative">
            <div
                node_ref=el_ref
                class="cm6-editor-container flex-1 text-sm min-h-[1.5em] outline-none break-words border-l-2 border-accent pl-1"
                on:blur=on_blur_cb
            >
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
