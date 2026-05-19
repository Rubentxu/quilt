//! Canvas-based graph renderer
//!
//! Renders a ForceSimulation to an HTML5 Canvas using web-sys.
//! Handles zoom/pan transforms, hit testing, and drawing nodes/edges.

#![allow(unused_must_use)]

use crate::pages::force_simulation::{SimEdge, SimNode};
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Renderer for the force-directed graph
#[allow(dead_code)]
pub struct CanvasRenderer {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    width: f64,
    height: f64,
}

impl CanvasRenderer {
    /// Create a new renderer attached to a canvas element
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;

        let ctx = canvas
            .get_context("2d")
            .map_err(|_| "Failed to get 2d context")?
            .ok_or("No 2d context available")?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| "Failed to cast to CanvasRenderingContext2d")?;

        Ok(Self {
            canvas,
            ctx,
            width,
            height,
        })
    }

    /// Resize the canvas (call when container resizes)
    pub fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    /// Clear the canvas
    pub fn clear(&self) {
        self.ctx.clear_rect(0.0, 0.0, self.width, self.height);
    }

    /// Draw with search filtering - filters nodes by name
    #[allow(clippy::too_many_arguments)]
    pub fn draw_with_search(
        &self,
        nodes: &[SimNode],
        edges: &[SimEdge],
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
        highlight_idx: Option<usize>,
        dimmed: bool,
        hover_idx: Option<usize>,
        filter_journals: bool,
        filter_pages: bool,
        search: &str,
    ) {
        if search.is_empty() {
            // No search filter - use regular draw
            self.draw(
                nodes,
                edges,
                zoom,
                pan_x,
                pan_y,
                highlight_idx,
                dimmed,
                hover_idx,
                filter_journals,
                filter_pages,
            );
            return;
        }

        // Filter nodes by search query
        let q = search.to_lowercase();
        let matching_indices: std::collections::HashSet<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.name.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();

        // Draw with search filtering
        self.draw_search(
            nodes,
            edges,
            zoom,
            pan_x,
            pan_y,
            highlight_idx,
            dimmed,
            hover_idx,
            filter_journals,
            filter_pages,
            &matching_indices,
        );
    }

    /// Internal draw with search filtering
    #[allow(clippy::too_many_arguments)]
    fn draw_search(
        &self,
        nodes: &[SimNode],
        edges: &[SimEdge],
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
        highlight_idx: Option<usize>,
        dimmed: bool,
        hover_idx: Option<usize>,
        filter_journals: bool,
        filter_pages: bool,
        matching_indices: &std::collections::HashSet<usize>,
    ) {
        self.clear();

        // Background
        self.ctx.set_fill_style_str("#0f172a");
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);

        // Save context and apply transform
        self.ctx.save();
        self.ctx.translate(pan_x, pan_y);
        self.ctx.scale(zoom, zoom);

        // Build node lookup
        let node_map: std::collections::HashMap<usize, &SimNode> =
            nodes.iter().enumerate().collect();

        // Draw edges (only those connecting visible nodes)
        self.draw_edges_search(
            nodes,
            edges,
            &node_map,
            highlight_idx,
            dimmed,
            filter_journals,
            filter_pages,
            matching_indices,
        );

        // Draw nodes
        for (i, node) in nodes.iter().enumerate() {
            // Apply filters
            if node.journal && !filter_journals {
                continue;
            }
            if !node.journal && !filter_pages {
                continue;
            }

            // Apply search filter - dim non-matching nodes
            let search_match = matching_indices.contains(&i);
            let is_highlighted = highlight_idx == Some(i);
            let is_hovered = hover_idx == Some(i);
            let dim =
                (dimmed && highlight_idx.is_some() && !is_highlighted) || (dimmed && !search_match);

            self.draw_node_search(
                node,
                i,
                is_highlighted,
                is_hovered,
                dim,
                !search_match,
            );
        }

        self.ctx.restore();
    }

    /// Draw a single node with search dimming
    fn draw_node_search(
        &self,
        node: &SimNode,
        _idx: usize,
        highlighted: bool,
        hovered: bool,
        dimmed: bool,
        search_dimmed: bool,
    ) {
        let radius = if search_dimmed {
            node.radius * 0.7
        } else {
            node.radius
        };

        let base_color = if node.journal { "#f59e0b" } else { "#6366f1" };
        let fill_color = if highlighted {
            base_color.to_string()
        } else if dimmed {
            format!(
                "rgba({},{},{},{})",
                if node.journal { 245 } else { 99 },
                if node.journal { 158 } else { 102 },
                if node.journal { 11 } else { 241 },
                0.3
            )
        } else {
            base_color.to_string()
        };

        if highlighted || hovered {
            self.ctx.set_shadow_color(base_color);
            self.ctx.set_shadow_blur(if hovered { 20.0 } else { 15.0 });
        } else {
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_blur(0.0);
        }

        self.ctx.begin_path();
        self.ctx
            .arc(node.x, node.y, radius, 0.0, 2.0 * std::f64::consts::PI);
        self.ctx.set_fill_style_str(&fill_color);
        self.ctx.fill();

        if highlighted {
            self.ctx.set_stroke_style_str("#e0e7ff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke();
        }

        self.ctx.set_shadow_color("transparent");
        self.ctx.set_shadow_blur(0.0);

        if !search_dimmed {
            self.draw_label(node, dimmed);
        }
    }

    /// Draw edges with search filtering
    #[allow(clippy::too_many_arguments)]
    fn draw_edges_search(
        &self,
        _nodes: &[SimNode],
        edges: &[SimEdge],
        node_map: &std::collections::HashMap<usize, &SimNode>,
        highlight_idx: Option<usize>,
        dimmed: bool,
        filter_journals: bool,
        filter_pages: bool,
        matching_indices: &std::collections::HashSet<usize>,
    ) {
        for edge in edges {
            let (Some(src), Some(tgt)) = (
                node_map.get(&edge.source_idx),
                node_map.get(&edge.target_idx),
            ) else {
                continue;
            };

            let src_visible = (src.journal && filter_journals) || (!src.journal && filter_pages);
            let tgt_visible = (tgt.journal && filter_journals) || (!tgt.journal && filter_pages);
            if !src_visible || !tgt_visible {
                continue;
            }

            let src_match = matching_indices.contains(&edge.source_idx);
            let tgt_match = matching_indices.contains(&edge.target_idx);

            let is_highlighted =
                highlight_idx.is_some_and(|hi| hi == edge.source_idx || hi == edge.target_idx);
            let alpha = if dimmed && !is_highlighted {
                0.03
            } else if is_highlighted {
                0.6
            } else if src_match && tgt_match {
                0.15
            } else {
                0.03
            };

            self.draw_edge(src, tgt, alpha, is_highlighted);
        }
    }

    /// Draw the entire graph visualization
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        nodes: &[SimNode],
        edges: &[SimEdge],
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
        highlight_idx: Option<usize>,
        dimmed: bool,
        hover_idx: Option<usize>,
        filter_journals: bool,
        filter_pages: bool,
    ) {
        self.clear();

        // Background
        self.ctx.set_fill_style_str("#0f172a");
        self.ctx.fill_rect(0.0, 0.0, self.width, self.height);

        // Save context and apply transform
        self.ctx.save();
        self.ctx.translate(pan_x, pan_y);
        self.ctx.scale(zoom, zoom);

        // Build node lookup
        let node_map: std::collections::HashMap<usize, &SimNode> =
            nodes.iter().enumerate().collect();

        // Draw edges first (behind nodes)
        self.draw_edges(
            nodes,
            edges,
            &node_map,
            highlight_idx,
            dimmed,
            filter_journals,
            filter_pages,
        );

        // Draw nodes
        for (i, node) in nodes.iter().enumerate() {
            // Apply filter
            if node.journal && !filter_journals {
                continue;
            }
            if !node.journal && !filter_pages {
                continue;
            }

            let is_highlighted = highlight_idx == Some(i);
            let is_hovered = hover_idx == Some(i);
            let dim = dimmed && highlight_idx.is_some() && !is_highlighted;

            self.draw_node(node, i, is_highlighted, is_hovered, dim);
        }

        self.ctx.restore();
    }

    /// Draw edges between nodes
    #[allow(clippy::too_many_arguments)]
    fn draw_edges(
        &self,
        _nodes: &[SimNode],
        edges: &[SimEdge],
        node_map: &std::collections::HashMap<usize, &SimNode>,
        highlight_idx: Option<usize>,
        dimmed: bool,
        filter_journals: bool,
        filter_pages: bool,
    ) {
        for edge in edges {
            let (Some(src), Some(tgt)) = (
                node_map.get(&edge.source_idx),
                node_map.get(&edge.target_idx),
            ) else {
                continue;
            };

            // Filter: skip if both endpoints are filtered
            let src_visible = (src.journal && filter_journals) || (!src.journal && filter_pages);
            let tgt_visible = (tgt.journal && filter_journals) || (!tgt.journal && filter_pages);
            if !src_visible || !tgt_visible {
                continue;
            }

            // Highlight logic
            let is_highlighted =
                highlight_idx.is_some_and(|hi| hi == edge.source_idx || hi == edge.target_idx);
            let alpha = if dimmed && !is_highlighted {
                0.05
            } else if is_highlighted {
                0.6
            } else {
                0.15
            };

            self.draw_edge(src, tgt, alpha, is_highlighted);
        }
    }

    /// Draw a single edge
    fn draw_edge(&self, src: &SimNode, tgt: &SimNode, alpha: f64, highlighted: bool) {
        self.ctx.begin_path();
        self.ctx.move_to(src.x, src.y);
        self.ctx.line_to(tgt.x, tgt.y);

        if highlighted {
            self.ctx.set_stroke_style_str("#818cf8");
            self.ctx.set_line_width(2.0);
        } else {
            let color = format!("rgba(99,102,241,{})", alpha);
            self.ctx.set_stroke_style_str(&color);
            self.ctx.set_line_width(1.0);
        }

        self.ctx.stroke();
    }

    /// Draw a single node
    fn draw_node(
        &self,
        node: &SimNode,
        _idx: usize,
        highlighted: bool,
        hovered: bool,
        dimmed: bool,
    ) {
        let radius = node.radius;

        // Node color
        let base_color = if node.journal { "#f59e0b" } else { "#6366f1" };
        let fill_color = if highlighted {
            base_color.to_string()
        } else if dimmed {
            format!(
                "rgba({},{},{},{})",
                if node.journal { 245 } else { 99 },
                if node.journal { 158 } else { 102 },
                if node.journal { 11 } else { 241 },
                0.3
            )
        } else {
            base_color.to_string()
        };

        // Glow for highlighted/hovered
        if highlighted || hovered {
            self.ctx.set_shadow_color(base_color);
            self.ctx.set_shadow_blur(if hovered { 20.0 } else { 15.0 });
        } else {
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_blur(0.0);
        }

        // Draw circle
        self.ctx.begin_path();
        self.ctx
            .arc(node.x, node.y, radius, 0.0, 2.0 * std::f64::consts::PI);
        self.ctx.set_fill_style_str(&fill_color);
        self.ctx.fill();

        // Border for highlighted
        if highlighted {
            self.ctx.set_stroke_style_str("#e0e7ff");
            self.ctx.set_line_width(2.0);
            self.ctx.stroke();
        }

        // Reset shadow
        self.ctx.set_shadow_color("transparent");
        self.ctx.set_shadow_blur(0.0);

        // Draw label
        self.draw_label(node, dimmed);
    }

    /// Draw node label
    fn draw_label(&self, node: &SimNode, dimmed: bool) {
        let label = &node.name;
        let x = node.x;
        let y = node.y + node.radius + 14.0;

        // Fixed width for label (page names are typically short)
        let text_width = 80.0;
        let padding = 4.0;
        self.ctx.set_fill_style_str("rgba(15, 23, 42, 0.8)");
        self.ctx.fill_rect(
            x - text_width / 2.0 - padding,
            y - 10.0,
            text_width + padding * 2.0,
            14.0,
        );

        // Text
        let alpha = if dimmed { 0.4 } else { 1.0 };
        let color = format!("rgba(226,232,240,{})", alpha);
        self.ctx.set_fill_style_str(&color);
        self.ctx.set_font("12px system-ui, sans-serif");
        self.ctx.set_text_align("center");
        self.ctx.set_text_baseline("top");
        self.ctx.fill_text(label, x, y);
    }

    /// Hit test: convert screen coordinates to find node at position
    #[allow(clippy::too_many_arguments)]
    pub fn hit_test(
        &self,
        screen_x: f64,
        screen_y: f64,
        nodes: &[SimNode],
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
        filter_journals: bool,
        filter_pages: bool,
    ) -> Option<usize> {
        // Convert screen to graph coordinates
        let graph_x = (screen_x - pan_x) / zoom;
        let graph_y = (screen_y - pan_y) / zoom;

        // Check nodes in reverse order (top-most first)
        for (i, node) in nodes.iter().enumerate().rev() {
            // Apply filter
            if node.journal && !filter_journals {
                continue;
            }
            if !node.journal && !filter_pages {
                continue;
            }

            let dx = graph_x - node.x;
            let dy = graph_y - node.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= node.radius {
                return Some(i);
            }
        }

        None
    }

    /// Convert screen coordinates to graph coordinates
    pub fn screen_to_graph(
        &self,
        screen_x: f64,
        screen_y: f64,
        zoom: f64,
        pan_x: f64,
        pan_y: f64,
    ) -> (f64, f64) {
        ((screen_x - pan_x) / zoom, (screen_y - pan_y) / zoom)
    }

    /// Get canvas dimensions
    pub fn dimensions(&self) -> (f64, f64) {
        (self.width, self.height)
    }
}
