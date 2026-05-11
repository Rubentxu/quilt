//! Empty state component

use leptos::prelude::*;

/// Empty state message component
#[component]
pub fn EmptyState(message: &'static str) -> impl IntoView {
    view! {
        <div class="empty-state">
            <p>{message}</p>
        </div>
    }
}
