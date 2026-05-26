//! Task item component

use leptos::prelude::*;
use crate::bridge::BlockDto;

/// Individual task item in a task list
#[component]
pub fn TaskItem(block: BlockDto) -> impl IntoView {
    let marker = block.marker.clone().unwrap_or_default();
    let priority = block.priority.clone();

    let marker_class = match marker.as_str() {
        "todo" => "marker-todo",
        "now" => "marker-now",
        "done" => "marker-done",
        _ => "",
    };

    let marker_icon = match marker.as_str() {
        "todo" => "○",
        "now" => "●",
        "done" => "✓",
        _ => "•",
    };

    view! {
        <div class="task-item">
            <span class=format!("task-marker {}", marker_class)>{marker_icon}</span>
            <span class="task-content">{block.content}</span>
            {priority.map(|p| {
                let p_class = format!("priority-{}", p.to_lowercase());
                view! {
                    <span class=format!("task-priority {}", p_class)>{p.to_uppercase()}</span>
                }
            })}
        </div>
    }
}
