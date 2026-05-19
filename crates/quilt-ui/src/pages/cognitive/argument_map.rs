//! Argument Map View — visualize argument structures
//!
//! Renders argument trees showing claim/evidence/rebuttal
//! structure with color-coded edges.

use leptos::callback::Callable;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::bridge::{self, BridgeError};

/// Role of a node in an argument
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArgumentRole {
    Claim,
    Evidence,
    Reasoning,
    Rebuttal,
}

/// An edge in the argument graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentEdgeDto {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub confidence: f32,
}

/// An argument node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentNodeDto {
    pub block_id: String,
    pub role: ArgumentRole,
    pub strength: f32,
    pub content_preview: String,
}

/// Complete argument graph for a page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentGraphDto {
    pub page_id: String,
    pub nodes: Vec<ArgumentNodeDto>,
    pub edges: Vec<ArgumentEdgeDto>,
}

/// Response from the argument_map Tauri command
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArgumentMapResponse {
    #[serde(rename = "available")]
    available: bool,
    #[serde(rename = "message")]
    message: Option<String>,
    #[serde(rename = "graph")]
    graph: Option<ArgumentGraphDto>,
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn node_color(role: &ArgumentRole) -> &'static str {
    match role {
        ArgumentRole::Claim => "#6366f1",
        ArgumentRole::Evidence => "#22c55e",
        ArgumentRole::Reasoning => "#f59e0b",
        ArgumentRole::Rebuttal => "#ef4444",
    }
}

fn edge_color(edge_type: &str) -> &'static str {
    match edge_type {
        "supports" => "green",
        "refutes" => "red",
        "qualifies" => "blue",
        _ => "gray",
    }
}

// ── Loading Skeleton ──────────────────────────────────────────────────────────

#[component]
fn LoadingSkeleton() -> impl IntoView {
    view! {
        <div class="argument-map-loading">
            <div class="skeleton-header"></div>
            <div class="skeleton-nodes">
                <div class="skeleton-node"></div>
                <div class="skeleton-node"></div>
                <div class="skeleton-node"></div>
            </div>
        </div>
    }
}

// ── Error State ───────────────────────────────────────────────────────────────

#[component]
fn MapErrorState(message: String, on_retry: Callback<()>) -> impl IntoView {
    view! {
        <div class="argument-map-error">
            <p class="error-message">"Failed to load argument map: " {message}</p>
            <button class="btn-retry" on:click={move |_| on_retry.run(())}>
                "🔄 Retry"
            </button>
        </div>
    }
}

// ── Argument Node Card ────────────────────────────────────────────────────────

#[component]
fn ArgumentNodeCard(node: ArgumentNodeDto) -> impl IntoView {
    let color = node_color(&node.role);
    view! {
        <div class="argument-node" style:border-left-color={color}>
            <div class="node-header">
                <span class="node-role">{format!("{:?}", node.role)}</span>
                <span class="node-strength">"{(node.strength * 100.0).round()}%"</span>
            </div>
            <div class="node-content">{node.content_preview}</div>
        </div>
    }
}

// ── Argument Edge Row ─────────────────────────────────────────────────────────

#[component]
fn ArgumentEdgeRow(edge: ArgumentEdgeDto) -> impl IntoView {
    let color = edge_color(&edge.edge_type);
    view! {
        <div class="argument-edge" style:color={color}>
            <span class="edge-arrow">"→"</span>
            <span class="edge-type">{edge.edge_type}</span>
            <span class="edge-confidence">"{(edge.confidence * 100.0).round()}%"</span>
        </div>
    }
}

// ── Argument Nodes List ───────────────────────────────────────────────────────

#[component]
fn ArgumentNodesList(nodes: Vec<ArgumentNodeDto>) -> impl IntoView {
    let nodes_for_empty = nodes.clone();
    view! {
        <div class="nodes-list">
            <Show
                when={move || !nodes_for_empty.is_empty()}
                fallback={move || view! {
                    <div class="empty-state">
                        <p>"No argument structure found for this page."</p>
                    </div>
                }}
            >
                <div class="nodes-container">
                    {nodes.iter().map(|n| {
                        view! { <ArgumentNodeCard node={n.clone()} /> }
                    }).collect::<Vec<_>>()}
                </div>
            </Show>
        </div>
    }
}

// ── Argument Edges List ───────────────────────────────────────────────────────

#[component]
fn ArgumentEdgesList(edges: Vec<ArgumentEdgeDto>) -> impl IntoView {
    let edges_for_empty = edges.clone();
    view! {
        <div class="edges-list">
            <Show
                when={move || !edges_for_empty.is_empty()}
                fallback={move || view! { <div></div> }}
            >
                <div class="edges-container">
                    <h4>"Edges"</h4>
                    {edges.iter().map(|e| {
                        view! { <ArgumentEdgeRow edge={e.clone()} /> }
                    }).collect::<Vec<_>>()}
                </div>
            </Show>
        </div>
    }
}

// ── Argument Map View Page ────────────────────────────────────────────────────

/// Argument map view component
#[component]
pub fn ArgumentMapView(page_name: String) -> impl IntoView {
    // Clone page_name before capturing in Fn closure
    let page_name_for_fetch = page_name.clone();

    // Async action to fetch argument map
    let fetch_arguments = Action::new_local(move |_: &()| {
        let name = page_name_for_fetch.clone();
        async move {
            match bridge::get_argument_map(&name).await {
                Ok(json) => match serde_json::from_value::<ArgumentMapResponse>(json.clone()) {
                    Ok(resp) if !resp.available => Err(BridgeError::Unavailable(
                        resp.message
                            .unwrap_or_else(|| "Argument cartographer unavailable".into()),
                    )),
                    Ok(resp) => Ok(resp.graph.unwrap_or(ArgumentGraphDto {
                        page_id: name,
                        nodes: vec![],
                        edges: vec![],
                    })),
                    Err(_) => match serde_json::from_value::<ArgumentGraphDto>(json) {
                        Ok(g) => Ok(g),
                        Err(_) => Ok(ArgumentGraphDto {
                            page_id: name,
                            nodes: vec![],
                            edges: vec![],
                        }),
                    },
                },
                Err(e) => Err(e),
            }
        }
    });

    // Store refresh callback
    let on_refresh = StoredValue::new(Callback::new(move |_| {
        let _ = fetch_arguments.dispatch(());
    }));

    // Trigger initial fetch
    fetch_arguments.dispatch(());

    // Extract reactive values BEFORE the view! macro — dashboard pattern
    let pending = fetch_arguments.pending();
    let value = fetch_arguments.value();
    let cb = on_refresh.get_value();
    let page_name_for_header = page_name.clone();

    view! {
        <div class="argument-map-view">
            <div class="page-header">
                <h2>"🔍 Argument Map: " {page_name_for_header}</h2>
                <p class="page-subtitle">"Argument structure visualization"</p>
            </div>

            <Show
                when={move || !pending.get()}
                fallback={move || view! { <LoadingSkeleton /> }}
            >
                <Show
                    when={move || !matches!(value.get(), Some(Err(_)))}
                    fallback={move || {
                        let msg = match value.get() {
                            Some(Err(BridgeError::TauriError(s))) => s.clone(),
                            Some(Err(BridgeError::JsonError(s))) => s.clone(),
                            Some(Err(BridgeError::Unavailable(s))) => s.clone(),
                            _ => String::new(),
                        };
                        view! { <MapErrorState message={msg} on_retry={cb} /> }
                    }}
                >
                    <div class="argument-tree">
                        <div class="tree-legend">
                            <span class="legend-item">
                                <span class="legend-color" style="background: #6366f1"></span>
                                "Claim"
                            </span>
                            <span class="legend-item">
                                <span class="legend-color" style="background: #22c55e"></span>
                                "Evidence"
                            </span>
                            <span class="legend-item">
                                <span class="legend-color" style="background: #f59e0b"></span>
                                "Reasoning"
                            </span>
                            <span class="legend-item">
                                <span class="legend-color" style="background: #ef4444"></span>
                                "Rebuttal"
                            </span>
                        </div>

                        <ArgumentNodesList
                            nodes={match value.get() {
                                Some(Ok(g)) => g.nodes,
                                _ => vec![],
                            }}
                        />

                        <ArgumentEdgesList
                            edges={match value.get() {
                                Some(Ok(g)) => g.edges,
                                _ => vec![],
                            }}
                        />
                    </div>
                </Show>
            </Show>
        </div>
    }
}
