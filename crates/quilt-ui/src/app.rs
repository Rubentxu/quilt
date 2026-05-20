//! Application root component with routing

use crate::components::sidebar::Sidebar;
use crate::pages::cognitive::{
    ArgumentMapView, CognitiveDashboard, MentalModelGarden, SerendipityFeed,
};
use crate::pages::{
    graph::GraphView, journal::JournalView, page_list::PagesView, query::QueryView,
    search::SearchView,
};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, hooks::use_params, params::Params, path};

/// Main application component with Logseq-style grid layout
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet href="/style.css" />
        <Title text="Quilt — AI-first Knowledge Graph" />

        <Router>
            <div class="app-shell">
                <Sidebar />
                <header class="mobile-header">
                    <div class="mobile-header-brand">
                        <span class="mobile-header-icon">"🧠"</span>
                        <span class="mobile-header-text">"Quilt"</span>
                    </div>
                    <button class="mobile-header-menu" type="button">
                        "Menu"
                    </button>
                </header>
                <main class="main-content">
                    <Routes fallback=|| view! { <NotFound /> }>
                        <Route path=path!("/") view=JournalView />
                        <Route path=path!("/journal") view=JournalView />
                        <Route path=path!("/journal/:date") view=JournalView />
                        <Route path=path!("/pages") view=PagesView />
                        <Route path=path!("/search") view=SearchView />
                        <Route path=path!("/query") view=QueryView />
                        <Route path=path!("/cognitive") view=CognitiveDashboard />
                        <Route path=path!("/cognitive/serendipity") view=SerendipityFeed />
                        <Route path=path!("/cognitive/arguments/:page") view=ArgumentMapRoute />
                        <Route path=path!("/cognitive/models") view=MentalModelGarden />
                        <Route path=path!("/graph") view=GraphView />
                    </Routes>
                </main>
            </div>
        </Router>
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
