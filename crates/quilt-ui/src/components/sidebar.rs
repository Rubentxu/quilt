use leptos::prelude::*;
use leptos_router::components::*;

#[component]
pub fn LeftSidebar(#[prop(into)] open: Signal<bool>) -> impl IntoView {
    let set_open = use_context::<WriteSignal<bool>>().unwrap_or_else(|| {
        let (_, w) = signal(true);
        w
    });

    view! {
        <aside class=move || {
            if open.get() { "w-60 border-r border-border bg-sidebar flex flex-col shrink-0 transition-all duration-200" }
            else { "w-0 overflow-hidden transition-all duration-200" }
        }>
            <div class="p-4 border-b border-border flex items-center justify-between">
                <h1 class="text-lg font-semibold">"Quilt"</h1>
                <button
                    class="text-text-muted hover:text-text p-1"
                    on:click=move |_| set_open.set(false)
                >
                    "«"
                </button>
            </div>

            <nav class="flex-1 overflow-y-auto p-2">
                <SidebarSection title="Navigation">
                    <SidebarLink href="/journal" icon="📅" label="Journals" />
                    <SidebarLink href="/pages" icon="📄" label="All Pages" />
                    <SidebarLink href="/search" icon="🔍" label="Search" />
                </SidebarSection>

                <SidebarSection title="Favorites">
                    <p class="text-xs text-text-muted px-3 py-2">"No favorites yet"</p>
                </SidebarSection>

                <SidebarSection title="Recent Pages">
                    <p class="text-xs text-text-muted px-3 py-2">"No recent pages"</p>
                </SidebarSection>
            </nav>
        </aside>

        <Show when=move || !open.get()>
            <button
                class="fixed top-2 left-2 z-50 bg-surface hover:bg-surface-hover p-2 rounded border border-border text-text-muted hover:text-text"
                on:click=move |_| set_open.set(true)
            >
                "»"
            </button>
        </Show>
    }
}

#[component]
fn SidebarSection(children: Children, title: &'static str) -> impl IntoView {
    view! {
        <div class="mb-3">
            <h3 class="text-xs font-semibold uppercase tracking-wider text-text-muted px-3 py-1">
                {title}
            </h3>
            <div>{children()}</div>
        </div>
    }
}

#[component]
fn SidebarLink(
    #[prop(into)] href: String,
    #[prop(into)] icon: String,
    #[prop(into)] label: String,
) -> impl IntoView {
    view! {
        <A href=href>
            <div class="flex items-center gap-2 px-3 py-1.5 rounded hover:bg-surface-hover text-sm transition-colors">
                <span>{icon}</span>
                <span class="flex-1">{label}</span>
            </div>
        </A>
    }
}
