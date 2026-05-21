//! Breadcrumb component for displaying block path in deep nesting
//!
//! Shows the path from root to current block for orientation.

use leptos::prelude::*;

/// Breadcrumb component for block path navigation
#[component]
pub fn BlockBreadcrumb(path: Signal<Vec<String>>) -> impl IntoView {
    view! {
        <nav class="breadcrumb" aria-label="Block path">
            <ol class="breadcrumb-list">
                <For each={move || {
                    let segments = path.get();
                    segments
                        .into_iter()
                        .enumerate()
                        .map(|(i, segment)| {
                            let is_last = i == path.get().len().saturating_sub(1);
                            view! {
                                <li class="breadcrumb-item">
                                    <span class="breadcrumb-segment">{segment}</span>
                                    {if !is_last {
                                        view! {
                                            <span class="breadcrumb-separator" aria-hidden="true">
                                                " > "
                                            </span>
                                        }
                                    } else {
                                        view! { <></> }
                                    }}
                                </li>
                            }
                        })
                        .collect::<Vec<_>>()
                }} key=|(_, segment)| segment.clone() let:item>
                    {item}
                </For>
            </ol>
        </nav>
    }
}

/// Compact breadcrumb for inline display
#[component]
pub fn InlineBreadcrumb(path: Signal<Vec<String>>) -> impl IntoView {
    let display_path = Signal::derive(move || {
        let segments = path.get();
        if segments.len() <= 3 {
            segments.join(" > ")
        } else {
            let start = &segments[..1];
            let end = &segments[segments.len() - 1..];
            format!("{} > ... > {}", start.join(""), end.join(""))
        }
    });

    view! {
        <span class="inline-breadcrumb" title={path.get().join(" > ")}>
            {display_path.get()}
        </span>
    }
}
