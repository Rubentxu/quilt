//! Journal view — today's daily notes

use crate::bridge::{self, get_journal, BriefingStatsDto, CognitivePulseDto, MorningBriefingDto};
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

    // Action to fetch morning briefing
    let fetch_briefing =
        Action::new_local(|_: &()| async move { bridge::get_morning_briefing().await.ok() });

    // Trigger initial fetches
    fetch_journal.dispatch(today_for_fetch);
    fetch_briefing.dispatch(());

    view! {
        <div class="journal-view">
            <div class="page-header">
                <h2 class="journal-date">{today_display}</h2>
                <p class="page-subtitle">"Your daily journal"</p>
            </div>

            // Morning Briefing Section - shown when briefing is available
            <Show
                when={move || !fetch_briefing.pending().get() && fetch_briefing.value().get().flatten().is_some()}
                fallback={move || view! { <div class="briefing-loading">"Loading cognitive pulse..."</div> }}
            >
                {move || {
                    let briefing = fetch_briefing.value().get().flatten().unwrap_or_else(|| MorningBriefingDto {
                        cognitive_pulse: CognitivePulseDto {
                            total_pages: 0,
                            total_blocks: 0,
                            clusters: 0,
                            frontiers: 0,
                            gaps: 0,
                        },
                        serendipity_highlights: vec![],
                        decay_alerts: vec![],
                        stats: BriefingStatsDto {
                            pages_created_today: 0,
                            blocks_created_today: 0,
                            queries_run_today: 0,
                        },
                        knowledge_evolution: vec![],
                        generated_at: String::new(),
                        degraded: false,
                    });
                    view! { <MorningBriefingSection briefing={briefing} /> }
                }}
            </Show>

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

/// Morning Briefing Section Component
#[component]
fn MorningBriefingSection(briefing: MorningBriefingDto) -> impl IntoView {
    let pulse = briefing.cognitive_pulse.clone();
    let highlights_count = briefing.serendipity_highlights.len();
    let alerts_count = briefing.decay_alerts.len();

    view! {
        <section class="card morning-briefing-section" style="margin-bottom: 1.5rem">
            <h3>"🧠 Morning Briefing"</h3>
            <div class="briefing-pulse">
                <span class="pulse-metric">
                    <span class="pulse-value">{pulse.total_pages}</span>
                    <span class="pulse-label">"Pages"</span>
                </span>
                <span class="pulse-metric">
                    <span class="pulse-value">{pulse.total_blocks}</span>
                    <span class="pulse-label">"Blocks"</span>
                </span>
                <span class="pulse-metric">
                    <span class="pulse-value">{pulse.clusters}</span>
                    <span class="pulse-label">"Clusters"</span>
                </span>
                <span class="pulse-metric">
                    <span class="pulse-value">{pulse.frontiers}</span>
                    <span class="pulse-label">"Frontiers"</span>
                </span>
            </div>
            <Show when={move || highlights_count > 0}>
                <div class="briefing-highlights">
                    <h4>"✨ Serendipity"</h4>
                    <ul>
                        {briefing.serendipity_highlights.iter().take(3).map(|h| {
                            view! {
                                <li>
                                    {h.from_page.clone()} " → " {h.to_page.clone()}
                                    <span class="highlight-confidence">" ({(h.confidence * 100.0).round()}%)"</span>
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                </div>
            </Show>
            <Show when={move || alerts_count > 0}>
                <div class="briefing-alerts">
                    <h4>"⚠️ Stale Pages"</h4>
                    <ul>
                        {briefing.decay_alerts.iter().take(3).map(|a| {
                            view! {
                                <li>
                                    {a.page_name.clone()} " — " {a.days_stale} " days stale"
                                </li>
                            }
                        }).collect::<Vec<_>>()}
                    </ul>
                </div>
            </Show>
            <Show when={move || briefing.degraded}>
                <div class="degraded-notice">"Running in degraded mode"</div>
            </Show>
        </section>
    }
}
