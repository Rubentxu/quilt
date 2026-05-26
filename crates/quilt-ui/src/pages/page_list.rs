use crate::bridge::{self, PageDto};
use crate::components::loading::Loading;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn PagesView() -> impl IntoView {
    let (pages, set_pages) = signal(Vec::<PageDto>::new());
    let (loading, set_loading) = signal(true);

    Effect::new(move || {
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            match bridge::list_pages().await {
                Ok(p) => set_pages.set(p),
                Err(_) => set_pages.set(vec![]),
            }
            set_loading.set(false);
        });
    });

    view! {
        <div class="pages-view">
            <h1 class="text-2xl font-bold mb-6">"All Pages"</h1>

            <Show when=move || loading.get()>
                <Loading />
            </Show>

            <Show
                when=move || !loading.get() && !pages.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !loading.get()>
                        <div class="text-text-muted text-sm py-4">"No pages yet"</div>
                    </Show>
                }
            >
                <div class="space-y-1">
                    <For each=move || pages.get() key=|p| p.id.clone() let:page>
                        <PageListItem page=page />
                    </For>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn PageListItem(page: PageDto) -> impl IntoView {
    let href = format!("/page/{}", page.name);
    let name = page.name.clone();
    let is_journal = page.journal;
    view! {
        <A href=href>
            <div class="flex items-center gap-2 px-3 py-2 rounded hover:bg-surface-hover transition-colors">
                <span class="text-sm">{name}</span>
                {is_journal.then(|| view! {
                    <span class="text-xs bg-accent/20 text-accent px-1.5 py-0.5 rounded">"journal"</span>
                })}
            </div>
        </A>
    }
}
