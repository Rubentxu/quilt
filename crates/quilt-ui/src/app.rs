//! Application root component with routing

use crate::components::sidebar::Sidebar;
use crate::pages::cognitive::{
    ArgumentMapView, CognitiveDashboard, MentalModelGarden, SerendipityFeed,
};
use crate::pages::{
    journal::JournalView, page_list::PagesView, query::QueryView, search::SearchView,
};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

/// Main application component
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet href="/style.css" />
        <Title text="Quilt — AI-first Knowledge Graph" />

        <Router>
            <div class="app-layout">
                <Sidebar />
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
                        <Route path=path!("/cognitive/arguments/:page") view=move || {
                            view! { <ArgumentMapView _page_name="demo-page".to_string() /> }
                        } />
                        <Route path=path!("/cognitive/models") view=MentalModelGarden />
                    </Routes>
                </main>
            </div>
        </Router>
    }
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
