//! CodeMirror 6 backed block editor component.
//!
//! Replaces the contenteditable-based `BlockEditor` with a CM6 editor.
//! Key differences:
//! - Undo/redo is owned by the Outliner/HistoryStack, NOT by CM6.
//! - Custom keybindings dispatch tree operations (Enter, Tab, etc.)
//!   to the Rust `TreeOps` and `PageOutliner`.
//! - Single-line editing only (CM6 configured without history).
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

use crate::bridge::BlockDto;
use crate::editor::cm6_bridge::{self, Cm6Callbacks, Cm6Handle};
use crate::outliner::page::PageOutliner;
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
) -> impl IntoView {
    let content = RwSignal::new(block.get().content.clone());
    let el_ref: NodeRef<leptos::html::Div> = NodeRef::new();
    let on_save_blur = on_save.clone();
    let tree_ops = tree_ops.unwrap_or_default();
    let cm6_handle: Rc<RefCell<Option<Cm6Handle>>> = Rc::new(RefCell::new(None));

    // Page-level outliner coordinator (optional)
    let page_outliner: Option<PageOutliner> = use_context();

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

            let on_change_cb = {
                let c = content_signal;
                Closure::wrap(Box::new(move |text: String| {
                    c.set(text);
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
            };

            // ── Create the editor (with retry for JS bundle race) ──
            match try_create_cm6(&el, &initial_content, &callbacks) {
                Ok(handle) => {
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
                }
            }
        }
    });

    // ── Blur handler: save on blur ──
    let on_blur_cb = move |_| {
        on_save_blur.clone()(content.get_untracked(), None);
    };

    view! {
        <div class="block-editor-wrapper relative">
            <div
                node_ref=el_ref
                class="cm6-editor-container flex-1 text-sm min-h-[1.5em] outline-none break-words border-l-2 border-accent pl-1"
                on:blur=on_blur_cb
            >
            </div>
        </div>
    }
}
