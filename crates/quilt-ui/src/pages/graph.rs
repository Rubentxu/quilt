//! Graph View — visualize knowledge as a network
//!
//! Shows pages as nodes and references as edges in an interactive graph.

use leptos::callback::Callback;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::{self, BridgeError, GraphDataDto, GraphNodeDto};

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
            <p class="error-message">"Error al cargar el grafo: " {message}</p>
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

// ── Graph Node Card ──────────────────────────────────────────────────

#[component]
fn GraphNodeCard(node: GraphNodeDto, on_click: Callback<String>) -> impl IntoView {
    let node_type_color = if node.journal { "#f59e0b" } else { "#6366f1" };
    let node_type_label = if node.journal { "journal" } else { "page" };
    let testid = format!("graph-node-{}", node.name.to_lowercase().replace(' ', "-"));

    view! {
        <div
            class="graph-node"
            data-testid={testid}
            style:border-left-color={node_type_color}
            on:click={move |_| on_click.run(node.id.clone())}
        >
            <div class="node-header">
                <span class="node-name">{node.name.clone()}</span>
                <span class="node-type" style:color={node_type_color}>
                    {node_type_label}
                </span>
            </div>
        </div>
    }
}

// ── Graph Stats ─────────────────────────────────────────────────────

#[component]
fn GraphStats(data: GraphDataDto) -> impl IntoView {
    view! {
        <div class="graph-stats" data-testid="graph-stats">
            <span class="stat" data-testid="graph-stat-nodes">
                <span class="stat-value">{data.nodes.len()}</span>
                <span class="stat-label">"nodos"</span>
            </span>
            <span class="stat" data-testid="graph-stat-edges">
                <span class="stat-value">{data.edges.len()}</span>
                <span class="stat-label">"aristas"</span>
            </span>
            <span class="stat" data-testid="graph-stat-journals">
                <span class="stat-value">
                    {data.nodes.iter().filter(|n| n.journal).count()}
                </span>
                <span class="stat-label">"journals"</span>
            </span>
        </div>
    }
}

// ── Simple Graph Visualization ──────────────────────────────────────

#[component]
fn SimpleGraphVisualization(
    data: GraphDataDto,
    on_node_click: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="graph-visualization" data-testid="graph-visualization">
            <div class="graph-info">
                <span>"Vista de nodos (simplificada)"</span>
                <span class="hint">"Usa el zoom del navegador para ver mejor"</span>
            </div>
            <div class="nodes-grid" data-testid="graph-nodes-grid">
                {data.nodes.iter().map(|node| {
                    view! {
                        <GraphNodeCard node={node.clone()} on_click={on_node_click.clone()} />
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}

// ── Graph Legend ─────────────────────────────────────────────────────

#[component]
fn GraphLegend() -> impl IntoView {
    view! {
        <div class="graph-legend" data-testid="graph-legend">
            <span class="legend-item">
                <span class="legend-color" style="background: #6366f1"></span>
                "Página"
            </span>
            <span class="legend-item">
                <span class="legend-color" style="background: #f59e0b"></span>
                "Journal"
            </span>
        </div>
    }
}

// ── Graph Content (handles empty/loaded states) ─────────────────────

#[component]
fn GraphContent(data: GraphDataDto, on_node_click: Callback<String>) -> impl IntoView {
    view! {
        <div class="graph-content" data-testid="graph-content">
            <GraphStats data={data.clone()} />
            <GraphLegend />
            <SimpleGraphVisualization data={data} on_node_click={on_node_click} />
        </div>
    }
}

// ── Main Graph View ──────────────────────────────────────────────────

/// Main Graph View component - displays knowledge graph
#[component]
pub fn GraphView() -> impl IntoView {
    // Async action to fetch graph data
    let fetch_graph = Action::new_local(|_: &()| async move {
        bridge::get_graph_data().await
    });

    // Trigger initial fetch
    fetch_graph.dispatch(());

    // Store the retry callback in a StoredValue
    let on_retry = StoredValue::new(Callback::new(move |_| {
        let _ = fetch_graph.dispatch(());
    }));

    // Node click handler - would navigate to page (stub for now)
    let on_node_click = Callback::new(move |node_id: String| {
        log::info!("Node clicked: {}", node_id);
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
                            <GraphContent data={data} on_node_click={on_node_click.clone()} />
                        }
                    }}
                </Show>
            </Show>
        </div>
    }
}
