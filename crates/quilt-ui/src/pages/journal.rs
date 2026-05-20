//! Journal view — today's daily notes with Logseq-style layout

use crate::bridge::{self, get_journal, BriefingStatsDto, CognitivePulseDto, MorningBriefingDto};
use crate::components::block::{Block, EmptyState};
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
            <header class="journal-header">
                <h1 class="journal-date">{today_display}</h1>
                <p class="journal-subtitle">"Your daily journal"</p>
            </header>

            // Morning Briefing Section
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

            // Journal content
            <Show when={move || !fetch_journal.pending().get()}>
                <div class="journal-content">
                    <Block title="Tasks">
                        <EmptyState message="No tasks yet" />
                    </Block>

                    <Block title="Notes">
                        <EmptyState message="Start writing to see your notes" />
                    </Block>
                </div>
            </Show>
        </div>
    }
}

/// Morning Briefing Section Component with Logseq styling
#[component]
fn MorningBriefingSection(briefing: MorningBriefingDto) -> impl IntoView {
    let pulse = briefing.cognitive_pulse.clone();
    let highlights_count = briefing.serendipity_highlights.len();
    let alerts_count = briefing.decay_alerts.len();

    view! {
        <Block title="🧠 Morning Briefing">
            // Compact inline metrics - Logseq style
            <p class="briefing-metrics">
                {pulse.total_pages} " Pages · " {pulse.total_blocks} " Blocks · " {pulse.clusters} " Clusters · " {pulse.frontiers} " Frontiers"
            </p>

            <Show when={move || highlights_count > 0}>
                <ul class="briefing-list">
                    <li class="briefing-section-title">"✨ Serendipity"</li>
                    {briefing.serendipity_highlights.iter().take(3).map(|h| {
                        let confidence_pct = (h.confidence * 100.0).round() as i32;
                        view! {
                            <li class="briefing-list-item">
                                <span class="highlight-from">{h.from_page.clone()}</span>
                                <span class="highlight-arrow">" → "</span>
                                <span class="highlight-to">{h.to_page.clone()}</span>
                                <span class="highlight-confidence">" (" {confidence_pct} "%)"</span>
                            </li>
                        }
                    }).collect::<Vec<_>>()}
                </ul>
            </Show>

            <Show when={move || alerts_count > 0}>
                <ul class="briefing-list briefing-list-alerts">
                    <li class="briefing-section-title">"⚠️ Stale Pages"</li>
                    {briefing.decay_alerts.iter().take(3).map(|a| {
                        view! {
                            <li class="briefing-list-item">
                                <span class="stale-page-name">{a.page_name.clone()}</span>
                                <span class="stale-days">" — " {a.days_stale} " days stale"</span>
                            </li>
                        }
                    }).collect::<Vec<_>>()}
                </ul>
            </Show>

            <Show when={move || briefing.degraded}>
                <div class="degraded-notice">"Running in degraded mode"</div>
            </Show>
        </Block>
    }
}
