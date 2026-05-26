//! CodeMirror 6 bridge — safe Rust wrapper around `window.__quiltCm6`.
//!
//! This module provides type-safe access to the CM6 JS API via `js-sys`
//! and `web-sys`. No `#[wasm_bindgen]` module imports needed — the JS
//! side exposes its API on the `window` object as a plain global.
//!
//! # Architecture
//!
//! - `Cm6Handle` wraps an opaque editor ID returned by JS.
//! - The JS side owns the actual `EditorView` instance.
//! - WebSocket-like pattern: create → use → destroy.
//! - All methods are fallible (return `Result`) since they cross the
//!   JS boundary and may fail if the global API is not available.
//!
//! # Safety
//!
//! The JS functions are assumed to be synchronous (CM6 operations are
//! synchronous within the JS event loop). We do not cross async boundaries.

use js_sys::{Function, Object, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Error returned by CM6 bridge operations.
#[derive(Debug, Clone)]
pub enum Cm6BridgeError {
    /// `window.__quiltCm6` is not available (script not loaded).
    GlobalNotAvailable,
    /// The requested method does not exist on the API object.
    MethodNotFound(String),
    /// The JS function threw or returned an unexpected value.
    JsError(String),
}

impl std::fmt::Display for Cm6BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GlobalNotAvailable => {
                write!(f, "CM6 global API (window.__quiltCm6) not available")
            }
            Self::MethodNotFound(m) => write!(f, "CM6 method not found: {}", m),
            Self::JsError(e) => write!(f, "CM6 JS error: {}", e),
        }
    }
}

impl std::error::Error for Cm6BridgeError {}

type Result<T> = std::result::Result<T, Cm6BridgeError>;

/// Callbacks from the Rust side that the CM6 editor invokes.
///
/// These are the bridge between CM6 events and the Rust/Leptos outliner.
#[derive(Clone)]
pub struct Cm6Callbacks {
    /// Called on every document change with the full text.
    pub on_change: Option<js_sys::Function>,
    /// Called when Enter is pressed without modifiers.
    /// Receives the cursor offset (u32) as argument.
    pub on_enter: Option<js_sys::Function>,
    /// Called when Tab is pressed without modifiers.
    pub on_tab: Option<js_sys::Function>,
    /// Called when Shift+Tab is pressed.
    pub on_shift_tab: Option<js_sys::Function>,
    /// Called when Escape is pressed (cancel editing).
    pub on_escape: Option<js_sys::Function>,
    /// Called when Backspace is pressed on empty content (merge intention).
    pub on_backspace: Option<js_sys::Function>,
    /// Called when Ctrl+Backspace is pressed (merge with next).
    pub on_ctrl_backspace: Option<js_sys::Function>,
    /// Called for Ctrl+Z (outliner undo).
    pub on_undo: Option<js_sys::Function>,
    /// Called for Ctrl+Shift+Z / Ctrl+Y (outliner redo).
    pub on_redo: Option<js_sys::Function>,
    // ── Autocomplete callbacks ──
    /// Called when ArrowUp (-1) or ArrowDown (1) is pressed while
    /// the autocomplete dropdown is active.
    pub on_ac_navigate: Option<js_sys::Function>,
    /// Called when Enter is pressed while the autocomplete dropdown
    /// is active (confirm selected item).
    pub on_ac_select: Option<js_sys::Function>,
    /// Called when Escape is pressed while the autocomplete dropdown
    /// is active (cancel/close dropdown).
    pub on_ac_cancel: Option<js_sys::Function>,
}

impl Cm6Callbacks {
    /// Create an empty callbacks struct (all disabled).
    pub fn empty() -> Self {
        Self {
            on_change: None,
            on_enter: None,
            on_tab: None,
            on_shift_tab: None,
            on_escape: None,
            on_backspace: None,
            on_ctrl_backspace: None,
            on_undo: None,
            on_redo: None,
            on_ac_navigate: None,
            on_ac_select: None,
            on_ac_cancel: None,
        }
    }

    /// Convert to a JS object suitable for `createEditor`.
    fn to_js_object(&self) -> Result<Object> {
        let obj = Object::new();
        if let Some(ref f) = self.on_change {
            Reflect::set(&obj, &"onChange".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onChange: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_enter {
            Reflect::set(&obj, &"onEnter".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onEnter: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_tab {
            Reflect::set(&obj, &"onTab".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onTab: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_shift_tab {
            Reflect::set(&obj, &"onShiftTab".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onShiftTab: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_escape {
            Reflect::set(&obj, &"onEscape".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onEscape: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_backspace {
            Reflect::set(&obj, &"onBackspace".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onBackspace: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_ctrl_backspace {
            Reflect::set(&obj, &"onCtrlBackspace".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onCtrlBackspace: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_undo {
            Reflect::set(&obj, &"onUndo".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onUndo: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_redo {
            Reflect::set(&obj, &"onRedo".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onRedo: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_ac_navigate {
            Reflect::set(&obj, &"onAcNavigate".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onAcNavigate: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_ac_select {
            Reflect::set(&obj, &"onAcSelect".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onAcSelect: {:?}", e)))?;
        }
        if let Some(ref f) = self.on_ac_cancel {
            Reflect::set(&obj, &"onAcCancel".into(), f)
                .map_err(|e| Cm6BridgeError::JsError(format!("set onAcCancel: {:?}", e)))?;
        }
        Ok(obj)
    }
}

/// Get a reference to the global `window.__quiltCm6` object.
fn get_global_api() -> Result<Object> {
    let window = web_sys::window().ok_or(Cm6BridgeError::GlobalNotAvailable)?;
    let api = Reflect::get(&window, &"__quiltCm6".into())
        .map_err(|_| Cm6BridgeError::GlobalNotAvailable)?;
    if api.is_undefined() || api.is_null() {
        return Err(Cm6BridgeError::GlobalNotAvailable);
    }
    Ok(api.dyn_into::<Object>().map_err(|_| Cm6BridgeError::GlobalNotAvailable)?)
}

/// Call a method on the global API with given args.
fn call_api_method(method: &str, args: &[JsValue]) -> Result<JsValue> {
    let api = get_global_api()?;
    let func = Reflect::get(&api, &method.into())
        .map_err(|_| Cm6BridgeError::MethodNotFound(method.to_string()))?;
    let func = func
        .dyn_into::<Function>()
        .map_err(|_| Cm6BridgeError::MethodNotFound(method.to_string()))?;
    let this = JsValue::from(&api);
    let js_args = js_sys::Array::new();
    for arg in args {
        js_args.push(arg);
    }
    func.apply(&this, &js_args)
        .map_err(|e| Cm6BridgeError::JsError(format!("{} failed: {:?}", method, e)))
}

/// A handle to a live CM6 editor instance.
///
/// Drop this to destroy the editor (calls `destroyEditor` on JS side).
#[derive(Debug)]
pub struct Cm6Handle {
    id: JsValue,
}

impl Cm6Handle {
    /// Create a new CM6 editor inside the given DOM element.
    ///
    /// - `container`: The DOM element to mount into.
    /// - `content`: Initial text content.
    /// - `callbacks`: Rust-side callbacks for editor events.
    pub fn create(
        container: &web_sys::Element,
        content: &str,
        callbacks: &Cm6Callbacks,
    ) -> Result<Self> {
        let cbs_obj = callbacks.to_js_object()?;
        let id = call_api_method(
            "createEditor",
            &[container.into(), content.into(), JsValue::from(&cbs_obj)],
        )?;
        Ok(Self { id })
    }

    /// Get the current editor content.
    pub fn get_content(&self) -> Result<String> {
        let result = call_api_method("getContent", &[self.id.clone()])?;
        result
            .as_string()
            .ok_or_else(|| Cm6BridgeError::JsError("getContent did not return a string".into()))
    }

    /// Replace the entire editor content.
    ///
    /// Does NOT trigger the `onChange` callback.
    pub fn set_content(&self, content: &str) -> Result<()> {
        call_api_method("setContent", &[self.id.clone(), content.into()])?;
        Ok(())
    }

    /// Focus the editor.
    pub fn focus(&self) -> Result<()> {
        call_api_method("focus", &[self.id.clone()])?;
        Ok(())
    }

    /// Set content and cursor position in a single transaction.
    /// Does NOT fire the onChange callback.
    pub fn set_content_with_cursor(&self, content: &str, cursor_offset: u32) -> Result<()> {
        call_api_method(
            "setContentAndCursor",
            &[self.id.clone(), content.into(), JsValue::from(cursor_offset)],
        )?;
        Ok(())
    }

    /// Tell CM6 whether the autocomplete dropdown is active.
    /// When active, ArrowUp/Down/Enter/Escape are intercepted for
    /// autocomplete navigation instead of cursor movement.
    pub fn set_autocomplete_state(&self, active: bool) -> Result<()> {
        call_api_method(
            "setAutocompleteState",
            &[self.id.clone(), JsValue::from(active)],
        )?;
        Ok(())
    }

    /// Apply visual decorations (parser-based highlighting) to the editor.
    ///
    /// `decorations_json` is a JSON array of `{from, to, class}` objects
    /// describing the visual decorations to apply.
    pub fn set_decorations(&self, decorations_json: &str) -> Result<()> {
        call_api_method(
            "setDecorations",
            &[self.id.clone(), decorations_json.into()],
        )?;
        Ok(())
    }

    /// Get the cursor offset (character position) within the document.
    /// Returns 0 if unavailable.
    pub fn cursor_offset(&self) -> Result<u32> {
        let result = call_api_method("getCursorOffset", &[self.id.clone()])?;
        result
            .as_f64()
            .map(|n| n as u32)
            .ok_or_else(|| Cm6BridgeError::JsError("getCursorOffset did not return a number".into()))
    }

    /// Get cursor viewport-relative coordinates.
    ///
    /// Returns `(top, left, bottom)` in CSS pixels relative to the viewport,
    /// or `None` if position cannot be determined.
    pub fn cursor_coords(&self) -> Option<(f64, f64, f64)> {
        let result = call_api_method("getCursorCoords", &[self.id.clone()]).ok()?;
        if result.is_null() || result.is_undefined() {
            return None;
        }
        let obj = result.dyn_into::<Object>().ok()?;
        let top = Reflect::get(&obj, &"top".into()).ok()?.as_f64()?;
        let left = Reflect::get(&obj, &"left".into()).ok()?.as_f64()?;
        let bottom = Reflect::get(&obj, &"bottom".into()).ok()?.as_f64()?;
        Some((top, left, bottom))
    }

    /// Access the raw JsValue ID (for FFI edge cases).
    pub fn id(&self) -> &JsValue {
        &self.id
    }
}

impl Drop for Cm6Handle {
    fn drop(&mut self) {
        // Best-effort destroy — ignore errors (may happen during page teardown)
        let _ = call_api_method("destroyEditor", &[self.id.clone()]);
    }
}

/// Check whether the CM6 global API is available in the current environment.
///
/// Returns `true` if `window.__quiltCm6` is defined and callable.
pub fn is_cm6_available() -> bool {
    get_global_api().is_ok()
}

// ── Tests ──
// Tests for Cm6BridgeError formatting only (no WASM environment in unit tests).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_global_not_available() {
        let err = Cm6BridgeError::GlobalNotAvailable;
        assert_eq!(
            err.to_string(),
            "CM6 global API (window.__quiltCm6) not available"
        );
    }

    #[test]
    fn test_error_display_method_not_found() {
        let err = Cm6BridgeError::MethodNotFound("foobar".into());
        assert_eq!(err.to_string(), "CM6 method not found: foobar");
    }

    #[test]
    fn test_error_display_js_error() {
        let err = Cm6BridgeError::JsError("something broke".into());
        assert_eq!(err.to_string(), "CM6 JS error: something broke");
    }

    #[test]
    fn test_cm6_callbacks_empty_works() {
        // Verify empty callbacks can be constructed (non-WASM target).
        // to_js_object() requires js-sys which panics off WASM,
        // so we only test construction.
        let cbs = Cm6Callbacks::empty();
        assert!(cbs.on_change.is_none());
        assert!(cbs.on_enter.is_none());
        assert!(cbs.on_tab.is_none());
        assert!(cbs.on_shift_tab.is_none());
        assert!(cbs.on_escape.is_none());
        assert!(cbs.on_backspace.is_none());
        assert!(cbs.on_ctrl_backspace.is_none());
        assert!(cbs.on_undo.is_none());
        assert!(cbs.on_redo.is_none());
        assert!(cbs.on_ac_navigate.is_none());
        assert!(cbs.on_ac_select.is_none());
        assert!(cbs.on_ac_cancel.is_none());
    }
}
