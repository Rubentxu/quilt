//! Right sidebar — backlinks, references, page context
//!
//! Logseq-like right sidebar that shows:
//! - Backlinks (linked references)
//! - Unlinked references
//! - Can hold multiple items
//! - Resizable (min 320px, max 70% viewport)

use crate::bridge::BacklinkDto;
use crate::components::backlinks_panel::BacklinksPanel;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn RightSidebar(#[prop(into)] open: Signal<bool>) -> impl IntoView {
    let set_open = use_context::<WriteSignal<bool>>().unwrap_or_else(|| {
        let (_, w) = signal(false);
        w
    });

    let backlinks_rw = use_context::<RwSignal<Vec<BacklinkDto>>>();
    let backlinks = Signal::derive(move || backlinks_rw.map(|b| b.get()).unwrap_or_default());

    let backlinks_loading_rw = use_context::<RwSignal<bool>>();
    let backlinks_loading =
        Signal::derive(move || backlinks_loading_rw.map(|b| b.get()).unwrap_or(false));

    let unlinked_rw = use_context::<RwSignal<Vec<BacklinkDto>>>();
    let unlinked_references =
        Signal::derive(move || unlinked_rw.map(|u| u.get()).unwrap_or_default());

    let unlinked_loading_rw = use_context::<RwSignal<bool>>();
    let unlinked_loading =
        Signal::derive(move || unlinked_loading_rw.map(|u| u.get()).unwrap_or(false));

    view! {
        <Show when=move || open.get()>
            <aside class="w-80 min-w-80 border-l border-border bg-sidebar flex flex-col shrink-0 overflow-hidden">
                <div class="p-3 border-b border-border flex items-center justify-between">
                    <h2 class="text-sm font-semibold">"References"</h2>
                    <button
                        class="text-text-muted hover:text-text p-1"
                        on:click=move |_| set_open.set(false)
                    >
                        <span>"✕"</span>
                    </button>
                </div>

                <div class="flex-1 overflow-y-auto p-3">
                    // ── Linked References ──
                    <div class="mb-4">
                        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">
                            "Linked References"
                        </h3>
                        <BacklinksPanel
                            backlinks=backlinks
                            loading=backlinks_loading
                        />
                    </div>

                    // ── Unlinked References ──
                    <div class="mb-4">
                        <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted mb-2">
                            "Unlinked References"
                        </h3>
                        <Show when=move || unlinked_loading.get()>
                            <div class="text-xs text-text-muted py-2">"Loading..."</div>
                        </Show>
                        <Show when=move || !unlinked_loading.get() && unlinked_references.get().is_empty()>
                            <div class="text-xs text-text-muted py-2">"No unlinked references"</div>
                        </Show>
                        <Show when=move || !unlinked_loading.get() && !unlinked_references.get().is_empty()>
                            <div class="space-y-2">
                                <For
                                    each=move || unlinked_references.get()
                                    key=|u| u.source_block_id.clone()
                                    let:u
                                >
                                    <div class="backlink-item border-l-2 border-dashed border-text-muted pl-3 py-1.5">
                                        <A href=format!("/page/{}", u.source_page_name)>
                                            <span class="text-sm text-accent hover:underline">
                                                {u.source_page_name.clone()}
                                            </span>
                                        </A>
                                        <div class="text-xs text-text-muted mt-0.5 line-clamp-2 italic">
                                            {u.content_preview.clone()}
                                        </div>
                                        <span class="text-xs text-text-muted mt-0.5">
                                            "unlinked"
                                        </span>
                                    </div>
                                </For>
                            </div>
                        </Show>
                    </div>
                </div>
            </aside>
        </Show>
    }
}
