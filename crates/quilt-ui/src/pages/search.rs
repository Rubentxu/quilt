//! Search view — search interface

use crate::bridge::search_blocks;
use leptos::prelude::*;

/// Search interface view
#[component]
pub fn SearchView() -> impl IntoView {
    let search_query = StoredValue::new(String::new());

    // Action to search blocks
    let perform_search = Action::new_local(move |query: &String| {
        let q = query.clone();
        async move {
            if q.trim().is_empty() {
                vec![]
            } else {
                match search_blocks(&q, 50).await {
                    Ok(results) => results,
                    Err(e) => {
                        log::warn!("Search failed: {}", e);
                        vec![]
                    }
                }
            }
        }
    });

    // Derived state
    let is_loading = move || perform_search.pending().get();
    let get_results = move || perform_search.value().get().unwrap_or_default();

    view! {
        <div class="search-view">
            <div class="page-header">
                <h2>"Search"</h2>
                <p class="page-subtitle">"Search your knowledge graph"</p>
            </div>

            <div class="card" style="margin-bottom: 1rem">
                <input
                    type="text"
                    placeholder="Search blocks..."
                    attr:data-testid="search-input"
                    on:keypress={move |e: web_sys::KeyboardEvent| {
                        if e.key() == "Enter" {
                            let target = event_target::<web_sys::HtmlInputElement>(&e);
                            let value = target.value();
                            if !value.trim().is_empty() {
                                search_query.set_value(value.clone());
                                perform_search.dispatch(value);
                            }
                        }
                    }}
                />
            </div>

            <Show when={is_loading} fallback={move || {
                view! {
                    <Show when={move || !search_query.get_value().trim().is_empty()} fallback={move || view! {
                        <div class="card">
                            <p class="empty-state">"Enter a search term to find blocks"</p>
                        </div>
                    }}>
                        <Show when={move || !get_results().is_empty()} fallback={move || view! {
                            <div class="card">
                                <p class="empty-state">"No results found"</p>
                            </div>
                        }}>
                            <div class="search-results">
                                <p class="results-count">{format!("{} result(s) found", get_results().len())}</p>
                                {get_results().iter().map(|r| view! {
                                    <div class="card" style="margin-bottom: 0.5rem">
                                        <div class="search-result-item">
                                            <div class="result-meta">
                                                <span class="result-page">{r.page_name.clone()}</span>
                                                <span class="result-block-id">{"#".to_string() + &r.block_id}</span>
                                            </div>
                                            <div class="result-snippet">
                                                {r.snippet.as_deref().unwrap_or(&r.content).to_string()}
                                            </div>
                                        </div>
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                        </Show>
                    </Show>
                }
            }}>
                <div class="loading">"Searching..."</div>
            </Show>
        </div>
    }
}
