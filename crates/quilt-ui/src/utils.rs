//! Performance utilities for Quilt UI
//!
//! Provides:
//! - Virtual list state for efficient rendering

/// Virtual list state for efficient rendering of large lists
///
/// Only renders items visible in the viewport plus a buffer.
#[derive(Clone)]
pub struct VirtualListState {
    /// Total number of items
    pub total_items: usize,
    /// Height of each item in pixels
    pub item_height: f64,
    /// Number of items to render above/below viewport
    pub buffer_size: usize,
    /// Current scroll position
    pub scroll_top: f64,
    /// Container height
    pub container_height: f64,
}

impl VirtualListState {
    /// Create new virtual list state
    pub fn new(total_items: usize, item_height: f64) -> Self {
        Self {
            total_items,
            item_height,
            buffer_size: 3,
            scroll_top: 0.0,
            container_height: 400.0,
        }
    }

    /// Calculate the start index (first visible item)
    pub fn start_index(&self) -> usize {
        let start = (self.scroll_top / self.item_height) as usize;
        start.saturating_sub(self.buffer_size)
    }

    /// Calculate the end index (last visible item)
    pub fn end_index(&self) -> usize {
        let _visible_count = (self.container_height / self.item_height) as usize;
        let end = ((self.scroll_top + self.container_height) / self.item_height) as usize;
        (end + self.buffer_size).min(self.total_items)
    }

    /// Calculate the offset for the start item (for padding)
    pub fn start_offset(&self) -> f64 {
        self.start_index() as f64 * self.item_height
    }

    /// Calculate total height of all items
    pub fn total_height(&self) -> f64 {
        self.total_items as f64 * self.item_height
    }

    /// Update scroll position
    pub fn update_scroll(&mut self, scroll_top: f64) {
        self.scroll_top = scroll_top;
    }

    /// Update container height
    pub fn update_container_height(&mut self, height: f64) {
        self.container_height = height;
    }

    /// Update total items (e.g., after filter)
    pub fn update_total(&mut self, total: usize) {
        self.total_items = total;
    }
}
