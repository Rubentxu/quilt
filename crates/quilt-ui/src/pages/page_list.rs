//! Pages view — page listing

use crate::bridge::list_pages;
use leptos::prelude::*;

/// Pages listing view
#[component]
pub fn PagesView() -> impl IntoView {
    // Action to fetch all pages
    let fetch_pages = Action::new(|_: &()| async move {
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

    // Derived state
    let has_pages = move || {
        fetch_pages
            .value()
            .get()
            .flatten()
            .is_some_and(|p| !p.is_empty())
    };
    let is_loading = move || fetch_pages.pending().get();

    view! {
        <div class="pages-view">
            <div class="page-header">
                <h2>"Pages"</h2>
                <p class="page-subtitle">"All your pages"</p>
            </div>

            <Show when={is_loading} fallback={move || {
                view! {
                    <Show when={has_pages} fallback={move || view! {
                        <div class="card">
                            <p class="empty-state">"No pages yet. Create your first page!"</p>
                        </div>
                    }}>
                        <div class="block-list">
                            {fetch_pages.value().get().unwrap_or(None).unwrap_or_default().iter().map(|p| view! {
                                <div class="card" style="margin-bottom: 0.5rem">
                                    <div class="block-item">
                                        <span class="block-bullet">
                                            {if p.journal { "📅" } else { "📄" }}
                                        </span>
                                        <span class="block-content">
                                            {p.title.clone().unwrap_or(p.name.clone())}
                                        </span>
                                    </div>
                                </div>
                            }).collect::<Vec<_>>()}
                        </div>
                    </Show>
                }
            }}>
                <div class="loading">"Loading pages..."</div>
            </Show>
        </div>
    }
}
