//! Toast container component - renders active toast notifications
//!
//! This component should be placed at the root of the application
//! to display toast notifications globally.

use crate::components::toast::{ToastState, ToastType};
use leptos::prelude::*;

/// Get CSS class for toast type
fn toast_class(toast_type: &ToastType) -> &'static str {
    match toast_type {
        ToastType::Success => "toast toast-success",
        ToastType::Error => "toast toast-error",
        ToastType::Warning => "toast toast-warning",
        ToastType::Info => "toast toast-info",
    }
}

/// Toast container component - displays all active toasts
#[component]
pub fn ToastContainer(toast_state: ToastState) -> impl IntoView {
    let toasts = toast_state.toasts;

    view! {
        <div class="toast-container" role="status" aria-live="polite">
            <Show when={move || !toasts.get().is_empty()}>
                <div class="toast-list">
                    {toasts.get().iter().map(|toast| {
                        let id = toast.id.clone();
                        let toast_type = toast.toast_type;
                        let toast_state = toast_state.clone();
                        view! {
                            <div class={toast_class(&toast_type)}>
                                <span class="toast-icon">{toast_type.icon()}</span>
                                <span class="toast-message">{toast.message.clone()}</span>
                                <button
                                    class="toast-dismiss"
                                    on:click={move |_| {
                                        toast_state.remove(&id);
                                    }}
                                >
                                    "×"
                                </button>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Show>
        </div>
    }
}
