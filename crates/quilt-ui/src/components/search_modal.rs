//! Search modal component with Cmd+K shortcut
//!
//! Provides a quick search overlay for finding blocks, pages, and commands.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub result_type: SearchResultType,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SearchResultType {
    Page,
    Block,
    Command,
}

impl SearchResultType {
    pub fn icon(&self) -> &str {
        match self {
            SearchResultType::Page => "📄",
            SearchResultType::Block => "▢",
            SearchResultType::Command => "⌘",
        }
    }
}

#[component]
pub fn SearchModal(
    is_open: bool,
    query: String,
    results: Vec<SearchResult>,
    on_select: Callback<SearchResult, ()>,
    on_close: Callback<(), ()>,
) -> impl IntoView {
    let query_sig = Signal::derive(move || query.clone());
    let results_sig = Signal::derive(move || results.clone());

    view! {
        <Show when={move || is_open}>
            <div
                class="search-modal-overlay"
                on:click={move |_| on_close.call(())}
                role="dialog"
                aria-modal="true"
                aria-labelledby="search-modal-title"
            >
                <div class="search-modal" on:click={move |ev| ev.stop_propagation()}>
                    <div class="search-modal-header">
                        <span class="search-icon">"*</span>
                        <input
                            type="text"
                            class="search-input"
                            placeholder="Search pages, blocks, commands..."
                            value={query_sig.get()}
                            aria-label="Search query"
                        />
                        <span class="search-shortcut">ESC</span>
                    </div>
                    <h2 id="search-modal-title" class="visually-hidden">"Search"</h2>
                    <Show when={move || results_sig.get().is_empty()}>
                        <div class="search-empty">
                            <p>"No results found"</p>
                        </div>
                    </Show>
                    <div class="search-results" role="listbox" aria-label="Search results">
                        <For each={move || results_sig.get()} key=|result| result.id.clone() let:result>
                            <button
                                class="search-result-item"
                                role="option"
                                aria-selected="false"
                            >
                                <span class="search-result-icon">{result.icon}</span>
                                <div class="search-result-content">
                                    <span class="search-result-title">{result.title}</span>
                                    <span class="search-result-preview">{result.preview}</span>
                                </div>
                            </button>
                        </For>
                    </div>
                    <div class="search-modal-footer">
                        <span class="search-hint">
                            <kbd>up/down</kbd> to navigate
                            <kbd>enter</kbd> to select
                            <kbd>esc</kbd> to close
                        </span>
                    </div>
                </div>
            </div>
        </Show>
    }
}
