use crate::bridge::{self, SearchResultDto};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn SearchView() -> impl IntoView {
    let (query, set_query) = signal(String::new());
    let (results, set_results) = signal(Vec::<SearchResultDto>::new());
    let (searching, set_searching) = signal(false);

    Effect::new(move || {
        let q = query.get();
        if q.is_empty() {
            set_results.set(vec![]);
            return;
        }
        if q.len() < 2 {
            return;
        }
        set_searching.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            match bridge::search(&q).await {
                Ok(r) => set_results.set(r),
                Err(_) => set_results.set(vec![]),
            }
            set_searching.set(false);
        });
    });

    view! {
        <div class="search-view">
            <h1 class="text-2xl font-bold mb-6">"Search"</h1>

            <div class="relative mb-6">
                <input
                    type="text"
                    class="w-full px-4 py-3 bg-surface border border-border rounded-lg text-text placeholder-text-muted focus:outline-none focus:border-accent"
                    placeholder="Search pages and blocks..."
                    prop:value=move || query.get()
                    on:input=move |ev| set_query.set(event_target_value(&ev))
                />
            </div>

            <Show when=move || searching.get()>
                <div class="text-text-muted text-sm">"Searching..."</div>
            </Show>

            <Show
                when=move || !searching.get() && !results.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !query.get().is_empty() && !searching.get()>
                        <div class="text-text-muted text-sm">"No results found"</div>
                    </Show>
                }
            >
                <div class="space-y-2">
                    <For each=move || results.get() key=|r| r.block_id.clone() let:result>
                        <SearchResult result=result />
                    </For>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn SearchResult(result: SearchResultDto) -> impl IntoView {
    let href = format!("/page/{}", result.page_name);
    let name = result.page_name.clone();
    let snippet = result
        .snippet
        .clone()
        .unwrap_or_else(|| result.content.clone());
    view! {
        <A href=href>
            <div class="block p-3 rounded hover:bg-surface-hover border border-border transition-colors">
                <div class="text-xs text-text-muted mb-1">{name}</div>
                <div class="text-sm">{snippet}</div>
            </div>
        </A>
    }
}
