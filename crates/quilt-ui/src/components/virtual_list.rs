//! Virtual list component for efficient rendering of large lists
//!
//! Only renders items visible in the viewport plus a buffer zone.

use leptos::prelude::*;

/// Visible range of items
#[derive(Clone, Copy, Debug)]
pub struct VisibleRange {
    pub start: usize,
    pub end: usize,
}

/// Virtual list state for tracking scroll position (Leptos reactive version)
#[derive(Clone)]
pub struct VirtualListState {
    /// Total number of items (stored in RwSignal for mutability)
    total_items: RwSignal<usize>,
    /// Height of each item in pixels
    pub item_height: f64,
    /// Number of items to render above/below viewport
    pub buffer_size: usize,
    /// Current scroll position
    pub scroll_top: RwSignal<f64>,
    /// Container height
    pub container_height: RwSignal<f64>,
}

impl VirtualListState {
    /// Create new virtual list state
    pub fn new(total_items: usize, item_height: f64) -> Self {
        Self {
            total_items: RwSignal::new(total_items),
            item_height,
            buffer_size: 3,
            scroll_top: RwSignal::new(0.0),
            container_height: RwSignal::new(400.0),
        }
    }

    /// Create from existing signals
    pub fn from_signals(
        total_items: RwSignal<usize>,
        item_height: f64,
        scroll_top: RwSignal<f64>,
        container_height: RwSignal<f64>,
    ) -> Self {
        Self {
            total_items,
            item_height,
            buffer_size: 3,
            scroll_top,
            container_height,
        }
    }

    /// Get total items signal for reading
    pub fn total_items_signal(&self) -> ReadSignal<usize> {
        self.total_items.read_only()
    }

    /// Calculate the start index (first visible item)
    pub fn start_index(&self) -> usize {
        let scroll_top = self.scroll_top.get();
        let start = (scroll_top / self.item_height) as usize;
        start.saturating_sub(self.buffer_size)
    }

    /// Calculate the end index (last visible item)
    pub fn end_index(&self) -> usize {
        let scroll_top = self.scroll_top.get();
        let container_height = self.container_height.get();
        let total_items = self.total_items.get();
        let _visible_count = (container_height / self.item_height) as usize;
        let end = ((scroll_top + container_height) / self.item_height) as usize;
        (end + self.buffer_size).min(total_items)
    }

    /// Get visible range
    pub fn visible_range(&self) -> VisibleRange {
        VisibleRange {
            start: self.start_index(),
            end: self.end_index(),
        }
    }

    /// Calculate the offset for the start item (for padding)
    pub fn start_offset(&self) -> f64 {
        self.start_index() as f64 * self.item_height
    }

    /// Calculate total height of all items
    pub fn total_height(&self) -> f64 {
        self.total_items.get() as f64 * self.item_height
    }

    /// Update scroll position
    pub fn update_scroll(&self, scroll_top: f64) {
        self.scroll_top.set(scroll_top);
    }

    /// Update container height
    pub fn update_container_height(&self, height: f64) {
        self.container_height.set(height);
    }

    /// Update total items (e.g., after filter)
    pub fn update_total(&self, total: usize) {
        self.total_items.set(total);
    }
}

/// Virtual list viewport component
#[component]
pub fn VirtualListViewport(
    #[prop(default = 400.0)] container_height: f64,
    #[prop(default = 50.0)] item_height: f64,
    total_items: ReadSignal<usize>,
    children: Children,
) -> impl IntoView {
    let scroll_top = RwSignal::new(0.0f64);
    let container_height_sig = RwSignal::new(container_height);
    let total_items_sig = RwSignal::new(total_items.get());

    // Update total_items when signal changes
    Effect::new(move |_| {
        total_items_sig.set(total_items.get());
    });

    let virtual_state = VirtualListState::from_signals(
        total_items_sig,
        item_height,
        scroll_top,
        container_height_sig,
    );

    // Clone for use in multiple closures
    let vs_for_height = virtual_state.clone();
    let vs_for_offset = virtual_state.clone();
    let vs_for_scroll = virtual_state.clone();
    let vs_for_resize = virtual_state.clone();

    let _visible_range = Signal::derive(move || virtual_state.visible_range());
    let total_height = Signal::derive(move || vs_for_height.total_height());
    let start_offset = Signal::derive(move || vs_for_offset.start_offset());

    view! {
        <div
            class="virtual-list-container"
            on:scroll={move |e| {
                use wasm_bindgen::JsCast;
                if let Some(target) = e.target() {
                    if let Ok(target) = target.dyn_into::<web_sys::HtmlDivElement>() {
                        vs_for_scroll.update_scroll(target.scroll_top() as f64);
                    }
                }
            }}
            on:resize={move |e| {
                use wasm_bindgen::JsCast;
                if let Some(target) = e.target() {
                    if let Ok(target) = target.dyn_into::<web_sys::HtmlDivElement>() {
                        vs_for_resize.update_container_height(target.client_height() as f64);
                    }
                }
            }}
        >
            <div
                class="virtual-list-content"
                style:height={move || format!("{}px", total_height.get())}
            >
                <div
                    class="virtual-list-offset"
                    style:height={move || format!("{}px", start_offset.get())}
                ></div>
                <div class="virtual-list-items">
                    {children()}
                </div>
            </div>
        </div>
    }
}
