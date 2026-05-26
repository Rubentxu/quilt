//! Application root component with Logseq-like layout

use crate::bridge::BacklinkDto;
use crate::components::right_sidebar::RightSidebar;
use crate::components::sidebar::LeftSidebar;
use crate::pages::{
    journal::JournalView, page::PageView, page_list::PagesView, search::SearchView,
};
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

#[component]
pub fn App() -> impl IntoView {
    let (left_sidebar_open, set_left_sidebar_open) = signal(true);
    let (right_sidebar_open, set_right_sidebar_open) = signal(false);
    let backlinks = RwSignal::new(Vec::<BacklinkDto>::new());
    let backlinks_loading = RwSignal::new(false);

    provide_context(left_sidebar_open);
    provide_context(set_left_sidebar_open);
    provide_context(right_sidebar_open);
    provide_context(set_right_sidebar_open);
    provide_context(backlinks);
    provide_context(backlinks_loading);

    view! {
        <Stylesheet href="/style.css" />
        <Title text="Quilt" />

        <div class="flex h-screen bg-base text-text overflow-hidden">
            <LeftSidebar open=left_sidebar_open />

            <main class="flex-1 overflow-y-auto px-8 py-4 max-w-4xl mx-auto w-full">
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=JournalView />
                    <Route path=path!("/journal") view=JournalView />
                    <Route path=path!("/journal/:date") view=JournalView />
                    <Route path=path!("/page/:name") view=PageView />
                    <Route path=path!("/pages") view=PagesView />
                    <Route path=path!("/search") view=SearchView />
                </Routes>
            </main>

            <RightSidebar open=right_sidebar_open />
        </div>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <div class="flex items-center justify-center h-full">
            <div class="text-center">
                <h2 class="text-2xl font-bold mb-2">"Page not found"</h2>
                <p class="text-text-muted">"The page you're looking for doesn't exist."</p>
            </div>
        </div>
    }
}
