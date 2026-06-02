//! Drag-and-drop state for block reordering.
//!
//! Tracks the currently dragged block, the drop target, and the drop position.
//! Provided as Leptos context by PageView; consumed by Block components for
//! drag source and drop target behavior.

use crate::outliner::tree::DropPosition;
use leptos::prelude::*;

/// Reactive drag-and-drop state shared across all blocks on a page.
///
/// - `drag_source_id`: the block currently being dragged.
/// - `drag_active`: true while a drag operation is in progress.
/// - `drop_target_id`: the block currently being hovered as a drop target.
/// - `drop_position`: where relative to the target the drop would land.
#[derive(Clone, Copy)]
pub struct DragState {
    /// The block being dragged.
    pub drag_source_id: RwSignal<Option<String>>,
    /// True while a drag operation is active.
    pub drag_active: RwSignal<bool>,
    /// The block being hovered as a potential drop target.
    pub drop_target_id: RwSignal<Option<String>>,
    /// Where relative to the target the drop would land.
    pub drop_position: RwSignal<Option<DropPosition>>,
}

impl DragState {
    pub fn new() -> Self {
        Self {
            drag_source_id: RwSignal::new(None),
            drag_active: RwSignal::new(false),
            drop_target_id: RwSignal::new(None),
            drop_position: RwSignal::new(None),
        }
    }

    /// Start dragging a block.
    pub fn start_drag(&self, block_id: &str) {
        self.drag_source_id.set(Some(block_id.to_string()));
        self.drag_active.set(true);
    }

    /// Update the current drop target and position.
    pub fn set_drop_target(&self, block_id: &str, position: DropPosition) {
        self.drop_target_id.set(Some(block_id.to_string()));
        self.drop_position.set(Some(position));
    }

    /// Clear the current drop target (e.g., when the cursor leaves a block).
    pub fn clear_drop_target(&self) {
        self.drop_target_id.set(None);
        self.drop_position.set(None);
    }

    /// Clear all drag state (on drop or dragend).
    pub fn clear_drag(&self) {
        self.drag_source_id.set(None);
        self.drag_active.set(false);
        self.drop_target_id.set(None);
        self.drop_position.set(None);
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}
