//! Query view — query builder interface with DSL support
//!
//! This module provides a full-featured query interface including:
//! - DSL query input with syntax highlighting and validation
//! - Auto-complete for common query patterns
//! - Query results in table/tree view with pagination
//! - Query history with saved queries

use crate::bridge::{query_blocks, search_blocks, BlockDto, QueryHistoryItem, SearchResultDto};
use leptos::prelude::*;

/// Query view component
#[component]
pub fn QueryView() -> impl IntoView {
    // Query input state
    let query_input = StoredValue::new(String::new());
    let query_error = StoredValue::new(Option::<String>::None);
    let is_executing = StoredValue::new(false);

    // Results state
    let query_results = StoredValue::new(Vec::<BlockDto>::new());
    let search_results = StoredValue::new(Vec::<SearchResultDto>::new());
    let result_mode = StoredValue::new(ResultMode::Query); // Query or Search
    let current_page = StoredValue::new(0);
    let page_size = 20;

    // History state
    let query_history = StoredValue::new(Vec::<QueryHistoryItem>::new());
    let show_history = StoredValue::new(false);

    // Auto-complete state
    let show_autocomplete = StoredValue::new(false);
    let autocomplete_items = StoredValue::new(Vec::<AutocompleteItem>::new());
    let selected_autocomplete_index = StoredValue::new(0);

    // Load history from localStorage on mount
    load_history_from_storage(query_history);

    // Get results for display
    let get_paginated_results = move || {
        let results = query_results.get_value();
        let page = current_page.get_value();
        let start = page * page_size;
        let end = (start + page_size).min(results.len());
        if start < results.len() {
            results[start..end].to_vec()
        } else {
            vec![]
        }
    };

    let total_pages = move || {
        let total = query_results.get_value().len();
        total.div_ceil(page_size)
    };

    // Execute query action
    let execute_query = Action::new(move |input: &String| {
        let query = input.clone();
        async move {
            is_executing.set_value(true);
            query_error.set_value(None);

            match query_blocks(&query, 100).await {
                Ok(results) => {
                    query_results.set_value(results);
                    result_mode.set_value(ResultMode::Query);
                    current_page.set_value(0);

                    // Add to history
                    add_to_history(&query, query_history);
                }
                Err(e) => {
                    query_error.set_value(Some(e.to_string()));
                }
            }

            is_executing.set_value(false);
        }
    });

    // Execute search action
    let execute_search = Action::new(move |input: &String| {
        let query = input.clone();
        async move {
            is_executing.set_value(true);
            query_error.set_value(None);

            match search_blocks(&query, 50).await {
                Ok(results) => {
                    search_results.set_value(results);
                    result_mode.set_value(ResultMode::Search);
                    current_page.set_value(0);

                    // Add to history
                    add_to_history(&query, query_history);
                }
                Err(e) => {
                    query_error.set_value(Some(e.to_string()));
                }
            }

            is_executing.set_value(false);
        }
    });

    // Handle keyboard shortcuts
    let handle_keydown = move |e: web_sys::KeyboardEvent, input_value: String| {
        // Ctrl+Enter or Cmd+Enter to execute
        if e.key() == "Enter" && (e.ctrl_key() || e.meta_key()) {
            e.prevent_default();
            if !input_value.trim().is_empty() {
                query_input.set_value(input_value.clone());
                // Detect if it's a search or query
                if input_value.trim().starts_with('/') {
                    execute_search.dispatch(input_value.trim_start_matches('/').to_string());
                } else {
                    execute_query.dispatch(input_value);
                }
            }
        }
        // Escape to close autocomplete
        else if e.key() == "Escape" {
            show_autocomplete.set_value(false);
        }
        // Arrow down for autocomplete navigation
        else if e.key() == "ArrowDown" && show_autocomplete.get_value() {
            e.prevent_default();
            let max = autocomplete_items.get_value().len();
            if max > 0 {
                let next = (selected_autocomplete_index.get_value() + 1) % max;
                selected_autocomplete_index.set_value(next);
            }
        }
        // Arrow up for autocomplete navigation
        else if e.key() == "ArrowUp" && show_autocomplete.get_value() {
            e.prevent_default();
            let max = autocomplete_items.get_value().len();
            if max > 0 {
                let prev = if selected_autocomplete_index.get_value() == 0 {
                    max - 1
                } else {
                    selected_autocomplete_index.get_value() - 1
                };
                selected_autocomplete_index.set_value(prev);
            }
        }
        // Tab or Enter to select autocomplete
        else if (e.key() == "Tab" || e.key() == "Enter") && show_autocomplete.get_value() {
            e.prevent_default();
            let items = autocomplete_items.get_value();
            let idx = selected_autocomplete_index.get_value();
            if !items.is_empty() && idx < items.len() {
                let item = &items[idx];
                query_input.set_value(item.insert_text.clone());
                show_autocomplete.set_value(false);
            }
        }
    };

    // Update autocomplete on input
    let update_autocomplete = move |input: String| {
        if input.is_empty() {
            show_autocomplete.set_value(false);
            return;
        }

        let items = get_autocomplete_suggestions(&input);
        let has_items = !items.is_empty();
        autocomplete_items.set_value(items);
        selected_autocomplete_index.set_value(0);
        show_autocomplete.set_value(has_items);
    };

    let is_loading = move || is_executing.get_value();
    let has_error = move || query_error.get_value().is_some();
    let get_error = move || query_error.get_value().clone();
    let mode = move || result_mode.get_value();
    let search_result_count = move || search_results.get_value().len();
    let query_result_count = move || query_results.get_value().len();

    view! {
        <div class="query-view">
            <div class="page-header">
                <h2>"Query"</h2>
                <p class="page-subtitle">"Query your knowledge graph with QuiltQL"</p>
            </div>

            {/* Query Input Section */}
            <div class="card query-input-section">
                <div class="query-input-header">
                    <span class="query-mode-indicator">
                        {move || match mode() {
                            ResultMode::Query => "Query Mode",
                            ResultMode::Search => "Search Mode",
                        }}
                    </span>
                    <button
                        class="btn btn-ghost btn-sm"
                        on:click={move |_| show_history.set_value(!show_history.get_value())}
                    >
                        {move || if show_history.get_value() { "Hide History" } else { "Show History" }}
                    </button>
                </div>

                {/* History Panel */}
                <Show when={move || show_history.get_value()}>
                    <div class="history-panel">
                        <div class="history-section">
                            <h4>"Recent Queries"</h4>
                            <div class="history-list">
                                <Show when={move || !query_history.get_value().is_empty()}>
                                    <For each={move || query_history.get_value().iter().take(10).cloned().collect::<Vec<_>>()}
                                        key=|item| item.timestamp
                                        let:item
                                    >
                                        <button
                                            class="history-item"
                                            on:click={move |_| {
                                                query_input.set_value(item.query.clone());
                                                show_history.set_value(false);
                                            }}
                                        >
                                            <span class="history-query">{item.query.clone()}</span>
                                            <span class="history-time">{item.human_time.clone()}</span>
                                        </button>
                                    </For>
                                </Show>
                                <Show when={move || query_history.get_value().is_empty()}>
                                    <p class="empty-hint">"No recent queries"</p>
                                </Show>
                            </div>
                        </div>
                    </div>
                </Show>

                {/* Query Input */}
                <div class="query-input-wrapper">
                    <div class="query-input-container">
                        <input
                            type="text"
                            class="query-input"
                            placeholder="Enter query: (task todo) or /search term..."
                            attr:data-testid="query-input"
                            value={query_input.get_value()}
                            on:input={move |e: web_sys::Event| {
                                let target = event_target::<web_sys::HtmlInputElement>(&e);
                                let value = target.value();
                                query_input.set_value(value.clone());
                                update_autocomplete(value);
                            }}
                            on:keydown={move |e: web_sys::KeyboardEvent| {
                                let target = event_target::<web_sys::HtmlInputElement>(&e);
                                handle_keydown(e, target.value());
                            }}
                        />

                        {/* Autocomplete dropdown */}
                        <Show when={move || show_autocomplete.get_value()}>
                            <div class="autocomplete-dropdown">
                                <For each={move || autocomplete_items.get_value()}
                                    key=|item| item.label.clone()
                                    let:item
                                >
                                    <button
                                        class="autocomplete-item"
                                        class:selected={move || selected_autocomplete_index.get_value() == 0}
                                        on:click={move |_| {
                                            query_input.set_value(item.insert_text.clone());
                                            show_autocomplete.set_value(false);
                                        }}
                                    >
                                        <span class="autocomplete-label">{item.label.clone()}</span>
                                        <span class="autocomplete-category">{item.category.clone()}</span>
                                    </button>
                                </For>
                            </div>
                        </Show>
                    </div>

                    <button
                        class="btn btn-primary"
                        attr:data-testid="run-query-button"
                        on:click={move |_| {
                            let value = query_input.get_value();
                            if !value.trim().is_empty() {
                                if value.trim().starts_with('/') {
                                    execute_search
                                        .dispatch(value.trim_start_matches('/').to_string());
                                } else {
                                    execute_query.dispatch(value);
                                }
                            }
                        }}
                        disabled={is_loading()}
                    >
                        {move || if is_loading() { "Executing..." } else { "Run Query" }}
                    </button>
                </div>

                {/* Query syntax help */}
                <details class="query-help">
                    <summary>"Query Syntax Help"</summary>
                    <div class="query-help-content">
                        <div class="help-section">
                            <h5>"Basic Queries"</h5>
                            <code>"(task todo)"</code> - Tasks with marker<br/>
                            <code>"(priority a)"</code> - Priority A items<br/>
                            <code>"[[Page Name]]"</code> - References to a page<br/>
                            <code>"(page \"Name\")"</code> - Blocks on a page<br/>
                            <code>"(tags \"tag\")"</code> - Blocks with tag<br/>
                        </div>
                        <div class="help-section">
                            <h5>"Combined Queries"</h5>
                            <code>"(and (task todo) (priority a))"</code> - Both conditions<br/>
                            <code>"(or (task todo) (task done))"</code> - Either condition<br/>
                            <code>"(not (task done))"</code> - Exclude matches<br/>
                        </div>
                        <div class="help-section">
                            <h5>"Full-Text Search"</h5>
                            <code>"/search term"</code> - Search blocks<br/>
                            <code>"/\"exact phrase\""</code> - Exact phrase search<br/>
                        </div>
                        <div class="help-section">
                            <h5>"Modifiers"</h5>
                            <code>"(sample 10)"</code> - Random 10 results<br/>
                            <code>"(sort-by created_at desc)"</code> - Sort results<br/>
                        </div>
                    </div>
                </details>

                {/* Error display */}
                <Show when={has_error}>
                    <div class="query-error">
                        <span class="error-icon">"!"</span>
                        <span class="error-message">{get_error().unwrap_or_default()}</span>
                    </div>
                </Show>
            </div>

            {/* Results Section */}
            <Show when={move || !query_results.get_value().is_empty() || !search_results.get_value().is_empty()}>
                <div class="query-results">
                    <div class="results-header">
                        <h3>
                            {move || match mode() {
                                ResultMode::Query => format!("{} Query Results", query_result_count()),
                                ResultMode::Search => format!("{} Search Results", search_result_count()),
                            }}
                        </h3>

                        {/* Pagination */}
                        <Show when={move || total_pages() > 1}>
                            <div class="pagination">
                                <button
                                    class="btn btn-ghost btn-sm"
                                    disabled={current_page.get_value() == 0}
                                    on:click={move |_| {
                                        current_page.set_value(current_page.get_value().saturating_sub(1))
                                    }}
                                >
                                    "Previous"
                                </button>
                                <span class="page-info">
                                    {move || format!("Page {} of {}", current_page.get_value() + 1, total_pages())}
                                </span>
                                <button
                                    class="btn btn-ghost btn-sm"
                                    disabled={current_page.get_value() >= total_pages() - 1}
                                    on:click={move |_| {
                                        current_page.set_value(current_page.get_value() + 1)
                                    }}
                                >
                                    "Next"
                                </button>
                            </div>
                        </Show>
                    </div>

                    {/* Query Results Table */}
                    <Show when={move || mode() == ResultMode::Query}>
                        <div class="results-table-container">
                            <table class="results-table">
                                <thead>
                                    <tr>
                                        <th>"Page"</th>
                                        <th>"Content"</th>
                                        <th>"Marker"</th>
                                        <th>"Priority"</th>
                                        <th>"Created"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    <For each={move || get_paginated_results()} key=|block| block.id.clone() let:block>
                                        <tr class="result-row">
                                            <td class="result-page">{block.page_name.clone().unwrap_or_default()}</td>
                                            <td class="result-content">{block.content.clone()}</td>
                                            <td class="result-marker">
                                                <span class="marker-badge">
                                                    {block.marker.clone().unwrap_or_default()}
                                                </span>
                                            </td>
                                            <td class="result-priority">
                                                <span class="priority-badge">
                                                    {block.priority.clone().unwrap_or_default().to_uppercase()}
                                                </span>
                                            </td>
                                            <td class="result-date">{format_date(&block.created_at)}</td>
                                        </tr>
                                    </For>
                                </tbody>
                            </table>
                        </div>
                    </Show>

                    {/* Search Results with Snippets */}
                    <Show when={move || mode() == ResultMode::Search}>
                        <div class="search-results-list">
                            <For each={move || search_results.get_value()} key=|r| r.block_id.clone() let:r>
                                <div class="search-result-card">
                                    <div class="search-result-header">
                                        <span class="result-page">{r.page_name.clone()}</span>
                                        <span class="result-score" title="Relevance score">
                                            {format!("{:.2}", r.score)}
                                        </span>
                                    </div>
                                    <div class="search-result-snippet">{r.snippet.clone()}</div>
                                    <div class="search-result-content">{r.content.clone()}</div>
                                </div>
                            </For>
                        </div>
                    </Show>
                </div>
            </Show>

            {/* Empty state when no results */}
            <Show when={move || {
                !is_loading()
                && query_results.get_value().is_empty()
                && search_results.get_value().is_empty()
                && !query_input.get_value().is_empty()
                && !has_error()
            }}>
                <div class="card">
                    <p class="empty-state">"Run a query to see results"</p>
                </div>
            </Show>

            {/* Initial empty state */}
            <Show when={move || query_input.get_value().is_empty() && !is_loading()}>
                <div class="card query-suggestions">
                    <h4>"Quick Start"</h4>
                    <div class="suggestion-chips">
                        <button
                            class="chip"
                            attr:data-testid="query-chip-task-todo"
                            on:click={move |_| {
                                query_input.set_value("(task todo)".to_string());
                            }}
                        >
                            "(task todo)"
                        </button>
                        <button
                            class="chip"
                            attr:data-testid="query-chip-priority-a"
                            on:click={move |_| {
                                query_input.set_value("(priority a)".to_string());
                            }}
                        >
                            "(priority a)"
                        </button>
                        <button
                            class="chip"
                            attr:data-testid="query-chip-combined"
                            on:click={move |_| {
                                query_input.set_value("(and (task todo) (priority a))".to_string());
                            }}
                        >
                            "(and (task todo) (priority a))"
                        </button>
                        <button
                            class="chip"
                            attr:data-testid="query-chip-search"
                            on:click={move |_| {
                                query_input.set_value("/rust".to_string());
                            }}
                        >
                            "/search term"
                        </button>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Result display mode
#[derive(Debug, Clone, Copy, PartialEq)]
enum ResultMode {
    Query,
    Search,
}

/// Autocomplete suggestion item
#[derive(Debug, Clone)]
struct AutocompleteItem {
    label: String,
    insert_text: String,
    category: String,
}

/// Get autocomplete suggestions based on input
fn get_autocomplete_suggestions(input: &str) -> Vec<AutocompleteItem> {
    let input_lower = input.to_lowercase();
    let suggestions = vec![
        ("(task todo)", "Filter unfinished tasks", "Task"),
        ("(task done)", "Filter completed tasks", "Task"),
        ("(task later)", "Filter deferred tasks", "Task"),
        ("(task now)", "Filter current tasks", "Task"),
        ("(task cancelled)", "Filter cancelled tasks", "Task"),
        ("(priority a)", "Filter priority A", "Priority"),
        ("(priority b)", "Filter priority B", "Priority"),
        ("(priority c)", "Filter priority C", "Priority"),
        ("(page \"\")", "Filter by page name", "Page"),
        ("(tags \"\")", "Filter by tag", "Tags"),
        (
            "(and (task todo) (priority a))",
            "Unfinished priority A tasks",
            "Combined",
        ),
        ("(or (task todo) (task done))", "All tasks", "Combined"),
        ("(not (task done))", "Exclude completed", "Combined"),
        ("(sample 10)", "Random 10 results", "Modifier"),
        ("(sort-by created_at desc)", "Sort by date", "Modifier"),
        ("(full-text-search \"\")", "Full-text search", "Search"),
        ("[[", "Page reference", "Reference"),
    ];

    suggestions
        .into_iter()
        .filter(|(pattern, _, _)| {
            pattern.to_lowercase().contains(&input_lower) || input_lower.is_empty()
        })
        .map(|(pattern, description, category)| AutocompleteItem {
            label: format!("{} - {}", pattern, description),
            insert_text: pattern.to_string(),
            category: category.to_string(),
        })
        .collect()
}

/// Format date string for display
fn format_date(date_str: &str) -> String {
    // Simple formatting - just show the date part
    date_str.split('T').next().unwrap_or(date_str).to_string()
}

/// Add a query to history
fn add_to_history(query: &str, history: StoredValue<Vec<QueryHistoryItem>>) {
    let mut h = history.get_value();

    // Remove duplicate if exists
    h.retain(|item| item.query != query);

    // Add new item at the beginning
    h.insert(
        0,
        QueryHistoryItem {
            query: query.to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            human_time: chrono::Utc::now().format("%H:%M").to_string(),
        },
    );

    // Keep only last 50
    h.truncate(50);

    history.set_value(h.clone());

    // Persist to localStorage
    save_history_to_storage(&h);
}

/// Load history from localStorage
fn load_history_from_storage(history: StoredValue<Vec<QueryHistoryItem>>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let window = web_sys::window().expect("no global window");
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(value)) = storage.get_item("quilt_query_history") {
                if let Ok(items) = serde_json::from_str::<Vec<QueryHistoryItem>>(&value) {
                    history.set_value(items);
                }
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = history;
    }
}

/// Save history to localStorage
fn save_history_to_storage(history: &[QueryHistoryItem]) {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window().expect("no global window");
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(value) = serde_json::to_string(history) {
                let _ = storage.set_item("quilt_query_history", &value);
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = history;
    }
}
