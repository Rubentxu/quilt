use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CursorOffset(pub u32);

#[derive(Debug)]
pub enum DispatchResult {
    Handled,
    Bubble,
}

#[derive(Clone)]
pub struct KeyboardHandlers {
    pub on_enter: Arc<dyn Fn(u32) + 'static>,
    pub on_tab: Arc<dyn Fn() + 'static>,
    pub on_shift_tab: Arc<dyn Fn() + 'static>,
    pub on_backspace: Arc<dyn Fn(u32) + 'static>,
    pub on_escape: Arc<dyn Fn() + 'static>,
    pub on_ctrl_enter: Arc<dyn Fn(u32) + 'static>,
    pub on_ctrl_backspace: Arc<dyn Fn() + 'static>,
}

impl KeyboardHandlers {
    pub fn dispatch(
        &self,
        key: &str,
        modifiers: Modifiers,
        cursor_offset: u32,
        is_composing: bool,
    ) -> DispatchResult {
        if is_composing {
            return DispatchResult::Bubble;
        }

        match (key, modifiers.shift, modifiers.ctrl) {
            ("Enter", false, false) => {
                (self.on_enter)(cursor_offset);
                DispatchResult::Handled
            }
            ("Enter", true, false) => DispatchResult::Bubble,
            ("Tab", false, false) => {
                (self.on_tab)();
                DispatchResult::Handled
            }
            ("Tab", true, false) => {
                (self.on_shift_tab)();
                DispatchResult::Handled
            }
            ("Backspace", false, false) => {
                (self.on_backspace)(cursor_offset);
                DispatchResult::Handled
            }
            ("Escape", _, _) => {
                (self.on_escape)();
                DispatchResult::Handled
            }
            ("Enter", false, true) => {
                (self.on_ctrl_enter)(cursor_offset);
                DispatchResult::Handled
            }
            ("Backspace", false, true) => {
                (self.on_ctrl_backspace)();
                DispatchResult::Handled
            }
            _ => DispatchResult::Bubble,
        }
    }
}

pub fn get_cursor_offset(container: &web_sys::HtmlElement) -> u32 {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return 0,
    };
    let selection = match window.get_selection() {
        Ok(Some(s)) => s,
        _ => return 0,
    };
    let container_text = container.text_content().unwrap_or_default();
    let cursor_pos = selection.anchor_offset() as usize;
    if cursor_pos <= container_text.len() {
        cursor_pos as u32
    } else {
        0
    }
}

pub fn set_cursor(_container: &web_sys::HtmlElement, _offset: u32) {}
