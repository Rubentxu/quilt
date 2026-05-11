//! Argument Map View — visualize argument structures
//!
//! Renders argument trees showing claim/evidence/rebuttal
//! structure with color-coded edges.

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

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

/// Argument map view component
#[component]
pub fn ArgumentMapView(_page_name: String) -> impl IntoView {
    // Mock data for development
    let mock_nodes = [
        ArgumentNodeDto {
            block_id: "node-1".to_string(),
            role: ArgumentRole::Claim,
            strength: 0.9,
            content_preview: "Rust is safer than C++".to_string(),
        },
        ArgumentNodeDto {
            block_id: "node-2".to_string(),
            role: ArgumentRole::Evidence,
            strength: 0.85,
            content_preview: "Memory safety guarantees via ownership".to_string(),
        },
        ArgumentNodeDto {
            block_id: "node-3".to_string(),
            role: ArgumentRole::Rebuttal,
            strength: 0.6,
            content_preview: "But Rust has a steeper learning curve".to_string(),
        },
    ];

    let mock_edges = [
        ArgumentEdgeDto {
            source: "node-2".to_string(),
            target: "node-1".to_string(),
            edge_type: "supports".to_string(),
            confidence: 0.85,
        },
        ArgumentEdgeDto {
            source: "node-3".to_string(),
            target: "node-1".to_string(),
            edge_type: "refutes".to_string(),
            confidence: 0.6,
        },
    ];

    let edge_color = |edge_type: &str| -> &'static str {
        match edge_type {
            "supports" => "green",
            "refutes" => "red",
            "qualifies" => "blue",
            _ => "gray",
        }
    };

    let node_color = |role: &ArgumentRole| -> &'static str {
        match role {
            ArgumentRole::Claim => "#6366f1",
            ArgumentRole::Evidence => "#22c55e",
            ArgumentRole::Reasoning => "#f59e0b",
            ArgumentRole::Rebuttal => "#ef4444",
        }
    };

    view! {
        <div class="argument-map-view">
            <div class="page-header">
                <h2>"🔍 Argument Map: {page_name}"</h2>
                <p class="page-subtitle">"Argument structure visualization"</p>
            </div>

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
                        <span class="legend-color" style="background: #ef4444"></span>
                        "Rebuttal"
                    </span>
                </div>

                <div class="nodes-list">
                    {mock_nodes.iter().map(|node| {
                        let color = node_color(&node.role);
                        view! {
                            <div class="argument-node" style:border-left-color={color}>
                                <div class="node-header">
                                    <span class="node-role">{format!("{:?}", node.role)}</span>
                                    <span class="node-strength">"{(node.strength * 100.0).round()}%"</span>
                                </div>
                                <div class="node-content">{node.content_preview.clone()}</div>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>

                <div class="edges-list">
                    {mock_edges.iter().map(|edge| {
                        let color = edge_color(&edge.edge_type);
                        view! {
                            <div class="argument-edge" style:color={color}>
                                <span class="edge-arrow">"→"</span>
                                <span class="edge-type">{edge.edge_type.clone()}</span>
                                <span class="edge-confidence">"{(edge.confidence * 100.0).round()}%"</span>
                            </div>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }
}
