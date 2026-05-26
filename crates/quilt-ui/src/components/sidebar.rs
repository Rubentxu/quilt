use chrono::{Datelike, Duration, NaiveDate, Utc};
use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::hooks::use_navigate;
use std::sync::Arc;

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

            <nav class="flex-1 overflow-y-auto p-2 space-y-2">
                <CalendarWidget />

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

/// Minimal calendar widget showing a month grid with day navigation.
#[component]
fn CalendarWidget() -> impl IntoView {
    let today = Utc::now().date_naive();
    let (view_date, set_view_date) = signal(today);
    let navigate = use_navigate();

    // Wrap navigate in Arc so it can be shared across multiple closures in the view.
    let nav = Arc::new(navigate);

    // Ensure view_date always points to the first of its month.
    let view_month = Signal::derive(move || {
        let d = view_date.get();
        NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d)
    });

    // Build the month grid as rows of weeks.
    let weeks = Signal::derive(move || {
        let first = view_month.get();
        let year = first.year();
        let month = first.month();
        let last = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        }
        .pred_opt()
        .unwrap();

        let start_wd = first.weekday().num_days_from_monday();

        let mut rows: Vec<Vec<Option<NaiveDate>>> = Vec::new();
        let mut week: Vec<Option<NaiveDate>> = Vec::new();

        for _ in 0..start_wd {
            week.push(None);
        }

        let mut d = first;
        while d <= last {
            week.push(Some(d));
            if week.len() == 7 {
                rows.push(week);
                week = Vec::new();
            }
            d += Duration::days(1);
        }

        while week.len() < 7 {
            week.push(None);
        }
        if !week.is_empty() {
            rows.push(week);
        }

        rows
    });

    // Pre-compute the "today" button click handler.
    let on_today = move |_| set_view_date.set(today);

    let day_labels = ["M", "T", "W", "T", "F", "S", "S"];

    view! {
        <div class="calendar-widget bg-surface rounded-lg border border-border p-2 mb-2">
            // ── Month header ──
            <div class="flex items-center justify-between mb-2">
                <button
                    class="text-text-muted hover:text-text p-1 text-xs"
                    on:click={
                        let set_view_date = set_view_date.clone();
                        move |_| {
                            let cur = view_month.get();
                            let (y, m) = if cur.month() == 1 {
                                (cur.year() - 1, 12u32)
                            } else {
                                (cur.year(), cur.month() - 1)
                            };
                            set_view_date.set(NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(cur));
                        }
                    }
                    title="Previous month"
                >
                    "◀"
                </button>
                <span class="text-xs font-semibold">{move || view_month.get().format("%B %Y").to_string()}</span>
                <button
                    class="text-text-muted hover:text-text p-1 text-xs"
                    on:click={
                        let set_view_date = set_view_date.clone();
                        move |_| {
                            let cur = view_month.get();
                            let (y, m) = if cur.month() == 12 {
                                (cur.year() + 1, 1u32)
                            } else {
                                (cur.year(), cur.month() + 1)
                            };
                            set_view_date.set(NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(cur));
                        }
                    }
                    title="Next month"
                >
                    "▶"
                </button>
            </div>

            // ── Day-of-week header ──
            <div class="grid grid-cols-7 mb-1">
                {day_labels.iter().map(|d| {
                    view! { <div class="text-center text-text-muted text-[10px] font-medium p-0.5">{d.to_string()}</div> }
                }).collect::<Vec<_>>()}
            </div>

            // ── Day grid ──
            {move || weeks.get().into_iter().map(|row| {
                let nav = nav.clone();
                view! {
                    <div class="grid grid-cols-7">
                        {row.into_iter().map(|cell| {
                            match cell {
                                None => Either::Left(view! { <div class="p-0.5"></div> }),
                                Some(d) => {
                                    let is_today = d == today;
                                    let is_current_month = d.month() == view_month.get().month();
                                    let date_path = format!("/journal/{}", d.format("%Y-%m-%d"));
                                    let day_num = format!("{}", d.day());
                                    let btn_class = format!(
                                        "text-center p-0.5 text-xs rounded {} {} {}",
                                        if is_today { "bg-accent text-white font-bold" } else { "hover:bg-surface-hover" },
                                        if is_current_month { "text-text" } else { "text-text-muted" },
                                        "transition-colors",
                                    );
                                    let nav = nav.clone();
                                    let path = date_path.clone();
                                    Either::Right(view! {
                                        <button
                                            class=btn_class
                                            title=date_path
                                            on:click=move |_| nav(&path, Default::default())
                                        >
                                            {day_num}
                                        </button>
                                    })
                                }
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }
            }).collect::<Vec<_>>()}

            // ── Today button ──
            <div class="mt-2 text-center">
                <button
                    class="text-xs text-accent hover:underline"
                    on:click=on_today
                >
                    {format!("Today {}", today.format("%b %-d"))}
                </button>
            </div>
        </div>
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
