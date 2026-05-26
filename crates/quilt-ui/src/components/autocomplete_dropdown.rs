//! Autocomplete dropdown component and controller.
//!
//! Provides a lightweight dropdown UI for displaying autocomplete
//! suggestions and handling keyboard navigation (up/down/enter/escape).
//!
//! The `DropdownController` is a pure state machine extracted from
//! the Leptos component for testability. The `AutocompleteDropdown`
//! component wraps it in a UI view.

use crate::parser::autocomplete::AutocompleteItem;
use leptos::prelude::*;

// ── DropdownController (Pure Logic) ──

/// Pure state machine for dropdown keyboard navigation.
///
/// Extracted from the Leptos component so navigation logic can be
/// unit-tested without a WASM environment.
///
/// Usage:
/// ```ignore
/// // Full test coverage is in the #[cfg(test)] module below.
/// // This example is informational only.
/// let mut ctrl = DropdownController::new(10);
/// assert_eq!(ctrl.selected(), 0);
/// ctrl.move_down(); // → 1
/// ctrl.move_up();   // → 0
/// ctrl.move_down(); // → 1
/// ctrl.reset();
/// assert_eq!(ctrl.selected(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropdownController {
    selected: usize,
    item_count: usize,
}

impl DropdownController {
    /// Create a new controller for a list of `item_count` items.
    ///
    /// Panics if `item_count` is 0 — use `has_items()` or check
    /// `item_count > 0` before constructing.
    pub fn new(item_count: usize) -> Self {
        assert!(
            item_count > 0,
            "DropdownController requires at least 1 item"
        );
        Self {
            selected: 0,
            item_count,
        }
    }

    /// Current selected index.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Total number of items.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Move selection down by one, wrapping to first.
    pub fn move_down(&mut self) {
        self.selected = (self.selected + 1) % self.item_count;
    }

    /// Move selection up by one, wrapping to last.
    pub fn move_up(&mut self) {
        self.selected = if self.selected == 0 {
            self.item_count - 1
        } else {
            self.selected - 1
        };
    }

    /// Whether the current selection is at the last item.
    pub fn is_at_end(&self) -> bool {
        self.selected == self.item_count - 1
    }

    /// Whether the current selection is at the first item.
    pub fn is_at_start(&self) -> bool {
        self.selected == 0
    }

    /// Get the index that would result from pressing Enter.
    /// This is just `selected()` — it's the item that should be inserted.
    pub fn confirm_index(&self) -> usize {
        self.selected
    }

    /// Reset selection to first item.
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    /// Update item count (e.g., when suggestions change).
    /// Resets selection to 0.
    pub fn resize(&mut self, new_count: usize) {
        if new_count == 0 {
            return; // Don't allow 0 items — caller should not render dropdown
        }
        self.item_count = new_count;
        self.selected = 0;
    }

    /// Check if there are items to navigate.
    pub fn has_items(&self) -> bool {
        self.item_count > 0
    }
}

// ── AutocompleteDropdown Component ──

/// A lightweight dropdown for autocomplete suggestions.
///
/// Renders as a positioned popup below the trigger point.
/// Keyboard navigation (up/down/enter/escape) is handled by the parent
/// via `DropdownController`.
///
/// The component itself is purely visual — it renders the items with
/// the currently selected index highlighted.
///
/// Note: This component uses plain Leptos nodes instead of `<For>` to
/// avoid macro parsing edge cases with complex closures.
#[component]
pub fn AutocompleteDropdown(
    /// Items to display.
    items: Vec<AutocompleteItem>,
    /// Currently selected index.
    selected_index: usize,
    /// Callback when user clicks an item.
    on_select: impl Fn(usize) + 'static + Clone,
    /// Whether the dropdown is visible.
    visible: bool,
) -> leptos::prelude::AnyView {
    if !visible || items.is_empty() {
        return (view! { <div></div> }).into_any();
    }

    let sel = selected_index;
    let mut item_nodes: Vec<leptos::prelude::AnyView> = Vec::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        let on_item = on_select.clone();
        let label = item.label.clone();
        let desc = item.description.clone();
        let is_selected = idx == sel;

        let base_class =
            "flex items-center gap-2 px-3 py-1.5 text-sm cursor-pointer transition-colors";
        let row_class = if is_selected {
            format!("{} selected-item", base_class)
        } else {
            base_class.to_string()
        };

        let node: leptos::prelude::AnyView = if let Some(d) = desc {
            (view! {
                <div
                    class={row_class}
                    on:click=move |_| on_item(idx)
                >
                    <span class="text-text">{label}</span>
                    <span class="text-text-muted text-xs ml-auto">{d}</span>
                </div>
            })
            .into_any()
        } else {
            (view! {
                <div
                    class={row_class}
                    on:click=move |_| on_item(idx)
                >
                    <span class="text-text">{label}</span>
                </div>
            })
            .into_any()
        };

        item_nodes.push(node);
    }

    (view! {
        <div class="autocomplete-dropdown">
            <div class="bg-surface border border-border rounded-lg shadow-lg py-1 max-h-48 overflow-y-auto">
                {item_nodes}
            </div>
        </div>
    }).into_any()
}

#[cfg(test)]
mod tests {
    use super::DropdownController;

    // ── DropdownController Tests ──

    #[test]
    fn test_controller_new_selects_first() {
        let ctrl = DropdownController::new(5);
        assert_eq!(ctrl.selected(), 0);
        assert!(ctrl.is_at_start());
        assert!(!ctrl.is_at_end());
    }

    #[test]
    fn test_controller_move_down() {
        let mut ctrl = DropdownController::new(5);
        ctrl.move_down();
        assert_eq!(ctrl.selected(), 1);
        ctrl.move_down();
        assert_eq!(ctrl.selected(), 2);
    }

    #[test]
    fn test_controller_move_up() {
        let mut ctrl = DropdownController::new(5);
        ctrl.move_down();
        ctrl.move_down();
        assert_eq!(ctrl.selected(), 2);
        ctrl.move_up();
        assert_eq!(ctrl.selected(), 1);
    }

    #[test]
    fn test_controller_wraps_around_down() {
        let mut ctrl = DropdownController::new(3);
        ctrl.move_down(); // 1
        ctrl.move_down(); // 2
        assert!(ctrl.is_at_end());
        ctrl.move_down(); // wraps to 0
        assert_eq!(ctrl.selected(), 0);
        assert!(ctrl.is_at_start());
    }

    #[test]
    fn test_controller_wraps_around_up() {
        let mut ctrl = DropdownController::new(3);
        ctrl.move_up(); // wraps to 2
        assert!(ctrl.is_at_end());
        assert_eq!(ctrl.selected(), 2);
        ctrl.move_up(); // 1
        assert_eq!(ctrl.selected(), 1);
    }

    #[test]
    fn test_controller_reset() {
        let mut ctrl = DropdownController::new(5);
        ctrl.move_down();
        ctrl.move_down();
        ctrl.move_down();
        assert_eq!(ctrl.selected(), 3);
        ctrl.reset();
        assert_eq!(ctrl.selected(), 0);
    }

    #[test]
    fn test_controller_resize() {
        let mut ctrl = DropdownController::new(5);
        ctrl.move_down();
        ctrl.move_down();
        ctrl.move_down();
        assert_eq!(ctrl.selected(), 3);

        ctrl.resize(2);
        assert_eq!(ctrl.selected(), 0);
        assert_eq!(ctrl.item_count(), 2);
    }

    #[test]
    fn test_controller_confirm_index() {
        let mut ctrl = DropdownController::new(5);
        assert_eq!(ctrl.confirm_index(), 0);
        ctrl.move_down();
        ctrl.move_down();
        assert_eq!(ctrl.confirm_index(), 2);
    }

    #[test]
    fn test_controller_has_items() {
        let ctrl = DropdownController::new(3);
        assert!(ctrl.has_items());
    }

    #[test]
    fn test_controller_single_item() {
        let mut ctrl = DropdownController::new(1);
        assert_eq!(ctrl.selected(), 0);
        assert!(ctrl.is_at_start());
        assert!(ctrl.is_at_end());
        ctrl.move_down(); // stays on 0 (wraps)
        assert_eq!(ctrl.selected(), 0);
        ctrl.move_up(); // stays on 0 (wraps)
        assert_eq!(ctrl.selected(), 0);
    }

    #[test]
    #[should_panic(expected = "DropdownController requires at least 1 item")]
    fn test_controller_zero_items_panics() {
        let _ctrl = DropdownController::new(0);
    }

    #[test]
    fn test_controller_resize_zero_is_noop() {
        let mut ctrl = DropdownController::new(3);
        ctrl.move_down();
        ctrl.resize(0);
        // Should not change since 0 is invalid
        assert_eq!(ctrl.item_count(), 3);
        assert_eq!(ctrl.selected(), 1);
    }

    // ── Triangulation: edge cases ──

    #[test]
    fn test_controller_large_list() {
        let mut ctrl = DropdownController::new(100);
        for _ in 0..50 {
            ctrl.move_down();
        }
        assert_eq!(ctrl.selected(), 50);
        ctrl.move_up();
        assert_eq!(ctrl.selected(), 49);
    }

    #[test]
    fn test_controller_full_cycle() {
        let mut ctrl = DropdownController::new(4);
        // Cycle: 0→1→2→3→0→3→2→1→0
        ctrl.move_down(); // 1
        ctrl.move_down(); // 2
        ctrl.move_down(); // 3
        assert!(ctrl.is_at_end());
        ctrl.move_down(); // 0 (wrap)
        assert!(ctrl.is_at_start());
        ctrl.move_up(); // 3 (wrap up)
        assert!(ctrl.is_at_end());
        ctrl.move_up(); // 2
        ctrl.move_up(); // 1
        ctrl.move_up(); // 0
        assert!(ctrl.is_at_start());
    }
}
