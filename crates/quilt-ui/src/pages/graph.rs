//! Graph View — interactive force-directed knowledge graph visualization
//!
//! Renders pages as nodes and references as edges on an HTML5 Canvas.
//! Supports zoom (scroll wheel), pan (drag background), node drag, and
//! click-to-navigate to page detail views.

use leptos::callback::Callback;
use leptos::prelude::*;

use crate::bridge::{self, GraphDataDto};
use crate::pages::force_graph::ForceGraph;

// ── Loading State ────────────────────────────────────────────────────

#[component]
fn LoadingState() -> impl IntoView {
    view! {
        <div class="graph-loading" data-testid="graph-loading">
            <div class="spinner"></div>
            <p>"Cargando grafo..."</p>
        </div>
    }
}

// ── Error State ─────────────────────────────────────────────────────

#[component]
fn ErrorState(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="graph-error" data-testid="graph-error">
            <p class="error-message" data-testid="graph-error-message">"Error al cargar el grafo: " {message}</p>
            <button class="btn-retry" data-testid="graph-retry-button" on:click={move |_| on_retry.run(())}>
                "Reintentar"
            </button>
        </div>
    }
}

// ── Empty State ─────────────────────────────────────────────────────

#[component]
fn EmptyState() -> impl IntoView {
    view! {
        <div class="graph-empty" data-testid="graph-empty">
            <p>"No hay datos para mostrar."</p>
            <p class="subtitle">"Crea algunas páginas para ver el grafo."</p>
        </div>
    }
}

// ── Main Graph View ──────────────────────────────────────────────────

/// Main Graph View component - displays knowledge graph
#[component]
pub fn GraphView() -> impl IntoView {
    // Async action to fetch graph data
    let fetch_graph = Action::new_local(|_: &()| async move { bridge::get_graph_data().await });

    // Trigger initial fetch
    fetch_graph.dispatch(());

    // Retry handler
    let on_retry = StoredValue::new(Callback::new(move |_| {
        let _ = fetch_graph.dispatch(());
    }));

    // Node click handler - navigate to page view
    let on_node_click = Callback::new(move |node_id: String| {
        let url = format!("/pages?name={}", node_id.replace(' ', "%20"));
        log::info!("Node clicked: {} — navigating to {}", node_id, url);
        if let Some(window) = web_sys::window() {
            if let Err(e) = window.location().set_href(&url) {
                log::error!("Failed to navigate: {:?}", e);
            }
        }
    });

    view! {
        <div class="graph-view" data-testid="graph-view">
            <div class="graph-header">
                <h2 data-testid="graph-title">"Vista Grafo"</h2>
                <p class="page-subtitle">"Navega tu conocimiento como una red"</p>
            </div>

            <Show
                when={move || !fetch_graph.pending().get()}
                fallback={move || view! { <LoadingState /> }}
            >
                <Show
                    when={move || {
                        fetch_graph.value().get()
                            .is_some_and(|r| r.is_ok() && !r.as_ref().ok().map(|d| d.nodes.is_empty()).unwrap_or(true))
                    }}
                    fallback={move || {
                        view! {
                            <Show
                                when={move || {
                                    fetch_graph.value().get()
                                        .is_some_and(|r| r.is_err())
                                }}
                                fallback={move || view! { <EmptyState /> }}
                            >
                                <ErrorState
                                    message={"Error loading graph".to_string()}
                                    on_retry={on_retry.get_value()}
                                />
                            </Show>
                        }
                    }}
                >
                    {move || {
                        let data = fetch_graph.value().get()
                            .and_then(|r| r.ok())
                            .unwrap_or_else(|| GraphDataDto {
                                nodes: vec![],
                                edges: vec![],
                                last_updated: String::new(),
                            });
                        view! {
                            <ForceGraph data={data} on_node_click={on_node_click} />
                        }
                    }}
                </Show>
            </Show>
        </div>
    }
}
