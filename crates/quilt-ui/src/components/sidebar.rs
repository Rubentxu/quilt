//! Application sidebar navigation with Logseq-style light theme
//!
//! Uses PluginRegistry for dynamic navigation items from plugins.
//! Currently uses hardcoded nav items; plugin integration is via extension points.

use crate::components::graph_switcher::GraphSwitcher;
use crate::components::theme::ThemeToggle;
use leptos::prelude::*;
use leptos_router::{components::*, hooks::use_location};

/// Sidebar navigation component - Logseq light style
#[component]
pub fn Sidebar() -> impl IntoView {
    // Get current path for active state
    let current_path = use_location().pathname;

    view! {
        <aside class="sidebar">
            <div class="sidebar-header">
                <div class="sidebar-logo">
                    <span class="sidebar-logo-icon">"🧠"</span>
                    <span class="sidebar-logo-text">"Quilt"</span>
                </div>
                <GraphSwitcher />
            </div>

            <nav class="sidebar-nav" aria-label="Main navigation">
                <A
                    href="/journal"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get() == "/journal" || current_path.get() == "/" { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get() == "/journal" || current_path.get() == "/" { "page" } else { "false" }}
                    attr:data-testid="nav-journal"
                >
                    <span class="nav-icon">"📅"</span>
                    <span class="nav-label">"Journal"</span>
                </A>
                <A
                    href="/pages"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get() == "/pages" { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get() == "/pages" { "page" } else { "false" }}
                    attr:data-testid="nav-pages"
                >
                    <span class="nav-icon">"📄"</span>
                    <span class="nav-label">"Pages"</span>
                </A>
                <A
                    href="/search"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get() == "/search" { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get() == "/search" { "page" } else { "false" }}
                    attr:data-testid="nav-search"
                >
                    <span class="nav-icon">"🔍"</span>
                    <span class="nav-label">"Search"</span>
                </A>
                <A
                    href="/query"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get() == "/query" { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get() == "/query" { "page" } else { "false" }}
                    attr:data-testid="nav-query"
                >
                    <span class="nav-icon">"💬"</span>
                    <span class="nav-label">"Query"</span>
                </A>
                <A
                    href="/graph"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get() == "/graph" { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get() == "/graph" { "page" } else { "false" }}
                    attr:data-testid="nav-graph"
                >
                    <span class="nav-icon">"🌐"</span>
                    <span class="nav-label">"Graph"</span>
                </A>
                <A
                    href="/cognitive"
                    attr:class={move || format!("sidebar-nav-item{}", if current_path.get().starts_with("/cognitive") { " active" } else { "" })}
                    attr:aria-current={move || if current_path.get().starts_with("/cognitive") { "page" } else { "false" }}
                    attr:data-testid="nav-cognitive"
                >
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
                <ThemeToggle />
                <button class="sidebar-footer-btn">"⚙️"</button>
                <button class="sidebar-footer-btn">"?"</button>
            </div>
        </aside>
    }
}
