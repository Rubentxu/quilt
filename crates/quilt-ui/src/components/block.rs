//! Block component with Logseq-style light theme outliner

use leptos::prelude::*;

/// Block section with white background, subtle border, and lateral bullet
#[component]
pub fn Block(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <section class="block-section">
            <span class="block-bullet" />
            <div class="block-header">
                <h2 class="block-title">{title}</h2>
                <button class="block-menu-btn" aria-label="Block options">
                    "⋮"
                </button>
            </div>
            <div class="block-content">
                {children()}
            </div>
        </section>
    }
}

/// Empty state message within a block - compact, left-aligned
#[component]
pub fn EmptyState(message: &'static str) -> impl IntoView {
    view! {
        <div class="block-empty-state">
            {message}
        </div>
    }
}
