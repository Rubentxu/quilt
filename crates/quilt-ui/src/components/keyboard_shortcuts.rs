//! Keyboard shortcuts module for global hotkey handling
//!
//! Provides keyboard event handling for common actions:
//! - Cmd/Ctrl + \ : Toggle right sidebar
//! - / : Trigger slash command palette (when not in input field)
//! - Escape : Close modals/palettes
//! - Cmd/Ctrl + k : Quick search

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Element, EventTarget, HtmlElement, KeyboardEvent};

/// Keyboard shortcut action enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    ToggleSidebar,
    ToggleSearch,
    CloseModal,
    TriggerSlashCommand,
    ToggleMobileMenu,
    Unknown,
}

/// Parse a keyboard event to determine the shortcut action
pub fn parse_keyboard_shortcut(event: &KeyboardEvent) -> ShortcutAction {
    let ctrl = event.ctrl_key();
    let meta = event.meta_key();
    let _shift = event.shift_key();
    let key = event.key();

    // Cmd/Ctrl + \ = Toggle sidebar
    if (ctrl || meta) && key == "\\" {
        return ShortcutAction::ToggleSidebar;
    }

    // Cmd/Ctrl + k = Quick search
    if (ctrl || meta) && key.to_lowercase() == "k" {
        return ShortcutAction::ToggleSearch;
    }

    // Escape = Close modal/palette
    if key == "Escape" {
        return ShortcutAction::CloseModal;
    }

    // / = Slash command (only when not in an input field)
    if key == "/" && !is_input_element(event.target()) {
        return ShortcutAction::TriggerSlashCommand;
    }

    // Cmd/Ctrl + m = Toggle mobile menu
    if (ctrl || meta) && key.to_lowercase() == "m" {
        return ShortcutAction::ToggleMobileMenu;
    }

    ShortcutAction::Unknown
}

/// Check if the event target is an input element
fn is_input_element(target: Option<EventTarget>) -> bool {
    if let Some(target) = target {
        if let Ok(element) = target.dyn_into::<HtmlElement>() {
            let tag_name = element.tag_name().to_lowercase();
            return tag_name == "input"
                || tag_name == "textarea"
                || tag_name == "select"
                || element.is_content_editable();
        }
    }
    false
}

/// Keyboard shortcuts provider - uses simple closures instead of Callback
pub struct KeyboardShortcuts {
    pub on_toggle_sidebar: Box<dyn Fn()>,
    pub on_toggle_search: Box<dyn Fn()>,
    pub on_close_modal: Box<dyn Fn()>,
    pub on_trigger_slash: Box<dyn Fn()>,
    pub on_toggle_mobile_menu: Box<dyn Fn()>,
}

impl KeyboardShortcuts {
    /// Set up keyboard event listener
    pub fn new(
        on_toggle_sidebar: impl Fn() + 'static,
        on_toggle_search: impl Fn() + 'static,
        on_close_modal: impl Fn() + 'static,
        on_trigger_slash: impl Fn() + 'static,
        on_toggle_mobile_menu: impl Fn() + 'static,
    ) -> Self {
        Self {
            on_toggle_sidebar: Box::new(on_toggle_sidebar),
            on_toggle_search: Box::new(on_toggle_search),
            on_close_modal: Box::new(on_close_modal),
            on_trigger_slash: Box::new(on_trigger_slash),
            on_toggle_mobile_menu: Box::new(on_toggle_mobile_menu),
        }
    }

    /// Mount the keyboard shortcuts
    pub fn mount(&self) {
        let on_toggle_sidebar = &self.on_toggle_sidebar;
        let on_toggle_search = &self.on_toggle_search;
        let on_close_modal = &self.on_close_modal;
        let on_trigger_slash = &self.on_trigger_slash;
        let on_toggle_mobile_menu = &self.on_toggle_mobile_menu;

        let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            let action = parse_keyboard_shortcut(&event);
            match action {
                ShortcutAction::ToggleSidebar => {
                    event.prevent_default();
                    on_toggle_sidebar();
                }
                ShortcutAction::ToggleSearch => {
                    event.prevent_default();
                    on_toggle_search();
                }
                ShortcutAction::CloseModal => {
                    event.prevent_default();
                    on_close_modal();
                }
                ShortcutAction::TriggerSlashCommand => {
                    event.prevent_default();
                    on_trigger_slash();
                }
                ShortcutAction::ToggleMobileMenu => {
                    event.prevent_default();
                    on_toggle_mobile_menu();
                }
                ShortcutAction::Unknown => {}
            }
        }) as Box<dyn Fn(KeyboardEvent)>);

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            // Leak the closure to keep it alive
            closure.forget();
        }
    }
}

/// Hook to check if we're currently in an input field
#[allow(dead_code)]
pub fn is_in_input_field() -> bool {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(active_element) = document.active_element() {
                if let Ok(element) = active_element.dyn_into::<Element>() {
                    let tag_name = element.tag_name().to_lowercase();
                    return tag_name == "input"
                        || tag_name == "textarea"
                        || element.get_attribute("contenteditable").is_some();
                }
            }
        }
    }
    false
}
