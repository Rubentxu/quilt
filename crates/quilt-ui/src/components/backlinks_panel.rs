//! Backlinks panel component showing pages/blocks that reference the current page
//!
//! Displays enriched provenance information including context, timestamps, and relationship strength.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::mini_backlinks_graph::MiniBacklinksGraphView;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Backlink {
    pub id: String,
    pub source_id: String,
    pub source_title: String,
    pub source_preview: String,
    pub context: String,
    pub relationship_type: RelationshipType,
    pub created_at: String,
    pub provenance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    Direct,
    Transitive,
    Semantic,
}

impl RelationshipType {
    pub fn label(&self) -> &str {
        match self {
            RelationshipType::Direct => "Direct",
            RelationshipType::Transitive => "Transitive",
            RelationshipType::Semantic => "Semantic",
        }
    }
}

#[component]
pub fn BacklinksPanel(
    backlinks: Vec<Backlink>,
    current_page: String,
) -> impl IntoView {
    let backlinks_sig = Signal::derive(move || backlinks.clone());
    let current_page_sig = Signal::derive(move || current_page.clone());

    view! {
        <div class="backlinks-panel">
            <div class="backlinks-header">
                <h3>"Backlinks"</h3>
                <span class="backlinks-count">{backlinks_sig.get().len()}</span>
            </div>
            <Show when={move || backlinks_sig.get().is_empty()}>
                <div class="backlinks-empty">
                    <p>"No backlinks to this page"</p>
                </div>
            </Show>
            <Show when={move || !backlinks_sig.get().is_empty()}>
                <MiniBacklinksGraphView
                    backlinks={backlinks_sig.get()}
                    current_page={current_page_sig.get()}
                />
            </Show>
            <div class="backlinks-list">
                <For each={move || backlinks_sig.get()} key=|link| link.id.clone() let:link>
                    <div class="backlink-item">
                        <div class="backlink-header">
                            <span class="backlink-source">{link.source_title.clone()}</span>
                            <span class="backlink-rel-type">{link.relationship_type.label().to_string()}</span>
                        </div>
                        <p class="backlink-context">"{link.context.clone()}"</p>
                        <div class="backlink-meta">
                            <span class="backlink-score">
                                {format!("{:.0}%", link.provenance_score * 100.0)}
                            </span>
                            <span class="backlink-date">{link.created_at.clone()}</span>
                        </div>
                    </div>
                </For>
            </div>
        </div>
    }
}
