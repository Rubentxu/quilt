//! Pages view — page listing with Logseq-style

use crate::bridge::{list_pages, PageDto};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

/// Pages listing view - Logseq style
#[component]
pub fn PagesView() -> impl IntoView {
    let fetch_pages = Action::new_local(|_: &()| async move {
        match list_pages().await {
            Ok(pages) => Some(pages),
            Err(e) => {
                log::warn!("Failed to load pages: {}", e);
                None
            }
        }
    });

    // Trigger initial fetch
    fetch_pages.dispatch(());

    view! {
        <div class="pages-view">
            <header class="pages-header">
                <h1 class="pages-title">"Pages"</h1>
                <p class="pages-subtitle">"All your pages"</p>
            </header>

            <Show when={move || fetch_pages.pending().get()} fallback={move || {
                view! {
                    <PagesList pages={fetch_pages.value().get().unwrap_or(None).unwrap_or_default()} />
                }
            }}>
                <div class="loading">"Loading pages..."</div>
            </Show>
        </div>
    }
}

/// Separate component for the actual pages list
#[component]
fn PagesList(pages: Vec<PageDto>) -> impl IntoView {
    let navigate = use_navigate();

    view! {
        <ul class="pages-list">
            {pages.iter().map(|p| {
                let page_id = p.id.clone();
                let title = p.title.clone().unwrap_or(p.name.clone());
                let icon = if p.journal { "📅" } else { "📄" };
                let nav = navigate.clone();
                view! {
                    <li>
                        <button
                            class="page-row"
                            on:click={move |_| {
                                let target = format!("/pages/{}", page_id);
                                nav(&target, Default::default());
                            }}
                        >
                            <span class="page-row-icon">{icon}</span>
                            <span class="page-row-title">{title}</span>
                        </button>
                    </li>
                }
            }).collect::<Vec<_>>()}
        </ul>
    }
}
