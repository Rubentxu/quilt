//! Selection state for keyboard-first block navigation.
//!
//! Tracks which block is currently selected and mediates edit requests.
//! Provided as Leptos context by PageView; consumed by Block components.

use leptos::prelude::*;

/// Reactive selection state shared across all blocks on a page.
///
/// - `selected_block_id`: the block currently highlighted (blue border).
/// - `editing_block_id`: the block currently being edited (CM6 mounted).
///   `None` means no block is being edited.
/// - `edit_request`: one-shot signal — when set to `Some(block_id)`, the
///   matching Block component starts editing. Cleared after consumption.
#[derive(Clone, Copy)]
pub struct SelectionState {
    /// The block currently highlighted for keyboard navigation.
    pub selected_block_id: RwSignal<Option<String>>,
    /// The block currently being edited (if any) — CM6 is mounted.
    /// `None` when all blocks are in "selected" (non-editing) mode.
    pub editing_block_id: RwSignal<Option<String>>,
    /// One-shot: when set, the matching Block starts editing.
    /// Consumed by an Effect in the Block component.
    pub edit_request: RwSignal<Option<String>>,
    /// One-shot: when set, the matching Block toggles collapsed state.
    /// Consumed by an Effect in the Block component.
    pub collapse_request: RwSignal<Option<(String, bool)>>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected_block_id: RwSignal::new(None),
            editing_block_id: RwSignal::new(None),
            edit_request: RwSignal::new(None),
            collapse_request: RwSignal::new(None),
        }
    }

    /// Select a block by ID (deselects previous).
    pub fn select(&self, block_id: &str) {
        self.selected_block_id.set(Some(block_id.to_string()));
    }

    /// Deselect the current block.
    pub fn deselect(&self) {
        self.selected_block_id.set(None);
    }

    /// Mark a block as entering edit mode.
    pub fn set_editing(&self, block_id: &str) {
        self.editing_block_id.set(Some(block_id.to_string()));
    }

    /// Mark that no block is being edited.
    pub fn clear_editing(&self) {
        self.editing_block_id.set(None);
    }

    /// Request that a block starts editing on the next reactive tick.
    pub fn request_edit(&self, block_id: &str) {
        self.edit_request.set(Some(block_id.to_string()));
    }

    /// Request a collapse toggle on a block.
    pub fn request_collapse(&self, block_id: &str, collapsed: bool) {
        self.collapse_request
            .set(Some((block_id.to_string(), collapsed)));
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}
