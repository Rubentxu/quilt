//! Graph Switcher Component for multi-graph support
//!
//! Allows users to switch between different .quilt graph files.

use crate::bridge::{get_current_graph, open_graph};
use crate::state::use_app_state;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Graph Switcher Component
#[component]
pub fn GraphSwitcher() -> impl IntoView {
    let app_state = use_app_state();
    let current_graph = move || app_state.current_graph.get();
    let is_loading = RwSignal::new(false);

    // Load current graph on mount
    spawn_local({
        let app_state = app_state.clone();
        async move {
            match get_current_graph().await {
                Ok(Some(info)) => {
                    app_state.set_current_graph(info.path, info.name);
                }
                Ok(None) => {
                    app_state.clear_current_graph();
                }
                Err(e) => {
                    tracing::warn!("Failed to load current graph: {}", e);
                }
            }
        }
    });

    let _handle_open_graph = move |path: String, name: String| {
        spawn_local({
            let app_state = app_state.clone();
            let is_loading = is_loading;
            async move {
                is_loading.set(true);
                match open_graph(&path).await {
                    Ok(_) => {
                        app_state.set_current_graph(path, name);
                    }
                    Err(e) => {
                        tracing::error!("Failed to open graph: {}", e);
                    }
                }
                is_loading.set(false);
            }
        });
    };

    view! {
        <div class="graph-switcher">
            <button class="graph-name" disabled={is_loading.get()}>
                {move || current_graph().map(|g| g.name).unwrap_or_else(|| "No graph".into())}
            </button>
        </div>
    }
}
