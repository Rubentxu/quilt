//! Journal view — today's daily notes

use crate::bridge::get_journal;
use chrono::Local;
use leptos::prelude::*;

/// Journal page showing today's notes and tasks
#[component]
pub fn JournalView() -> impl IntoView {
    let today_str = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let today_display = Local::now().date_naive().format("%B %d, %Y").to_string();
    let today_for_fetch = today_str.clone();

    // Action to fetch today's journal page
    let fetch_journal = Action::new_local(move |_: &String| {
        let date = today_str.clone();
        async move {
            match get_journal(&date).await {
                Ok(page) => Some(page),
                Err(e) => {
                    log::warn!("Failed to load journal for {}: {}", date, e);
                    None
                }
            }
        }
    });

    // Trigger initial fetch
    fetch_journal.dispatch(today_for_fetch);

    view! {
        <div class="journal-view">
            <div class="page-header">
                <h2 class="journal-date">{today_display}</h2>
                <p class="page-subtitle">"Your daily journal"</p>
            </div>

            <Show when={move || fetch_journal.pending().get()} fallback={move || {
                view! {
                    <Show when={move || fetch_journal.value().get().flatten().is_some()} fallback={move || view! {
                        <div>
                            <section class="card" style="margin-bottom: 1.5rem">
                                <h3>"Tasks"</h3>
                                <p class="empty-state">"No tasks yet"</p>
                            </section>
                            <section class="card">
                                <h3>"Notes"</h3>
                                <p class="empty-state">"Start writing to see your notes"</p>
                            </section>
                        </div>
                    }}>
                        <div>
                            <section class="card" style="margin-bottom: 1.5rem">
                                <h3>"Tasks"</h3>
                                <p class="empty-state">"No tasks yet"</p>
                            </section>
                            <section class="card">
                                <h3>"Notes"</h3>
                                <p class="empty-state">"Start writing to see your notes"</p>
                            </section>
                        </div>
                    </Show>
                }
            }}>
                <div class="loading">"Loading journal..."</div>
            </Show>
        </div>
    }
}
