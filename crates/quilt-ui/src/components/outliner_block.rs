//! Outliner block component for displaying hierarchical blocks
//!
//! Provides a tree component with:
//! - Indentation based on block level
//! - Marker and priority display
//! - Expand/collapse toggle
//! - Keyboard navigation (Tab, Shift+Tab, Enter, Escape, ArrowUp/ArrowDown)
//!
//! Note: Inline editing is pending due to closure capture constraints in Leptos 0.7

use crate::bridge::BlockDto;
use leptos::prelude::*;
use web_sys::KeyboardEvent;

/// Marker options for blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    None,
    Todo,
    Doing,
    Done,
}

impl Marker {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &Option<String>) -> Self {
        match s.as_deref() {
            Some("todo") => Marker::Todo,
            Some("doing") => Marker::Doing,
            Some("done") => Marker::Done,
            _ => Marker::None,
        }
    }
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    None,
    A,
    B,
    C,
}

impl Priority {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &Option<String>) -> Self {
        match s.as_deref() {
            Some("a") => Priority::A,
            Some("b") => Priority::B,
            Some("c") => Priority::C,
            _ => Priority::None,
        }
    }
}

/// Outliner block component - displays a single block with indentation
#[component]
pub fn OutlinerBlock(
    block: BlockDto,
    has_children: bool,
    expanded: RwSignal<bool>,
    on_collapse: Option<Callback<(), ()>>,
    on_focus_next: Option<Callback<(), ()>>,
    on_focus_prev: Option<Callback<(), ()>>,
) -> impl IntoView {
    let marker = Marker::from_str(&block.marker);
    let priority = Priority::from_str(&block.priority);

    let marker_class = match marker {
        Marker::Todo => "marker-todo",
        Marker::Doing => "marker-doing",
        Marker::Done => "marker-done",
        Marker::None => "",
    };

    let priority_class = match priority {
        Priority::A => "priority-a",
        Priority::B => "priority-b",
        Priority::C => "priority-c",
        Priority::None => "",
    };

    let priority_label = match priority {
        Priority::A => "A",
        Priority::B => "B",
        Priority::C => "C",
        Priority::None => "",
    };

    let marker_icon = match marker {
        Marker::Todo => "○",
        Marker::Doing => "◐",
        Marker::Done => "✓",
        Marker::None => "•",
    };

    // Calculate indentation based on level
    let indent_px = block.level.saturating_sub(1) as u32 * 24;

    // Clone block.id for use in closures
    let block_id = block.id.clone();
    let block_id2 = block.id.clone();
    let block_id3 = block.id.clone();

    view! {
        <div
            class="outliner-block"
            data-block-id={block_id.clone()}
            style:padding-left={format!("{}px", indent_px)}
            tabindex="0"
            on:keydown={move |ev: KeyboardEvent| {
                let key = ev.key();
                match key.as_str() {
                    "Enter" => {
                        // TODO: Enter edit mode when inline editing is implemented
                    }
                    "ArrowDown" => {
                        if let Some(callback) = &on_focus_next {
                            callback.run(());
                        }
                    }
                    "ArrowUp" => {
                        if let Some(callback) = &on_focus_prev {
                            callback.run(());
                        }
                    }
                    "Tab" => {
                        ev.prevent_default();
                        if ev.shift_key() {
                            if let Some(callback) = &on_focus_prev {
                                callback.run(());
                            }
                        } else if let Some(callback) = &on_focus_next {
                            callback.run(());
                        }
                    }
                    _ => {}
                }
            }}
        >
            <div class="outliner-block-row">
                {/* Expand/collapse toggle */}
                <Show when={move || has_children}>
                    <button
                        class="outliner-expand"
                        on:click={move |_ev: web_sys::MouseEvent| {
                            expanded.update(|e| *e = !*e);
                            if let Some(callback) = &on_collapse {
                                callback.run(());
                            }
                        }}
                        data-testid={format!("block-expand-{}", block_id2)}
                        tabindex="-1"
                    >
                        {move || if expanded.get() { "▼" } else { "▶" }}
                    </button>
                </Show>

                {/* Marker indicator */}
                <span class={format!("task-marker {}", marker_class)}>
                    {marker_icon}
                </span>

                {/* Priority indicator */}
                <Show when={move || priority != Priority::None}>
                    <span class={format!("task-priority {}", priority_class)}>
                        {priority_label}
                    </span>
                </Show>

                {/* Content - display only for now */}
                <span
                    class="block-content"
                    data-testid={format!("block-content-{}", block_id3)}
                >
                    {block.content.clone()}
                </span>
            </div>
        </div>
    }
}
