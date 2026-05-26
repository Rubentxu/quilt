//! Backlinks panel — shows pages/blocks that reference the current page.
//!
//! Displays each backlink's source page name as a clickable link
//! and a content preview of the referencing block.

use crate::bridge::BacklinkDto;
use leptos::prelude::*;
use leptos_router::components::A;

/// Panel that renders backlinks for the current page.
///
/// Supports three states:
/// - **Loading**: Shows "Loading..." while backlinks are being fetched.
/// - **Empty**: Shows "No backlinks" when the list is empty.
/// - **Data**: Renders each backlink with a linked page name and content preview.
#[component]
pub fn BacklinksPanel(
    /// Reactive list of backlinks to display.
    #[prop(into)]
    backlinks: Signal<Vec<BacklinkDto>>,
    /// Whether backlinks are still being fetched.
    #[prop(into)]
    loading: Signal<bool>,
) -> impl IntoView {
    view! {
        <div>
            // ── Loading state ──
            <Show when=move || loading.get()>
                <div class="text-xs text-text-muted py-2">"Loading..."</div>
            </Show>

            // ── Empty state ──
            <Show when=move || !loading.get() && backlinks.get().is_empty()>
                <div class="text-xs text-text-muted py-2">"No backlinks"</div>
            </Show>

            // ── Data state ──
            <Show when=move || !loading.get() && !backlinks.get().is_empty()>
                <div class="space-y-2">
                    <For
                        each=move || backlinks.get()
                        key=|bl| bl.source_block_id.clone()
                        let:bl
                    >
                        <div class="backlink-item border-l-2 border-border pl-3 py-1.5">
                            <A href=format!("/page/{}", bl.source_page_name)>
                                <span class="text-sm text-accent hover:underline">
                                    {bl.source_page_name.clone()}
                                </span>
                            </A>
                            <div class="text-xs text-text-muted mt-0.5 line-clamp-2">
                                {bl.content_preview.clone()}
                            </div>
                        </div>
                    </For>
                </div>
            </Show>
        </div>
    }
}
