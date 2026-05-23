//! Serendipity Feed — unexpected connections discovery
//!
//! Paginated view of unexpected but meaningful connections
//! discovered between knowledge blocks.

use leptos::callback::Callable;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::{self, BridgeError};

/// A serendipitous connection between two blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDto {
    pub score: f32,
    pub bridge: Option<String>,
    pub source_block_id: String,
    pub target_block_id: String,
    pub connection_type: String,
}

/// Response from the serendipity Tauri command
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerendipityResponse {
    #[serde(rename = "available")]
    available: bool,
    #[serde(rename = "message")]
    message: Option<String>,
    #[serde(rename = "connections")]
    connections: Option<Vec<ConnectionDto>>,
}

// ── Loading Skeleton ──────────────────────────────────────────────────────────

#[component]
fn LoadingSkeleton() -> impl IntoView {
    view! {
        <div class="serendipity-loading">
            <div class="skeleton-header"></div>
            <div class="skeleton-list">
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
            </div>
        </div>
    }
}

// ── Error State ───────────────────────────────────────────────────────────────

#[component]
fn ErrorState(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="serendipity-error">
            <p class="error-message">"Failed to load serendipity feed: " {message}</p>
            <button class="btn-retry" on:click={move |_| on_retry.run(())}>
                "🔄 Retry"
            </button>
        </div>
    }
}

// ── Connection Card ────────────────────────────────────────────────────────────

#[component]
fn ConnectionCard(conn: ConnectionDto) -> impl IntoView {
    view! {
        <div class="connection-card">
            <div class="connection-score">
                <span class="score-value">{(conn.score * 100.0).round()}</span>
                <span class="score-label">"% confidence"</span>
            </div>
            <div class="connection-bridge">
                {conn.bridge.clone().unwrap_or_else(|| "No bridge description".to_string())}
            </div>
            <div class="connection-meta">
                <span class="connection-type">{conn.connection_type.clone()}</span>
                <span class="block-refs">
                    "{conn.source_block_id[..8].min(conn.source_block_id.len())}... → {conn.target_block_id[..8].min(conn.target_block_id.len())}..."
                </span>
            </div>
        </div>
    }
}

// ── Serendipity Feed Page ─────────────────────────────────────────────────────

/// Serendipity feed page component
#[component]
pub fn SerendipityFeed() -> impl IntoView {
    let (min_confidence, set_min_confidence) = signal(0.3f32);

    // Get current value before capturing in closure
    let mc = min_confidence.get();

    // Action captures mc (f32, Copy) directly
    let fetch_serendipity = Action::new_local(move |_: &()| async move {
        match bridge::get_serendipity(None, Some(20), Some(mc)).await {
            Ok(json) => match serde_json::from_value::<SerendipityResponse>(json.clone()) {
                Ok(resp) if !resp.available => Err(BridgeError::Network(
                    resp.message
                        .unwrap_or_else(|| "Serendipity engine unavailable".into()),
                )),
                Ok(resp) => Ok(resp.connections.unwrap_or_default()),
                Err(_) => match serde_json::from_value::<Vec<ConnectionDto>>(json) {
                    Ok(conns) => Ok(conns),
                    Err(_) => Ok(vec![]),
                },
            },
            Err(e) => Err(e),
        }
    });

    let on_refresh = StoredValue::new(Callback::new(move |_| {
        fetch_serendipity.dispatch(());
    }));

    fetch_serendipity.dispatch(());

    view! {
        <div class="serendipity-feed">
            <div class="page-header">
                <h2>"✨ Serendipity Feed"</h2>
                <p class="page-subtitle">"Unexpected connections discovered"</p>
            </div>

            <div class="filters">
                <label>
                    "Min Confidence: "
                    <input
                        type="range"
                        min="0"
                        max="1"
                        step="0.1"
                        value={min_confidence.get()}
                        on:input={move |ev| {
                            let val = event_target_value(&ev).parse::<f32>().unwrap_or(0.3);
                            set_min_confidence.set(val);
                            fetch_serendipity.dispatch(());
                        }}
                    />
                    <span>{format!("{:.1}", min_confidence.get())}</span>
                </label>
            </div>

            <Show
                when={move || !fetch_serendipity.pending().get()}
                fallback={move || view! { <LoadingSkeleton /> }}
            >
                <Show
                    when={move || !matches!(fetch_serendipity.value().get(), Some(Err(_)))}
                    fallback={move || {
                        let msg = match fetch_serendipity.value().get() {
                            Some(Err(BridgeError::Network(s))) => s.clone(),
                            Some(Err(BridgeError::JsonError(s))) => s.clone(),
                            Some(Err(BridgeError::Network(s))) => s.clone(),
                            _ => String::new(),
                        };
                        let cb = on_refresh.get_value();
                        view! { <ErrorState message={msg} on_retry={cb} /> }
                    }}
                >
                    {move || {
                        let conns = match fetch_serendipity.value().get() {
                            Some(Ok(cs)) => cs,
                            _ => vec![],
                        };
                        let conns_is_empty = conns.is_empty();
                        let conns_clone = conns.clone();
                        view! {
                            <div class="connection-list">
                                <Show
                                    when={move || !conns_is_empty}
                                    fallback={move || view! { <div class="empty-state"><p>"No serendipitous connections found."</p></div> }}
                                >
                                    {conns_clone.iter().map(|conn| view! { <ConnectionCard conn={conn.clone()} /> }).collect::<Vec<_>>()}
                                </Show>
                            </div>
                        }
                    }}
                </Show>
            </Show>

            <div class="pagination">
                <button disabled>"← Previous"</button>
                <span class="page-indicator">"Page 1"</span>
                <button disabled>"Next →"</button>
            </div>
        </div>
    }
}
