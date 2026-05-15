//! Cognitive Dashboard — overview of cognitive state
//!
//! Displays clusters, frontiers, gaps, and influence metrics
//! from the CognitiveMirror via Tauri IPC.

use leptos::callback::Callable;
use leptos::prelude::*;

use crate::bridge::{self, CognitivePulseDto, DecayAlertDto, SerendipityHighlightDto};

// ── Widget Components ─────────────────────────────────────────────────────────

/// CognitivePulse widget — shows clusters, frontiers, gaps counts
#[component]
fn CognitivePulseWidget(pulse: CognitivePulseDto) -> impl IntoView {
    view! {
        <div class="widget cognitive-pulse-widget">
            <h3 class="widget-title">"📊 Cognitive Pulse"</h3>
            <div class="pulse-grid">
                <div class="pulse-metric">
                    <span class="pulse-value">{pulse.total_pages}</span>
                    <span class="pulse-label">"Pages"</span>
                </div>
                <div class="pulse-metric">
                    <span class="pulse-value">{pulse.total_blocks}</span>
                    <span class="pulse-label">"Blocks"</span>
                </div>
                <div class="pulse-metric">
                    <span class="pulse-value">{pulse.clusters}</span>
                    <span class="pulse-label">"Clusters"</span>
                </div>
                <div class="pulse-metric">
                    <span class="pulse-value">{pulse.frontiers}</span>
                    <span class="pulse-label">"Frontiers"</span>
                </div>
                <div class="pulse-metric">
                    <span class="pulse-value">{pulse.gaps}</span>
                    <span class="pulse-label">"Gaps"</span>
                </div>
            </div>
        </div>
    }
}

/// SerendipityHighlights widget — lists recent serendipity discoveries
#[component]
fn SerendipityHighlightsWidget(highlights: Vec<SerendipityHighlightDto>) -> impl IntoView {
    view! {
        <div class="widget serendipity-widget">
            <h3 class="widget-title">"✨ Serendipity Highlights"</h3>
            <ul class="highlights-list">
                {highlights.iter().map(|h| {
                    let from = h.from_page.clone();
                    let to = h.to_page.clone();
                    let ctype = h.connection_type.clone();
                    view! {
                        <li class="highlight-item">
                            <div class="highlight-connection">
                                <span class="highlight-from">{from}</span>
                                <span class="highlight-arrow">" → "</span>
                                <span class="highlight-to">{to}</span>
                            </div>
                            <div class="highlight-meta">
                                <span class="highlight-type">{ctype}</span>
                                <span class="highlight-confidence">
                                    "{(h.confidence * 100.0).round()}%" confidence
                                </span>
                            </div>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

/// DecayAlerts widget — lists stale pages
#[component]
fn DecayAlertsWidget(alerts: Vec<DecayAlertDto>) -> impl IntoView {
    view! {
        <div class="widget decay-widget">
            <h3 class="widget-title">"⚠️ Decay Alerts"</h3>
            <ul class="alerts-list">
                {alerts.iter().map(|a| {
                    let page_name = a.page_name.clone();
                    view! {
                        <li class="alert-item">
                            <span class="alert-page">{page_name}</span>
                            <span class="alert-staleness">
                                {a.days_stale}" days stale"
                            </span>
                        </li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}

// ── Loading Skeleton ──────────────────────────────────────────────────────────

#[component]
fn LoadingSkeleton() -> impl IntoView {
    view! {
        <div class="dashboard-loading">
            <div class="skeleton-header"></div>
            <div class="skeleton-grid">
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
            </div>
            <div class="skeleton-widgets">
                <div class="skeleton-widget"></div>
                <div class="skeleton-widget"></div>
                <div class="skeleton-widget"></div>
            </div>
        </div>
    }
}

// ── Error State ───────────────────────────────────────────────────────────────

#[component]
fn ErrorState(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="dashboard-error">
            <p class="error-message">"Failed to load morning briefing: " {message}</p>
            <button class="btn-retry" on:click={move |_| on_retry.run(())}>
                "🔄 Retry"
            </button>
        </div>
    }
}

// ── Briefing Content View ─────────────────────────────────────────────────────

#[component]
fn BriefingContent(
    briefing: bridge::MorningBriefingDto,
    on_refresh: Callback<()>,
) -> impl IntoView {
    let pulse = briefing.cognitive_pulse.clone();
    let highlights = briefing.serendipity_highlights.clone();
    let alerts = briefing.decay_alerts.clone();
    let stats = briefing.stats.clone();

    view! {
        <div class="briefing-content">
            <Show when={move || briefing.degraded}>
                <div class="degraded-banner">
                    "⚠️ Running in degraded mode — some engines unavailable"
                </div>
            </Show>

            <StatsBar
                pages={stats.pages_created_today}
                blocks={stats.blocks_created_today}
                queries={stats.queries_run_today}
            />

            <CognitivePulseWidget pulse={pulse} />

            <div class="widgets-row">
                <SerendipityHighlightsWidget highlights={highlights} />
                <DecayAlertsWidget alerts={alerts} />
            </div>

            <div class="dashboard-actions">
                <button
                    class="btn-primary"
                    attr:data-testid="refresh-button"
                    on:click={move |_| on_refresh.run(())}
                >
                    "🔄 Refresh"
                </button>
            </div>
        </div>
    }
}

// ── Stats Bar ─────────────────────────────────────────────────────────────────

#[component]
fn StatsBar(pages: usize, blocks: usize, queries: usize) -> impl IntoView {
    view! {
        <div class="stats-bar">
            <span class="stat-item">"📄 " {pages} " pages created today"</span>
            <span class="stat-item">"📝 " {blocks} " blocks created today"</span>
            <span class="stat-item">"🔍 " {queries} " queries run today"</span>
        </div>
    }
}

// ── Main Dashboard Page ────────────────────────────────────────────────────────

/// Cognitive dashboard page component
#[component]
pub fn CognitiveDashboard() -> impl IntoView {
    // Async action to fetch morning briefing
    let fetch_briefing =
        Action::new_local(|_: &()| async move { bridge::get_morning_briefing().await });

    // Trigger initial fetch
    fetch_briefing.dispatch(());

    // Store the refresh callback in a StoredValue so it can be used in multiple closures
    let on_refresh = StoredValue::new(Callback::new(move |_| {
        let _ = fetch_briefing.dispatch(());
    }));

    view! {
        <div class="cognitive-dashboard">
            <div class="page-header">
                <h2>"🧠 Morning Briefing"</h2>
                <p class="page-subtitle">"Your daily cognitive overview"</p>
            </div>

            <Show
                when={move || !fetch_briefing.pending().get()}
                fallback={move || view! { <LoadingSkeleton /> }}
            >
                <Show
                    when={move || matches!(fetch_briefing.value().get(), Some(Err(_)))}
                    fallback={move || {
                        let cb = on_refresh.get_value();
                        view! {
                            <Show
                                when={move || {
                                    match fetch_briefing.value().get() {
                                        Some(Ok(_)) => true,
                                        _ => false,
                                    }
                                }}
                                fallback={move || view! { <LoadingSkeleton /> }}
                            >
                                {move || {
                                    let briefing = match fetch_briefing.value().get() {
                                        Some(Ok(b)) => b,
                                        _ => unreachable!("briefing should exist when condition is true"),
                                    };
                                    view! { <BriefingContent briefing={briefing} on_refresh={cb.clone()} /> }
                                }}
                            </Show>
                        }
                    }}
                >
                    {move || {
                        let msg = match fetch_briefing.value().get() {
                            Some(Err(e)) => match e {
                                bridge::BridgeError::TauriError(s) => s.clone(),
                                bridge::BridgeError::JsonError(s) => s.clone(),
                                bridge::BridgeError::Unavailable(s) => s.clone(),
                            },
                            _ => String::new(),
                        };
                        let cb = on_refresh.get_value();
                        view! {
                            <ErrorState message={msg} on_retry={cb} />
                        }
                    }}
                </Show>
            </Show>
        </div>
    }
}
