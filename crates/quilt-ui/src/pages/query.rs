//! Query view — query builder interface

use leptos::prelude::*;
use crate::components::EmptyState;

/// Query builder view
#[component]
pub fn QueryView() -> impl IntoView {
    view! {
        <div class="query-view">
            <div class="page-header">
                <h2>"Query"</h2>
                <p class="page-subtitle">"Query your knowledge graph with QuiltQL"</p>
            </div>

            <div class="card">
                <EmptyState message="Query builder coming soon — wire to quilt-query crate" />
            </div>
        </div>
    }
}
