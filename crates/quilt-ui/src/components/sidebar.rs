//! Application sidebar navigation with Logseq-style light theme

use leptos::prelude::*;
use leptos_router::components::*;

/// Sidebar navigation component - Logseq light style
#[component]
pub fn Sidebar() -> impl IntoView {
    view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <div class="sidebar-logo">
                    <span class="sidebar-logo-icon">"🧠"</span>
                    <span class="sidebar-logo-text">"Quilt"</span>
                </div>
            </div>

            <nav class="sidebar-nav" aria-label="Main navigation">
                <A href="/journal" attr:class="sidebar-nav-item active" attr:data-testid="nav-journal">
                    <span class="nav-icon">"📅"</span>
                    <span class="nav-label">"Journal"</span>
                </A>
                <A href="/pages" attr:class="sidebar-nav-item" attr:data-testid="nav-pages">
                    <span class="nav-icon">"📄"</span>
                    <span class="nav-label">"Pages"</span>
                </A>
                <A href="/search" attr:class="sidebar-nav-item" attr:data-testid="nav-search">
                    <span class="nav-icon">"🔍"</span>
                    <span class="nav-label">"Search"</span>
                </A>
                <A href="/query" attr:class="sidebar-nav-item" attr:data-testid="nav-query">
                    <span class="nav-icon">"💬"</span>
                    <span class="nav-label">"Query"</span>
                </A>
                <A href="/graph" attr:class="sidebar-nav-item" attr:data-testid="nav-graph">
                    <span class="nav-icon">"🌐"</span>
                    <span class="nav-label">"Graph"</span>
                </A>
                <A href="/cognitive" attr:class="sidebar-nav-item" attr:data-testid="nav-cognitive">
                    <span class="nav-icon">"🧠"</span>
                    <span class="nav-label">"Cognitive"</span>
                </A>
            </nav>

            <div class="sidebar-agent">
                <button class="sidebar-agent-btn">
                    <span class="agent-icon">"🤖"</span>
                    <span class="agent-label">"Agent"</span>
                    <span class="agent-chevron">"›"</span>
                </button>
                <p class="sidebar-agent-hint">
                    "Agent on current page —"
                    <br />
                    "connect backend to enable"
                </p>
            </div>

            <div class="sidebar-footer">
                <button class="sidebar-footer-btn">"⚙️"</button>
                <button class="sidebar-footer-btn">"?"</button>
            </div>
        </aside>
    }
}
