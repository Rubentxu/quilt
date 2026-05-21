//! Right sidebar component for block context, properties, and annotations
//!
//! Shows contextual information for the currently selected block:
//! - Properties panel (custom key-value metadata)
//! - Backlinks panel (pages that reference current page)
//! - Annotations overlay (highlights, comments, questions)

use leptos::prelude::*;

use crate::components::annotations_overlay::{Annotation, AnnotationsOverlay};
use crate::components::backlinks_panel::{Backlink, BacklinksPanel};
use crate::components::properties_editor::{PropertiesEditor, Property};

/// Tab selection for right sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarTab {
    Properties,
    Backlinks,
    Annotations,
}

impl SidebarTab {
    pub fn label(&self) -> &'static str {
        match self {
            SidebarTab::Properties => "Properties",
            SidebarTab::Backlinks => "Backlinks",
            SidebarTab::Annotations => "Annotations",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SidebarTab::Properties => "🏷️",
            SidebarTab::Backlinks => "🔗",
            SidebarTab::Annotations => "💬",
        }
    }
}

/// Right sidebar component with tabbed interface
#[component]
pub fn RightSidebar(
    is_open: RwSignal<bool>,
    selected_tab: RwSignal<SidebarTab>,
    // Block info
    _block_id: String,
    block_properties: Vec<Property>,
    // Backlinks
    backlinks: Vec<Backlink>,
    current_page_title: String,
    // Annotations
    annotations: Vec<Annotation>,
    // Callbacks
    on_properties_update: Callback<Vec<Property>, ()>,
    on_annotation_resolve: Callback<String, ()>,
    on_annotation_delete: Callback<String, ()>,
) -> impl IntoView {
    // Convert props to signals for reactive access
    let properties_sig = Signal::derive(move || block_properties.clone());
    let backlinks_sig = Signal::derive(move || backlinks.clone());
    let annotations_sig = Signal::derive(move || annotations.clone());
    let page_title_sig = Signal::derive(move || current_page_title.clone());

    // Store signals in local variables for reactive access
    let is_open_sig = is_open;
    let selected_tab_sig = selected_tab;

    view! {
        <Show when={move || is_open_sig.get()}>
            <aside class="right-sidebar">
                <div class="right-sidebar-header">
                    <div class="right-sidebar-tabs">
                            <button
                                class="right-sidebar-tab"
                                class:active={move || selected_tab_sig.get() == SidebarTab::Properties}
                                on:click={move |_| selected_tab_sig.set(SidebarTab::Properties)}
                                data-testid="tab-properties"
                            >
                                {SidebarTab::Properties.icon()}
                            </button>
                            <button
                                class="right-sidebar-tab"
                                class:active={move || selected_tab_sig.get() == SidebarTab::Backlinks}
                                on:click={move |_| selected_tab_sig.set(SidebarTab::Backlinks)}
                                data-testid="tab-backlinks"
                            >
                                {SidebarTab::Backlinks.icon()}
                            </button>
                            <button
                                class="right-sidebar-tab"
                                class:active={move || selected_tab_sig.get() == SidebarTab::Annotations}
                                on:click={move |_| selected_tab_sig.set(SidebarTab::Annotations)}
                                data-testid="tab-annotations"
                            >
                                {SidebarTab::Annotations.icon()}
                            </button>
                    </div>
                    <button
                        class="right-sidebar-close"
                        on:click={move |_| is_open_sig.set(false)}
                    >
                        "×"
                    </button>
                </div>

                <div class="right-sidebar-content">
                    <Show when={move || selected_tab_sig.get() == SidebarTab::Properties}>
                        <div class="right-sidebar-panel">
                            <PropertiesEditor
                                properties={properties_sig.get()}
                                on_update={on_properties_update}
                            />
                        </div>
                    </Show>

                    <Show when={move || selected_tab_sig.get() == SidebarTab::Backlinks}>
                        <div class="right-sidebar-panel">
                            <BacklinksPanel
                                backlinks={backlinks_sig.get()}
                                current_page={page_title_sig.get()}
                            />
                        </div>
                    </Show>

                    <Show when={move || selected_tab_sig.get() == SidebarTab::Annotations}>
                        <div class="right-sidebar-panel">
                            <AnnotationsOverlay
                                annotations={annotations_sig.get()}
                                is_visible={true}
                                on_resolve={on_annotation_resolve}
                                on_delete={on_annotation_delete}
                                on_create={None}
                            />
                        </div>
                    </Show>
                </div>
            </aside>
        </Show>
    }
}

/// Compact toggle button for right sidebar (shown when sidebar is closed)
#[component]
pub fn RightSidebarToggle(
    is_open: RwSignal<bool>,
    selected_tab: RwSignal<SidebarTab>,
) -> impl IntoView {
    // Store signals in local variables for reactive access
    let is_open_sig = is_open;
    let selected_tab_sig = selected_tab;

    view! {
        <Show when={move || !is_open_sig.get()}>
            <div class="right-sidebar-toggle">
                <button
                    class="right-sidebar-toggle-btn"
                    on:click={move |_| is_open_sig.set(true)}
                    title="Open sidebar"
                >
                    {move || selected_tab_sig.get().icon()}
                </button>
            </div>
        </Show>
    }
}
