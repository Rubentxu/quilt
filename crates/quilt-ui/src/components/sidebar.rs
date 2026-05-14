//! Application sidebar navigation

use crate::components::AgentPanel;
use leptos::prelude::*;
use leptos_router::components::*;

/// Sidebar navigation component
#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <h1>"Quilt"</h1>
            </div>
            <nav class="sidebar-nav" aria-label="Main navigation">
                <A href="/journal" attr:data-testid="nav-journal">
                    <span class="nav-item">"📅 Journal"</span>
                </A>
                <A href="/pages" attr:data-testid="nav-pages">
                    <span class="nav-item">"📄 Pages"</span>
                </A>
                <A href="/search" attr:data-testid="nav-search">
                    <span class="nav-item">"🔍 Search"</span>
                </A>
                <A href="/query" attr:data-testid="nav-query">
                    <span class="nav-item">"💬 Query"</span>
                </A>
                <A href="/graph" attr:data-testid="nav-graph">
                    <span class="nav-item">"🌐 Graph"</span>
                </A>
                <A href="/cognitive" attr:data-testid="nav-cognitive">
                    <span class="nav-item">"🧠 Cognitive"</span>
                </A>
            </nav>
            <div class="sidebar-footer">
                <AgentPanel page_name="current-page".to_string() />
            </div>
        </aside>
    }
}
