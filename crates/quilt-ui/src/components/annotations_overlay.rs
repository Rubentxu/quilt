//! Annotations overlay for highlighting and marking content
//!
//! Provides visual markers, popups, and resolution for annotations on blocks.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub block_id: String,
    pub annotation_type: AnnotationType,
    pub content: String,
    pub resolved: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnnotationType {
    Highlight,
    Comment,
    Question,
    Important,
}

impl AnnotationType {
    pub fn label(&self) -> &str {
        match self {
            AnnotationType::Highlight => "Highlight",
            AnnotationType::Comment => "Comment",
            AnnotationType::Question => "Question",
            AnnotationType::Important => "Important",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            AnnotationType::Highlight => "yellow",
            AnnotationType::Comment => "blue",
            AnnotationType::Question => "purple",
            AnnotationType::Important => "red",
        }
    }
}

#[component]
pub fn AnnotationsOverlay(
    annotations: Vec<Annotation>,
    is_visible: bool,
    _on_resolve: Callback<String, ()>,
    _on_delete: Callback<String, ()>,
) -> impl IntoView {
    let annotations_sig = Signal::derive(move || annotations.clone());
    let is_visible_sig = Signal::derive(move || is_visible);

    view! {
        <Show when={move || is_visible_sig.get()}>
            <div class="annotations-overlay">
                <div class="annotations-panel">
                    <div class="annotations-header">
                        <h3>"Annotations"</h3>
                        <span class="annotations-count">{annotations_sig.get().len()}</span>
                    </div>
                    <div class="annotations-list">
                        <For each={move || annotations_sig.get()} key=|ann| ann.id.clone() let:ann>
                            <div class="annotation-item" class:resolved={ann.resolved}>
                                <div class="annotation-marker" style="background-color: var(--accent)">
                                </div>
                                <div class="annotation-content">
                                    <div class="annotation-header">
                                        <span class="annotation-type">{ann.annotation_type.label().to_string()}</span>
                                        <span class="annotation-date">{ann.created_at.clone()}</span>
                                    </div>
                                    <p class="annotation-text">{ann.content.clone()}</p>
                                    <div class="annotation-actions">
                                        <Show when={move || ann.resolved == false}>
                                            <button class="annotation-resolve-btn">
                                                "Resolve"
                                            </button>
                                        </Show>
                                        <button class="annotation-delete-btn">
                                            "Delete"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        </For>
                    </div>
                </div>
            </div>
        </Show>
    }
}
