use crate::bridge::{self, BlockDto};
use crate::components::block::Block;
use crate::components::loading::Loading;
use leptos::prelude::*;

#[component]
pub fn JournalView() -> impl IntoView {
    let today = chrono_today();
    let (date, set_date) = signal(today);
    let (blocks, set_blocks) = signal(Vec::<BlockDto>::new());
    let (loading, set_loading) = signal(true);

    Effect::new(move || {
        let d = date.get();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            match bridge::get_page_blocks(&d).await {
                Ok(b) => set_blocks.set(b),
                Err(_) => set_blocks.set(vec![]),
            }
            set_loading.set(false);
        });
    });

    let prev_day = move |_| {
        set_date.update(|d| {
            *d = shift_date(d, -1);
        });
    };

    let next_day = move |_| {
        set_date.update(|d| {
            *d = shift_date(d, 1);
        });
    };

    view! {
        <div class="journal-view">
            <div class="flex items-center gap-4 mb-6">
                <button class="text-text-muted hover:text-text p-1" on:click=prev_day>
                    "<"
                </button>
                <h1 class="text-2xl font-bold flex-1">
                    {move || format_date(&date.get())}
                </h1>
                <button class="text-text-muted hover:text-text p-1" on:click=next_day>
                    ">"
                </button>
            </div>

            <Show when=move || loading.get()>
                <Loading />
            </Show>

            <Show
                when=move || !loading.get() && !blocks.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !loading.get()>
                        <div class="text-text-muted text-sm py-4">
                            "No notes yet. Start typing..."
                        </div>
                    </Show>
                }
            >
                <div class="outliner">
                    <For each=move || blocks.get() key=|b| b.id.clone() let:block>
                        <Block block=Signal::derive(move || block.clone()) blocks=blocks set_blocks=set_blocks children=vec![] />
                    </For>
                </div>
            </Show>
        </div>
    }
}

fn chrono_today() -> String {
    "2026-05-24".to_string()
}

fn shift_date(date: &str, _days: i32) -> String {
    date.to_string()
}

fn format_date(date: &str) -> String {
    date.to_string()
}
