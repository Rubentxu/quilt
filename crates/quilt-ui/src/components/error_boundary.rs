//! Error boundary component for catching and displaying errors
//!
//! Provides a fallback UI when child components throw errors.

use leptos::callback::Callback;
use leptos::prelude::*;

/// Simple error display component for use in pages
#[component]
pub fn ErrorDisplay(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="error-boundary">
            <div class="error-boundary-icon">"⚠️"</div>
            <h3 class="error-boundary-title">"Error"</h3>
            <p class="error-boundary-message">{message}</p>
            <div class="error-boundary-actions">
                <button
                    class="btn-retry"
                    on:click={move |_| {
                        on_retry.run(());
                    }}
                >
                    "Try Again"
                </button>
            </div>
        </div>
    }
}
