use crate::bridge::{self, BlockDto};
use crate::components::block::Block;
use crate::components::loading::Loading;
use chrono::{Duration, NaiveDate, Utc};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use web_sys::KeyboardEvent;

#[component]
pub fn JournalView() -> impl IntoView {
    let params = use_params_map();
    let navigate = use_navigate();

    let today = Utc::now().date_naive();

    // Derive the current date from the route param, default to today.
    let current_date = Signal::derive(move || {
        let ds = params
            .get()
            .get("date")
            .map(|s| s.to_string())
            .unwrap_or_default();
        if ds.is_empty() {
            today
        } else {
            NaiveDate::parse_from_str(&ds, "%Y-%m-%d").unwrap_or(today)
        }
    });

    let (blocks, set_blocks) = signal(Vec::<BlockDto>::new());
    let (loading, set_loading) = signal(true);

    // Fetch blocks whenever the route date changes.
    Effect::new(move || {
        let date = current_date.get();
        let date_str = date.format("%Y-%m-%d").to_string();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            match bridge::get_page_blocks(&date_str).await {
                Ok(b) => set_blocks.set(b),
                Err(_) => set_blocks.set(vec![]),
            }
            set_loading.set(false);
        });
    });

    // ── Helper: navigate to a journal date ──
    let go_to = {
        let navigate = navigate.clone();
        move |date: NaiveDate| {
            navigate(
                &format!("/journal/{}", date.format("%Y-%m-%d")),
                Default::default(),
            );
        }
    };

    // ── Date navigation callbacks ──
    let go_prev = {
        let go_to = go_to.clone();
        move |_| go_to(current_date.get() - Duration::days(1))
    };
    let go_next = {
        let go_to = go_to.clone();
        move |_| go_to(current_date.get() + Duration::days(1))
    };
    let go_today = {
        let go_to = go_to.clone();
        move |_| go_to(today)
    };

    // ── g-prefix keyboard shortcuts ──
    let g_pending = RwSignal::new(false);
    let g_timestamp = RwSignal::new(0.0_f64);
    let on_journal_keydown = move |ev: KeyboardEvent| {
        let meta = ev.meta_key() || ev.ctrl_key();
        let alt = ev.alt_key();
        let shift = ev.shift_key();

        // Check for g-prefix second key
        if g_pending.get_untracked() {
            let now = js_sys::Date::now();
            let elapsed = now - g_timestamp.get_untracked();
            g_pending.set(false);

            if elapsed < 1000.0 && !meta && !alt && !shift {
                let target = match ev.key().as_str() {
                    "j" => Some(today),
                    "t" => Some(today + Duration::days(1)),
                    "n" => Some(current_date.get() + Duration::days(1)),
                    "p" => Some(current_date.get() - Duration::days(1)),
                    _ => None,
                };
                if let Some(d) = target {
                    ev.prevent_default();
                    go_to(d);
                    return;
                }
            }
        }

        // Start g-prefix
        if !meta && !alt && !shift && ev.key() == "g" {
            let now = js_sys::Date::now();
            g_pending.set(true);
            g_timestamp.set(now);
            ev.prevent_default();
        }
    };

    view! {
        <div
            class="journal-view"
            tabindex="0"
            on:keydown=on_journal_keydown
        >
            // ── Header with date navigation ──
            <div class="flex items-center gap-4 mb-6">
                <button
                    class="text-text-muted hover:text-text p-1"
                    on:click=go_prev
                    title="Previous day (g p)"
                >
                    "<"
                </button>
                <h1 class="text-2xl font-bold flex-1">
                    {move || format_date(&current_date.get())}
                </h1>
                <button
                    class="text-sm text-text-muted hover:text-text px-2 py-0.5 rounded border border-border"
                    on:click=go_today
                    title="Today (g j)"
                >
                    "Today"
                </button>
                <button
                    class="text-text-muted hover:text-text p-1"
                    on:click=go_next
                    title="Next day (g n)"
                >
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
                            "No notes yet. Start writing..."
                        </div>
                    </Show>
                }
            >
                <div class="outliner">
                    <For
                        each=move || blocks.get()
                        key=|b| b.id.clone()
                        let:block
                    >
                        <Block
                            block=Signal::derive(move || block.clone())
                            blocks=blocks
                            set_blocks=set_blocks
                            children=vec![]
                        />
                    </For>
                </div>
            </Show>
        </div>
    }
}

/// Format a NaiveDate as a human-readable date string.
/// Example: "May 28, 2026"
fn format_date(date: &NaiveDate) -> String {
    date.format("%B %-d, %Y").to_string()
}
