//! Outliner block component for displaying hierarchical blocks
//!
//! Provides a tree component with:
//! - Indentation based on block level
//! - Marker and priority display
//! - Expand/collapse toggle

use crate::bridge::BlockDto;
use leptos::prelude::*;

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
pub fn OutlinerBlock(block: BlockDto, has_children: bool, expanded: bool) -> impl IntoView {
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

    let expand_icon = if !has_children {
        ""
    } else if expanded {
        "▼"
    } else {
        "▶"
    };

    // Calculate indentation based on level
    let indent_px = block.level.saturating_sub(1) as u32 * 24;

    view! {
        <div
            class="outliner-block"
            data-block-id={block.id.clone()}
            style:padding-left={format!("{}px", indent_px)}
        >
            <div class="outliner-block-row">
                {/* Expand/collapse toggle */}
                <Show when={move || has_children}>
                    <span class="outliner-expand">
                        {expand_icon}
                    </span>
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

                {/* Content */}
                <span class="block-content">{block.content.clone()}</span>
            </div>
        </div>
    }
}
