//! Annotations overlay for highlighting and marking content
//!
//! Provides visual markers, popups, and resolution for annotations on blocks.
//! Supports creating, viewing, resolving, and deleting annotations.

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

impl Annotation {
    /// Create a new annotation with a generated ID and current timestamp
    pub fn new(block_id: &str, annotation_type: AnnotationType, content: &str) -> Self {
        Self {
            id: format!(
                "ann-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            block_id: block_id.to_string(),
            annotation_type,
            content: content.to_string(),
            resolved: false,
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[derive(Default)]
pub enum AnnotationType {
    Highlight,
    #[default]
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

    pub fn icon(&self) -> &'static str {
        match self {
            AnnotationType::Highlight => "📝",
            AnnotationType::Comment => "💬",
            AnnotationType::Question => "❓",
            AnnotationType::Important => "⭐",
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

/// New annotation form data
#[derive(Debug, Clone, Default)]
pub struct NewAnnotation {
    pub annotation_type: AnnotationType,
    pub content: String,
}

/// Single annotation item component
#[component]
fn AnnotationItem(
    annotation: Annotation,
    on_resolve: Callback<String, ()>,
    on_delete: Callback<String, ()>,
) -> impl IntoView {
    let ann = annotation;
    let id = ann.id.clone();
    let id_resolve = id.clone();
    let id_delete = id.clone();
    let icon = ann.annotation_type.icon().to_string();
    let label = ann.annotation_type.label().to_string();
    let color = ann.annotation_type.color().to_string();
    let resolved_class = ann.resolved;

    view! {
        <div class="annotation-item" class:resolved={resolved_class}>
            <div
                class="annotation-marker"
                style={format!("background-color: var(--accent-{})", color)}
            >
                {icon}
            </div>
            <div class="annotation-content">
                <div class="annotation-header">
                    <span class="annotation-type">{label}</span>
                    <span class="annotation-date">{ann.created_at.clone()}</span>
                </div>
                <p class="annotation-text">{ann.content.clone()}</p>
                <div class="annotation-actions">
                    <button
                        class="annotation-resolve-btn"
                        on:click={move |_| on_resolve.run(id_resolve.clone())}
                    >
                        "✓ Resolve"
                    </button>
                    <button
                        class="annotation-delete-btn"
                        on:click={move |_| on_delete.run(id_delete.clone())}
                    >
                        "🗑 Delete"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Annotations overlay component with create/view/respond functionality
#[component]
pub fn AnnotationsOverlay(
    annotations: Vec<Annotation>,
    is_visible: bool,
    on_resolve: Callback<String, ()>,
    on_delete: Callback<String, ()>,
    on_create: Option<Callback<Annotation, ()>>,
) -> impl IntoView {
    let annotations_sig = Signal::derive(move || annotations.clone());
    let is_visible_sig = Signal::derive(move || is_visible);

    // New annotation form state
    let new_annotation_type = RwSignal::new(AnnotationType::Comment);
    let new_annotation_content = RwSignal::new(String::new());
    let show_create_form = RwSignal::new(false);

    // Filtered annotations
    let active_annotations = move || {
        annotations_sig
            .get()
            .iter()
            .filter(|a| !a.resolved)
            .cloned()
            .collect::<Vec<_>>()
    };

    let resolved_annotations = move || {
        annotations_sig
            .get()
            .iter()
            .filter(|a| a.resolved)
            .cloned()
            .collect::<Vec<_>>()
    };

    // Handle creating a new annotation
    let handle_create = move |_| {
        let content = new_annotation_content.get();
        if content.trim().is_empty() {
            return;
        }

        if let Some(callback) = &on_create {
            let annotation = Annotation::new("current-block", new_annotation_type.get(), &content);
            callback.run(annotation);
        }

        // Reset form
        new_annotation_content.set(String::new());
        show_create_form.set(false);
    };

    view! {
        <Show when={move || is_visible_sig.get()}>
            <div class="annotations-overlay">
                <div class="annotations-panel">
                    <div class="annotations-header">
                        <h3>"Annotations"</h3>
                        <span class="annotations-count">{move || active_annotations().len()}</span>
                        <button
                            class="annotation-add-btn"
                            on:click={move |_| show_create_form.update(|v| *v = !*v)}
                        >
                            {move || if show_create_form.get() { "✕" } else { "+" }}
                        </button>
                    </div>

                    {/* Create annotation form */}
                    <Show when={move || show_create_form.get()}>
                        <div class="annotation-create-form">
                            <div class="annotation-type-selector">
                                <button
                                    class="annotation-type-btn"
                                    class:selected={move || new_annotation_type.get() == AnnotationType::Highlight}
                                    on:click={move |_| new_annotation_type.set(AnnotationType::Highlight)}
                                >
                                    "📝"
                                </button>
                                <button
                                    class="annotation-type-btn"
                                    class:selected={move || new_annotation_type.get() == AnnotationType::Comment}
                                    on:click={move |_| new_annotation_type.set(AnnotationType::Comment)}
                                >
                                    "💬"
                                </button>
                                <button
                                    class="annotation-type-btn"
                                    class:selected={move || new_annotation_type.get() == AnnotationType::Question}
                                    on:click={move |_| new_annotation_type.set(AnnotationType::Question)}
                                >
                                    "❓"
                                </button>
                                <button
                                    class="annotation-type-btn"
                                    class:selected={move || new_annotation_type.get() == AnnotationType::Important}
                                    on:click={move |_| new_annotation_type.set(AnnotationType::Important)}
                                >
                                    "⭐"
                                </button>
                            </div>
                            <textarea
                                class="annotation-input"
                                placeholder="Add your annotation..."
                                on:input={move |e| {
                                    new_annotation_content.set(event_target_value(&e));
                                }}
                            />
                            <div class="annotation-form-actions">
                                <button
                                    class="annotation-cancel-btn"
                                    on:click={move |_| {
                                        new_annotation_content.set(String::new());
                                        show_create_form.set(false);
                                    }}
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="annotation-submit-btn"
                                    on:click={handle_create}
                                >
                                    "Add"
                                </button>
                            </div>
                        </div>
                    </Show>

                    {/* Active annotations */}
                    <div class="annotations-list">
                        <Show when={move || active_annotations().is_empty()}>
                            <div class="annotations-empty">
                                "No active annotations"
                            </div>
                        </Show>

                        <For each={move || active_annotations()} key=|ann| ann.id.clone() let:ann>
                            <AnnotationItem
                                annotation={ann}
                                on_resolve={on_resolve}
                                on_delete={on_delete}
                            />
                        </For>
                    </div>

                    {/* Resolved annotations section */}
                    <Show when={move || !resolved_annotations().is_empty()}>
                        <div class="resolved-section">
                            <h4 class="resolved-title">"Resolved"</h4>
                            <div class="annotations-list resolved-list">
                                <For each={move || resolved_annotations()} key=|ann| ann.id.clone() let:ann>
                                    <AnnotationItem
                                        annotation={ann}
                                        on_resolve={on_resolve}
                                        on_delete={on_delete}
                                    />
                                </For>
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
