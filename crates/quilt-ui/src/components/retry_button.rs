//! Retry button component with loading state
//!
//! Provides a button that shows loading state while retry is in progress.

use leptos::callback::Callback;
use leptos::prelude::*;

/// Retry button component
#[component]
pub fn RetryButton(
    /// Callback to execute on retry
    on_retry: Callback<()>,
    /// Whether retry is currently loading
    loading: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            class="retry-button"
            class:loading={move || loading.get()}
            disabled={move || loading.get()}
            on:click={move |_| {
                loading.set(true);
                on_retry.run(());
            }}
        >
            <span class="spinner"></span>
            "Retry"
        </button>
    }
}
