//! Pages view — page listing with Logseq-style

use crate::bridge::list_pages;
use leptos::prelude::*;

/// Pages listing view - Logseq style
#[component]
pub fn PagesView() -> impl IntoView {
    // Action to fetch all pages
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
                    <Show when={move || {
                        fetch_pages
                            .value()
                            .get()
                            .flatten()
                            .is_some_and(|p| !p.is_empty())
                    }} fallback={move || view! {
                        <div class="pages-empty">
                            <p>"No pages yet. Create your first page!"</p>
                        </div>
                    }}>
                        <ul class="pages-list">
                            {fetch_pages.value().get().unwrap_or(None).unwrap_or_default().iter().map(|p| {
                                let title = p.title.clone().unwrap_or(p.name.clone());
                                let icon = if p.journal { "📅" } else { "📄" };
                                view! {
                                    <li>
                                        <button class="page-row">
                                            <span class="page-row-icon">{icon}</span>
                                            <span class="page-row-title">{title}</span>
                                        </button>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </Show>
                }
            }}>
                <div class="loading">"Loading pages..."</div>
            </Show>
        </div>
    }
}
