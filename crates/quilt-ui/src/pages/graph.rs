//! Graph View — interactive force-directed knowledge graph visualization
//!
//! Renders pages as nodes and references as edges on an HTML5 Canvas.
//! Supports zoom (scroll wheel), pan (drag background), node drag, and
//! click-to-navigate to page detail views.

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

// ── Graph Node Card ──────────────────────────────────────────────────

#[component]
fn GraphNodeCard(node: GraphNodeDto, on_click: Callback<String>) -> impl IntoView {
    let node_type_color = if node.journal { "#f59e0b" } else { "#6366f1" };
    let node_type_label = if node.journal { "journal" } else { "page" };
    let testid = format!("graph-node-{}", node.name.to_lowercase().replace(' ', "-"));

    view! {
        <div
            class="graph-node-card"
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

// ── Simple Graph Grid (interactive, zoomable) ────────────────────────

#[component]
fn SimpleGraphVisualization(
    data: GraphDataDto,
    on_node_click: Callback<String>,
) -> impl IntoView {
    // Zoom state
    let zoom = RwSignal::new(1.0f64);
    let pan_x = RwSignal::new(0f64);
    let pan_y = RwSignal::new(0f64);
    let is_panning = StoredValue::new(false);
    let last_mouse = StoredValue::new((0i32, 0i32));
    let hovered_idx = StoredValue::new(Option::<usize>::None);

    let zoom_in = {
        let z = zoom.clone();
        move |_| z.update(|v| *v = (*v * 1.2).min(3.0))
    };
    let zoom_out = {
        let z = zoom.clone();
        move |_| z.update(|v| *v = (*v * 0.8).max(0.3))
    };
    let zoom_reset = {
        let z = zoom.clone();
        let x = pan_x.clone();
        let y = pan_y.clone();
        move |_| {
            z.set(1.0);
            x.set(0.0);
            y.set(0.0);
        }
    };

    let on_wheel = {
        let z = zoom.clone();
        move |ev: leptos::ev::WheelEvent| {
            ev.prevent_default();
            let factor = if ev.delta_y() > 0.0 { 0.92 } else { 1.08 };
            z.update(|v| *v = (*v * factor).clamp(0.3, 3.0));
        }
    };

    let on_mouse_down = {
        let ip = is_panning.clone();
        let lm = last_mouse.clone();
        move |ev: leptos::ev::MouseEvent| {
            ip.set_value(true);
            lm.set_value((ev.client_x(), ev.client_y()));
        }
    };

    let on_mouse_move = {
        let ip = is_panning.clone();
        let lm = last_mouse.clone();
        let px = pan_x.clone();
        let py = pan_y.clone();
        move |ev: leptos::ev::MouseEvent| {
            let (lx, ly) = lm.get_value();
            let cx = ev.client_x();
            let cy = ev.client_y();
            if ip.get_value() {
                px.update(|v| *v += (cx - lx) as f64);
                py.update(|v| *v += (cy - ly) as f64);
            }
            lm.set_value((cx, cy));
        }
    };

    let on_mouse_up = {
        let ip = is_panning.clone();
        move |_| ip.set_value(false)
    };

    let container_style = {
        let s = zoom.get();
        let tx = pan_x.get();
        let ty = pan_y.get();
        format!(
            "transform: scale({}); transform-origin: center; transition: transform 0.1s ease",
            s
        )
    };

    view! {
        <div
            class="graph-visualization"
            data-testid="graph-visualization"
            on:wheel={on_wheel}
            on:mousedown={on_mouse_down}
            on:mousemove={on_mouse_move}
            on:mouseup={on_mouse_up}
        >
            <div class="graph-zoom-controls">
                <button class="zoom-btn" data-testid="zoom-in" on:click={zoom_in}>"+"</button>
                <button class="zoom-btn" data-testid="zoom-reset" on:click={zoom_reset}>"⟲"</button>
                <button class="zoom-btn" data-testid="zoom-out" on:click={zoom_out}>"−"</button>
            </div>
            <div class="graph-inner" style={container_style}>
                <div class="nodes-grid" data-testid="graph-nodes-grid">
                    {data.nodes.iter().map(|node| {
                        let node = node.clone();
                        let node_id = node.id.clone();
                        let node_name = node.name.clone();
                        let node_journal = node.journal;
                        let testid = format!("graph-node-{}", node_name.to_lowercase().replace(' ', "-"));
                        let color = if node_journal { "#f59e0b" } else { "#6366f1" };
                        view! {
                            <div
                                class="graph-node-card"
                                data-testid={testid}
                                style:border-left-color={color}
                                on:click={move |_| on_node_click.run(node_id.clone())}
                            >
                                <div class="node-header">
                                    <span class="node-name">{node_name}</span>
                                    <span class="node-type" style:color={color}>
                                        {if node_journal { "journal" } else { "page" }}
                                    </span>
                                </div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
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
                <span class="legend-dot" style="background: #6366f1"></span>
                "Página"
            </span>
            <span class="legend-item">
                <span class="legend-dot" style="background: #f59e0b"></span>
                "Journal"
            </span>
            <span class="legend-hint">"Scroll = zoom · Drag = pan · Click nodo = navegar"</span>
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
                            <GraphContent data={data} on_node_click={on_node_click.clone()} />
                        }
                    }}
                </Show>
            </Show>
        </div>
    }
}
