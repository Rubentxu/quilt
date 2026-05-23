//! Application root component with routing

use crate::components::agent_status::AgentStatusBar;
use crate::components::keyboard_shortcuts::KeyboardShortcuts;
use crate::components::right_sidebar::{RightSidebar, RightSidebarToggle};
use crate::components::sidebar::Sidebar;
use crate::components::theme::ThemeToggle;
use crate::pages::cognitive::{
    ArgumentMapView, CognitiveDashboard, MentalModelGarden, SerendipityFeed,
};
use crate::pages::{
    graph::GraphView, journal::JournalView, page_editor::PageEditor, page_list::PagesView, query::QueryView,
    search::SearchView,
};
use crate::state::{provide_app_state, use_app_state};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, hooks::use_params, params::Params, path};

/// Main application component with Logseq-style grid layout
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Provide global app state
    let app_state = provide_app_state();

    view! {
        <Stylesheet href="/style.css" />
        <Title text="Quilt — AI-first Knowledge Graph" />

        {/* Global keyboard shortcuts */}
        <KeyboardShortcutsWrapper />

        {/* Skip link for keyboard navigation - WCAG 2.1 SC 2.4.1 */}
        <a href="#main-content" class="skip-link">"Skip to main content"</a>

        <Router>
            <div class="app-shell">
                <SidebarWrapper />

                <header class="mobile-header">
                    <div class="mobile-header-brand">
                        <span class="mobile-header-icon">"🧠"</span>
                        <span class="mobile-header-text">"Quilt"</span>
                    </div>
                    <button
                        class="mobile-header-menu"
                        type="button"
                        on:click={move |_| app_state.toggle_mobile_menu()}
                        data-testid="mobile-menu-button"
                    >
                        {move || if app_state.sidebar.mobile_menu_open.get() { "×" } else { "☰" }}
                    </button>
                </header>

                {/* Agent status bar */}
                <AgentStatusBarWrapper />

                {/* Main content with sidebar */}
                <MainContentWithSidebar />
            </div>
        </Router>
    }
}

/// Wrapper component to access app state and render agent status bar
#[component]
fn AgentStatusBarWrapper() -> impl IntoView {
    let app_state = use_app_state();

    view! {
        <AgentStatusBar activities={app_state.agents.activities.get()} />
    }
}

/// Sidebar wrapper - shows sidebar on desktop, overlay on mobile
#[component]
fn SidebarWrapper() -> impl IntoView {
    let app_state = use_app_state();

    view! {
        <>
            {/* Desktop sidebar - always visible on desktop */}
            <div class="desktop-sidebar">
                <Sidebar />
            </div>

            {/* Mobile sidebar overlay */}
            <Show when={move || app_state.sidebar.mobile_menu_open.get()}>
                <div class="mobile-sidebar-overlay" on:click={move |_| {
                    let state = use_app_state();
                    state.close_mobile_menu();
                }}>
                    <div class="mobile-sidebar" on:click={move |ev| ev.stop_propagation()}>
                        <Sidebar />
                    </div>
                </div>
            </Show>
        </>
    }
}

/// Header bar with theme toggle
#[component]
fn HeaderBar() -> impl IntoView {
    view! {
        <header class="app-header">
            <ThemeToggle />
        </header>
    }
}

/// Wrapper component for global keyboard shortcuts
#[component]
fn KeyboardShortcutsWrapper() -> impl IntoView {
    // We need to clone app_state for each closure since it doesn't implement Copy
    let app_state1 = use_app_state().clone();
    let app_state2 = use_app_state().clone();
    let _app_state3 = use_app_state().clone();
    let app_state4 = use_app_state().clone();

    // Set up keyboard shortcuts immediately (WASM runs once)
    let shortcuts = KeyboardShortcuts::new(
        move || {
            app_state1.toggle_sidebar();
        },
        move || {
            // TODO: Open search modal
        },
        move || {
            // Close sidebar or mobile menu if open
            app_state2.close_sidebar();
            app_state2.close_mobile_menu();
        },
        move || {
            // TODO: Open slash command palette when in editor
        },
        move || {
            // Toggle mobile menu
            app_state4.toggle_mobile_menu();
        },
    );
    shortcuts.mount();

    // This component doesn't render anything
    view! { <></> }
}

/// Main content area with right sidebar
#[component]
fn MainContentWithSidebar() -> impl IntoView {
    let app_state = use_app_state();

    view! {
        <>
            {/* Right sidebar toggle button */}
            <RightSidebarToggle
                is_open={app_state.sidebar.is_open}
                selected_tab={app_state.sidebar.selected_tab}
            />

            {/* Right sidebar panel */}
            <RightSidebar
                is_open={app_state.sidebar.is_open}
                selected_tab={app_state.sidebar.selected_tab}
                _block_id={app_state.current_block.block_id.get().unwrap_or_default()}
                block_properties={app_state.current_block.properties.get()}
                backlinks={app_state.current_block.backlinks.get()}
                current_page_title={app_state.current_block.page_name.get().unwrap_or_default()}
                annotations={app_state.current_block.annotations.get()}
                on_properties_update={Callback::new(move |_| {
                    // TODO: persist property changes
                })}
                on_annotation_resolve={Callback::new(move |id| {
                    app_state.mark_notification_read(id);
                })}
                on_annotation_delete={Callback::new(move |_| {
                    // TODO: delete annotation
                })}
            />

            <main class="main-content" id="main-content" role="main">
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=JournalView />
                    <Route path=path!("/journal") view=JournalView />
                    <Route path=path!("/journal/:date") view=JournalView />
                    <Route path=path!("/pages") view=PagesView />
                    <Route path=path!("/pages/:id") view=PageEditor />
                    <Route path=path!("/search") view=SearchView />
                    <Route path=path!("/query") view=QueryView />
                    <Route path=path!("/cognitive") view=CognitiveDashboard />
                    <Route path=path!("/cognitive/serendipity") view=SerendipityFeed />
                    <Route path=path!("/cognitive/arguments/:page") view=ArgumentMapRoute />
                    <Route path=path!("/cognitive/models") view=MentalModelGarden />
                    <Route path=path!("/graph") view=GraphView />
                </Routes>
            </main>
        </>
    }
}

/// Route wrapper for ArgumentMapView — extracts :page param and passes to component
#[component]
fn ArgumentMapRoute() -> impl IntoView {
    let params = use_params::<ArgumentMapParams>();
    let page_name = params.with(|result| {
        result
            .as_ref()
            .ok()
            .and_then(|p| p.page.clone())
            .unwrap_or_else(|| "unknown".to_string())
    });
    view! { <ArgumentMapView page_name={page_name} /> }
}

#[derive(Params, Debug, PartialEq, Eq)]
struct ArgumentMapParams {
    page: Option<String>,
}

/// 404 Not Found page
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="empty-state">
            <h2>"Page not found"</h2>
            <p>"The page you're looking for doesn't exist."</p>
        </div>
    }
}
