//! Mental Model Garden — belief evolution tracking
//!
//! Visualizes belief evolution over time from journal entries,
//! shows contradictions and deepening suggestions.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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

/// Mental model garden page component
#[component]
pub fn MentalModelGarden() -> impl IntoView {
    // Mock data for development
    let mock_beliefs = [
        BeliefDto {
            id: "belief-1".to_string(),
            statement: "Rust async is the future of concurrency".to_string(),
            confidence: 0.85,
            state: BeliefState::Strengthened,
            supporting_blocks: 5,
            last_updated: "2026-05-01".to_string(),
        },
        BeliefDto {
            id: "belief-2".to_string(),
            statement: "Memory safety is critical for systems".to_string(),
            confidence: 0.9,
            state: BeliefState::New,
            supporting_blocks: 3,
            last_updated: "2026-04-28".to_string(),
        },
        BeliefDto {
            id: "belief-3".to_string(),
            statement: "Async Rust is too complex for simple tasks".to_string(),
            confidence: 0.6,
            state: BeliefState::Weakened,
            supporting_blocks: 1,
            last_updated: "2026-03-15".to_string(),
        },
    ];

    let mock_contradictions = [ContradictionDto {
        belief_a_id: "belief-1".to_string(),
        belief_b_id: "belief-3".to_string(),
        explanation:
            "These beliefs express tension between Rust async being 'the future' vs 'too complex'"
                .to_string(),
        severity: 0.7,
    }];

    let mock_suggestions = [DeepeningSuggestionDto {
        concept: "Rust async runtime internals".to_string(),
        current_depth: 1,
        suggested_questions: vec![
            "How does Tokio schedule tasks?".to_string(),
            "What are the tradeoffs of different async runtimes?".to_string(),
        ],
    }];

    let state_color = |state: &BeliefState| -> &'static str {
        match state {
            BeliefState::New => "#3b82f6",
            BeliefState::Strengthened => "#22c55e",
            BeliefState::Weakened => "#f59e0b",
            BeliefState::Abandoned => "#9ca3af",
            BeliefState::Unchanged => "#6366f1",
        }
    };

    view! {
        <div class="mental-model-garden">
            <div class="page-header">
                <h2>"🌱 Mental Model Garden"</h2>
                <p class="page-subtitle">"Belief evolution from your journal"</p>
            </div>

            <div class="model-sections">
                <div class="contradiction-alerts">
                    <h3>"⚠️ Contradiction Alerts"</h3>
                    <div class="alert-count">{mock_contradictions.len()} contradictions</div>
                    {mock_contradictions.iter().map(|c| {
                        view! {
                            <div class="contradiction-card">
                                <div class="contradiction-explanation">{c.explanation.clone()}</div>
                                <div class="contradiction-severity">"Severity: {(c.severity * 100.0).round()}%"</div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="belief-timeline">
                    <h3>"📈 Belief Timeline"</h3>
                    {mock_beliefs.iter().map(|belief| {
                        let color = state_color(&belief.state);
                        view! {
                            <div class="belief-card" style:border-left-color={color}>
                                <div class="belief-header">
                                    <span class="belief-statement">{belief.statement.clone()}</span>
                                    <span class="belief-state">{format!("{:?}", belief.state)}</span>
                                </div>
                                <div class="belief-meta">
                                    <span>"Confidence: {(belief.confidence * 100.0).round()}%"</span>
                                    <span>"Evidence: {belief.supporting_blocks} blocks"</span>
                                    <span>"Updated: {belief.last_updated.clone()}"</span>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="deepening-section">
                    <h3>"💡 Deepening Suggestions"</h3>
                    {mock_suggestions.iter().map(|s| {
                        view! {
                            <div class="suggestion-card">
                                <div class="suggestion-concept">{s.concept.clone()}</div>
                                <div class="suggestion-depth">"Current depth: {s.current_depth} observations"</div>
                                <div class="suggestion-questions">
                                    {s.suggested_questions.iter().map(|q| {
                                        view! { <li>{q.clone()}</li> }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}
