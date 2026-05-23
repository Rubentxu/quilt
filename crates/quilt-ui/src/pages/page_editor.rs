//! Page editor — edit blocks in a single page

use crate::bridge::get_page;
use crate::components::outliner_tree::OutlinerTree;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use serde::Deserialize;

#[derive(Params, Debug, PartialEq, Eq, Deserialize)]
pub struct PageParams {
    pub id: Option<String>,
}

#[component]
pub fn PageEditor() -> impl IntoView {
    let params = use_params::<PageParams>();
    let page_id = params.with(|p| {
        p.as_ref()
            .ok()
            .and_then(|p| p.id.clone())
            .unwrap_or_default()
    });

    let navigate = use_navigate();

    // Action to fetch page data with blocks
    let fetch_page = Action::new_local(move |id: &String| {
        let id = id.clone();
        async move {
            log::info!("Fetching page: {}", id);
            get_page(&id).await.ok()
        }
    });

    // Clone page_id for use in Effect
    let page_id_for_effect = page_id.clone();

    // Trigger initial fetch when page_id is available
    Effect::new(move |_| {
        if !page_id_for_effect.is_empty() {
            fetch_page.dispatch(page_id_for_effect.clone());
        }
    });

    view! {
        <div class="page-editor">
            <header class="page-editor-header">
                <button
                    class="page-editor-back"
                    on:click={move |_| {
                        navigate("/pages", Default::default());
                    }}
                >
                    "← Pages"
                </button>
                <h1 class="page-editor-title">
                    {move || {
                        fetch_page
                            .value()
                            .get()
                            .flatten()
                            .map(|p| p.title.clone().unwrap_or_else(|| format!("Page {}", page_id.clone())))
                            .unwrap_or_else(|| format!("Page {}", page_id.clone()))
                    }}
                </h1>
            </header>

            <main class="page-editor-main">
                <Show when={move || !fetch_page.pending().get()} fallback={move || view! { <div class="loading">"Loading..."</div> }}>
                    <Show
                        when={move || fetch_page.value().get().flatten().is_some()}
                        fallback={move || view! { <div class="error">"Failed to load page"</div> }}
                    >
                        <OutlinerTree
                            blocks={fetch_page.value().get().flatten().unwrap().blocks.clone()}
                            viewport_height={800.0}
                            item_height={50.0}
                        />
                    </Show>
                </Show>
            </main>
        </div>
    }
}
