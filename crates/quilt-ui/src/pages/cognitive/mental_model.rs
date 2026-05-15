//! Mental Model Garden — belief evolution tracking
//!
//! Visualizes belief evolution over time from journal entries,
//! shows contradictions and deepening suggestions.

use leptos::callback::Callable;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::{self, BridgeError};

/// State of a belief
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BeliefState {
    New,
    Strengthened,
    Weakened,
    Abandoned,
    Unchanged,
}

/// A belief in the mental model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeliefDto {
    pub id: String,
    pub statement: String,
    pub confidence: f32,
    pub state: BeliefState,
    pub supporting_blocks: usize,
    pub last_updated: String,
}

/// A contradiction between beliefs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionDto {
    pub belief_a_id: String,
    pub belief_b_id: String,
    pub explanation: String,
    pub severity: f32,
}

/// A deepening suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepeningSuggestionDto {
    pub concept: String,
    pub current_depth: usize,
    pub suggested_questions: Vec<String>,
}

/// Mental model data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalModelDto {
    pub agent_id: String,
    pub beliefs: Vec<BeliefDto>,
    pub contradictions: Vec<ContradictionDto>,
    pub suggestions: Vec<DeepeningSuggestionDto>,
}

/// Response from mental_model Tauri command
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MentalModelResponse {
    #[serde(rename = "available")]
    available: bool,
    #[serde(rename = "message")]
    message: Option<String>,
    #[serde(rename = "model")]
    model: Option<MentalModelDto>,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn state_color(state: &BeliefState) -> &'static str {
    match state {
        BeliefState::New => "#3b82f6",
        BeliefState::Strengthened => "#22c55e",
        BeliefState::Weakened => "#f59e0b",
        BeliefState::Abandoned => "#9ca3af",
        BeliefState::Unchanged => "#6366f1",
    }
}

// ── Loading Skeleton ──────────────────────────────────────────────────────────

#[component]
fn LoadingSkeleton() -> impl IntoView {
    view! {
        <div class="mental-model-loading">
            <div class="skeleton-header"></div>
            <div class="skeleton-sections">
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
                <div class="skeleton-card"></div>
            </div>
        </div>
    }
}

// ── Error State ───────────────────────────────────────────────────────────────

#[component]
fn ModelErrorState(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="mental-model-error">
            <p class="error-message">"Failed to load mental model: " {message}</p>
            <button class="btn-retry" on:click={move |_| on_retry.run(())}>
                "🔄 Retry"
            </button>
        </div>
    }
}

// ── Belief Card ───────────────────────────────────────────────────────────────

#[component]
fn BeliefCard(belief: BeliefDto) -> impl IntoView {
    let color = state_color(&belief.state);
    view! {
        <div class="belief-card" style:border-left-color={color}>
            <div class="belief-header">
                <span class="belief-statement">{belief.statement}</span>
                <span class="belief-state">{format!("{:?}", belief.state)}</span>
            </div>
            <div class="belief-meta">
                <span>"Confidence: {(belief.confidence * 100.0).round()}%"</span>
                <span>"Evidence: {} blocks", belief.supporting_blocks</span>
                <span>"Updated: {}", belief.last_updated</span>
            </div>
        </div>
    }
}

// ── Contradiction Cards ───────────────────────────────────────────────────────

#[component]
fn ContradictionCards(contradictions: Vec<ContradictionDto>) -> impl IntoView {
    let c_for_empty = contradictions.clone();
    view! {
        <Show
            when={move || !c_for_empty.is_empty()}
            fallback={move || view! { <div></div> }}
        >
            <div class="contradictions-container">
                {contradictions.iter().map(|c| {
                    view! {
                        <div class="contradiction-card">
                            <div class="contradiction-explanation">{c.explanation.clone()}</div>
                            <div class="contradiction-severity">"Severity: {(c.severity * 100.0).round()}%"</div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </Show>
    }
}

// ── Beliefs List ─────────────────────────────────────────────────────────────

#[component]
fn BeliefsList(beliefs: Vec<BeliefDto>) -> impl IntoView {
    let beliefs_for_empty = beliefs.clone();
    view! {
        <Show
            when={move || !beliefs_for_empty.is_empty()}
            fallback={move || view! { <div class="empty-state"><p>"No beliefs tracked yet."</p></div> }}
        >
            <div class="beliefs-container">
                {beliefs.iter().map(|b| {
                    view! { <BeliefCard belief={b.clone()} /> }
                }).collect::<Vec<_>>()}
            </div>
        </Show>
    }
}

// ── Suggestions List ───────────────────────────────────────────────────────────

#[component]
fn SuggestionsList(suggestions: Vec<DeepeningSuggestionDto>) -> impl IntoView {
    let suggestions_for_empty = suggestions.clone();
    view! {
        <Show
            when={move || !suggestions_for_empty.is_empty()}
            fallback={move || view! { <div class="empty-state"><p>"No suggestions yet."</p></div> }}
        >
            <div class="suggestions-container">
                {suggestions.iter().map(|s| {
                    view! {
                        <div class="suggestion-card">
                            <div class="suggestion-concept">{s.concept.clone()}</div>
                            <div class="suggestion-depth">"Current depth: {} observations", s.current_depth</div>
                            <div class="suggestion-questions">
                                {s.suggested_questions.iter().map(|q| {
                                    view! { <li>{q.clone()}</li> }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </Show>
    }
}

// ── Mental Model Garden Page ──────────────────────────────────────────────────

/// Mental model garden page component
#[component]
pub fn MentalModelGarden() -> impl IntoView {
    // Default agent ID — in future this could come from route or user context
    let agent_id = "default-agent".to_string();
    let agent_id_for_fetch = agent_id.clone();

    // Async action to fetch mental model
    let fetch_model = Action::new_local(move |_: &()| {
        let name = agent_id_for_fetch.clone();
        async move {
            match bridge::get_mental_model(&name).await {
                Ok(json) => match serde_json::from_value::<MentalModelResponse>(json.clone()) {
                    Ok(resp) if !resp.available => Err(BridgeError::Unavailable(
                        resp.message
                            .unwrap_or_else(|| "Mental model gardener unavailable".into()),
                    )),
                    Ok(resp) => Ok(resp.model.unwrap_or(MentalModelDto {
                        agent_id: name,
                        beliefs: vec![],
                        contradictions: vec![],
                        suggestions: vec![],
                    })),
                    Err(_) => match serde_json::from_value::<MentalModelDto>(json) {
                        Ok(m) => Ok(m),
                        Err(_) => Ok(MentalModelDto {
                            agent_id: name,
                            beliefs: vec![],
                            contradictions: vec![],
                            suggestions: vec![],
                        }),
                    },
                },
                Err(e) => Err(e),
            }
        }
    });

    // Store refresh callback
    let on_refresh = StoredValue::new(Callback::new(move |_| {
        let _ = fetch_model.dispatch(());
    }));

    // Trigger initial fetch
    fetch_model.dispatch(());

    // Extract reactive values BEFORE the view! macro — dashboard pattern
    let pending = fetch_model.pending();
    let value = fetch_model.value();
    let cb = on_refresh.get_value();

    view! {
        <div class="mental-model-garden">
            <div class="page-header">
                <h2>"🌱 Mental Model Garden"</h2>
                <p class="page-subtitle">"Belief evolution from your journal"</p>
            </div>

            <Show
                when={move || !pending.get()}
                fallback={move || view! { <LoadingSkeleton /> }}
            >
                <Show
                    when={move || !matches!(value.get(), Some(Err(_)))}
                    fallback={move || {
                        let msg = match value.get() {
                            Some(Err(BridgeError::TauriError(s))) => s.clone(),
                            Some(Err(BridgeError::JsonError(s))) => s.clone(),
                            Some(Err(BridgeError::Unavailable(s))) => s.clone(),
                            _ => String::new(),
                        };
                        view! { <ModelErrorState message={msg} on_retry={cb.clone()} /> }
                    }}
                >
                    <div class="model-sections">
                        <div class="contradiction-alerts">
                            <h3>"⚠️ Contradiction Alerts"</h3>
                            <div class="alert-count">{match value.get() {
                                Some(Ok(m)) => m.contradictions.len(),
                                _ => 0,
                            }} contradictions</div>
                            <ContradictionCards contradictions={match value.get() {
                                Some(Ok(m)) => m.contradictions,
                                _ => vec![],
                            }} />
                        </div>

                        <div class="belief-timeline">
                            <h3>"📈 Belief Timeline"</h3>
                            <BeliefsList beliefs={match value.get() {
                                Some(Ok(m)) => m.beliefs,
                                _ => vec![],
                            }} />
                        </div>

                        <div class="deepening-section">
                            <h3>"💡 Deepening Suggestions"</h3>
                            <SuggestionsList suggestions={match value.get() {
                                Some(Ok(m)) => m.suggestions,
                                _ => vec![],
                            }} />
                        </div>
                    </div>
                </Show>
            </Show>
        </div>
    }
}
