//! Virtual list component for efficient rendering of large lists
//!
//! Only renders items visible in the viewport plus a buffer zone.


/// Virtual list state for tracking scroll position
#[derive(Clone)]
pub struct VirtualListViewport {
    scroll_top: f64,
    container_height: f64,
    item_height: f64,
    total_items: usize,
    buffer_size: usize,
}

impl VirtualListViewport {
    pub fn new(item_height: f64, total_items: usize) -> Self {
        Self {
            scroll_top: 0.0,
            container_height: 400.0,
            item_height,
            total_items,
            buffer_size: 3,
        }
    }

    /// Start index (first visible item)
    pub fn start_index(&self) -> usize {
        let start = (self.scroll_top / self.item_height) as usize;
        start.saturating_sub(self.buffer_size)
    }

    /// End index (last visible item)
    pub fn end_index(&self) -> usize {
        let _visible_count = (self.container_height / self.item_height) as usize;
        let end = ((self.scroll_top + self.container_height) / self.item_height) as usize;
        (end + self.buffer_size).min(self.total_items)
    }

    /// Total height for scroll container
    pub fn total_height(&self) -> f64 {
        self.total_items as f64 * self.item_height
    }

    /// Padding top for invisible items above viewport
    pub fn offset_top(&self) -> f64 {
        self.start_index() as f64 * self.item_height
    }

    /// Update scroll position
    pub fn on_scroll(&mut self, scroll_top: f64) {
        self.scroll_top = scroll_top;
    }

    /// Update container height
    pub fn on_resize(&mut self, height: f64) {
        self.container_height = height;
    }

    /// Update total items
    pub fn update_items(&mut self, total: usize) {
        self.total_items = total;
    }
}
