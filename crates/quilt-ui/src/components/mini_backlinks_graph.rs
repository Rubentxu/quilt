//! Mini graph visualization for backlinks
//!
//! Shows a small graph with the current page at center
//! and its backlink sources as connected nodes.

use leptos::html::Canvas;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::bridge::CognitiveState;
use crate::components::backlinks_panel::Backlink;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniGraphNode {
    pub id: String,
    pub name: String,
    pub is_current: bool,
    pub cognitive_state: Option<CognitiveState>,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniGraphData {
    pub current_page: String,
    pub nodes: Vec<MiniGraphNode>,
}

pub struct MiniBacklinksGraph {
    width: f64,
    height: f64,
    canvas: HtmlCanvasElement,
}

impl MiniBacklinksGraph {
    pub fn new(canvas: HtmlCanvasElement, width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            canvas,
        }
    }

    pub fn render(&self, data: &MiniGraphData) -> Result<(), String> {
        let ctx = self.canvas
            .get_context("2d")
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("No 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        ctx.clear_rect(0.0, 0.0, self.width, self.height);

        if data.nodes.is_empty() {
            return Ok(());
        }

        let center_x = self.width / 2.0;
        let center_y = self.height / 2.0;

        let current_node = data.nodes.iter().find(|n| n.is_current);
        let backlink_nodes: Vec<_> = data.nodes.iter().filter(|n| !n.is_current).collect();

        if let Some(current) = current_node {
            ctx.begin_path();
            let _ = ctx.arc(center_x, center_y, 14.0, 0.0, 2.0 * std::f64::consts::PI);
            let _ = ctx.set_fill_style_str("#6366f1");
            let _ = ctx.fill();

            ctx.begin_path();
            let _ = ctx.arc(center_x, center_y, 18.0, 0.0, 2.0 * std::f64::consts::PI);
            let _ = ctx.set_stroke_style_str("#6366f1");
            let _ = ctx.set_line_width(2.0);
            let _ = ctx.stroke();
        }

        let num_backlinks = backlink_nodes.len();
        for (i, node) in backlink_nodes.iter().enumerate() {
            let angle = if num_backlinks == 1 {
                0.0
            } else {
                (i as f64 / num_backlinks as f64) * 2.0 * std::f64::consts::PI - std::f64::consts::FRAC_PI_2
            };
            let radius = 50.0;
            let x = center_x + angle.cos() * radius;
            let y = center_y + angle.sin() * radius;

            let _ = ctx.begin_path();
            let _ = ctx.move_to(center_x, center_y);
            let _ = ctx.line_to(x, y);
            let _ = ctx.set_stroke_style_str("rgba(99, 102, 241, 0.4)");
            let _ = ctx.set_line_width(1.5);
            let _ = ctx.stroke();

            let color = match &node.cognitive_state {
                Some(CognitiveState::Active) => "#22c55e",
                Some(CognitiveState::Exploring) => "#3b82f6",
                Some(CognitiveState::Stale) => "#f59e0b",
                Some(CognitiveState::Archived) => "#6b7280",
                None => "#8b5cf6",
            };

            ctx.begin_path();
            let _ = ctx.arc(x, y, 8.0, 0.0, 2.0 * std::f64::consts::PI);
            let _ = ctx.set_fill_style_str(color);
            let _ = ctx.fill();
        }

        Ok(())
    }
}

#[component]
pub fn MiniBacklinksGraphView(
    backlinks: Vec<Backlink>,
    current_page: String,
) -> impl IntoView {
    let canvas_ref: NodeRef<Canvas> = NodeRef::new();
    let width = 280.0;
    let height = 180.0;

    let mini_data = Signal::derive(move || {
        let mut nodes: Vec<MiniGraphNode> = vec![
            MiniGraphNode {
                id: current_page.clone(),
                name: current_page.clone(),
                is_current: true,
                cognitive_state: None,
                x: 0.0,
                y: 0.0,
            }
        ];

        let mut seen_ids = std::collections::HashSet::new();
        seen_ids.insert(current_page.clone());

        for backlink in &backlinks {
            if seen_ids.insert(backlink.source_id.clone()) {
                nodes.push(MiniGraphNode {
                    id: backlink.source_id.clone(),
                    name: backlink.source_title.clone(),
                    is_current: false,
                    cognitive_state: None,
                    x: 0.0,
                    y: 0.0,
                });
            }
        }

        MiniGraphData {
            current_page: current_page.clone(),
            nodes,
        }
    });

    Effect::new(move || {
        if let Some(canvas) = canvas_ref.get() {
            let element: HtmlCanvasElement = canvas.into();
            let graph = MiniBacklinksGraph::new(element, width, height);
            let _ = graph.render(&mini_data.get());
        }
    });

    view! {
        <div class="mini-backlinks-graph">
            <canvas
                node_ref={canvas_ref}
                width={width as u32}
                height={height as u32}
                class="mini-graph-canvas"
            />
        </div>
    }
}
