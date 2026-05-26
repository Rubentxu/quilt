//! Loading indicator

use leptos::prelude::*;

#[component]
pub fn Loading() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center py-8">
            <div class="animate-spin rounded-full h-6 w-6 border-2 border-accent border-t-transparent"></div>
        </div>
    }
}
