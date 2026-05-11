//! Loading spinner component

use leptos::prelude::*;

/// Loading indicator component
#[component]
pub fn Loading() -> impl IntoView {
    view! {
        <div class="loading">
            <p>"Loading..."</p>
        </div>
    }
}
