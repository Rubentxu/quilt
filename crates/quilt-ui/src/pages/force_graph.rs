//! Force-directed graph visualization component
//!
//! Replaces SimpleGraphVisualization with a Canvas-based force-directed layout.

use leptos::callback::Callback;
use leptos::ev::{MouseEvent, WheelEvent};
use leptos::html::Canvas;
use leptos::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::bridge::GraphDataDto;
use crate::pages::canvas_renderer::CanvasRenderer;
use crate::pages::force_simulation::ForceSimulation;

/// Get mouse position relative to the graph canvas element
fn get_mouse_pos_in_canvas(ev: &MouseEvent) -> Option<(f64, f64)> {
    // Use offsetX/offsetY from the mouse event (relative to canvas)
    let x = ev.offset_x();
    let y = ev.offset_y();
    Some((x as f64, y as f64))
}

/// Interactive force-directed graph component
#[component]
pub fn ForceGraph(data: GraphDataDto, on_node_click: Callback<String>) -> impl IntoView {
    // Canvas element reference
    let canvas_ref: NodeRef<Canvas> = NodeRef::new();

    // Simulation state — use RwSignal so we can mutate inside closures
    let simulation = RwSignal::new(Option::<ForceSimulation>::None);

    // Viewport state
    let zoom = RwSignal::new(1.0f64);
    let pan_x = RwSignal::new(0.0f64);
    let pan_y = RwSignal::new(0.0f64);

    // Interaction state
    let hovered_idx = StoredValue::new(Option::<usize>::None);
    let highlight_idx = StoredValue::new(Option::<usize>::None);
    let is_panning = StoredValue::new(false);
    let dragged_idx = StoredValue::new(Option::<usize>::None);
    let last_mouse = StoredValue::new((0.0, 0.0));

    // Filter state
    let filter_pages = RwSignal::new(true);
    let filter_journals = RwSignal::new(true);

    // Search filter state
    let search_query = RwSignal::new(String::new());

    // Tooltip state
    let tooltip = StoredValue::new(Option::<(f64, f64, String)>::None);

    // Build simulation from data
    let build_sim = {
        let data = data.clone();
        move || {
            if data.nodes.is_empty() {
                return;
            }

            let name_to_idx: std::collections::HashMap<String, usize> = data
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| (n.id.clone(), i))
                .collect();

            let mut sources = Vec::new();
            let mut targets = Vec::new();
            for edge in &data.edges {
                if let (Some(&src), Some(&tgt)) =
                    (name_to_idx.get(&edge.source), name_to_idx.get(&edge.target))
                {
                    sources.push(src);
                    targets.push(tgt);
                }
            }

            let ids: Vec<String> = data.nodes.iter().map(|n| n.id.clone()).collect();
            let names: Vec<String> = data.nodes.iter().map(|n| n.name.clone()).collect();
            let journals: Vec<bool> = data.nodes.iter().map(|n| n.journal).collect();

            let mut sim = ForceSimulation::new(ids, names, journals, sources, targets);
            sim.run();
            simulation.set(Some(sim));
        }
    };

    // Initialize
    build_sim();

    // Redraw function
    let redraw = {
        let canvas_ref = canvas_ref.clone();
        let zoom = zoom.clone();
        let pan_x = pan_x.clone();
        let pan_y = pan_y.clone();
        let hovered_idx = hovered_idx.clone();
        let highlight_idx = highlight_idx.clone();
        let filter_pages = filter_pages.clone();
        let filter_journals = filter_journals.clone();
        let search_query = search_query.clone();
        let simulation = simulation.clone();

        move || {
            let canvas: HtmlCanvasElement = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };

            let renderer = match CanvasRenderer::new(canvas) {
                Ok(r) => r,
                Err(_) => return,
            };

            let sim = simulation.get();
            let sim = match sim.as_ref() {
                Some(s) => s,
                None => return,
            };

            let nodes = sim.nodes();
            let edges = sim.edges();
            let hi = highlight_idx.get_value();
            let ho = hovered_idx.get_value();
            let fp = filter_pages.get();
            let fj = filter_journals.get();
            let sq = search_query.get();

            renderer.draw_with_search(
                nodes,
                edges,
                zoom.get(),
                pan_x.get(),
                pan_y.get(),
                hi,
                hi.is_some(),
                ho,
                fj,
                fp,
                &sq,
            );
        }
    };

    // Animation loop — runs continuously for smooth interaction
    let _start_animation = {
        let simulation = simulation.clone();
        let redraw = redraw.clone();

        move || {
            let sim_clone = simulation.clone();
            let redraw_clone = redraw.clone();

            fn anim_loop(sim: RwSignal<Option<ForceSimulation>>, rd: impl Fn() + Clone + 'static) {
                let sim2 = sim.clone();
                let rd2 = rd.clone();
                // Always step simulation for smooth animation
                sim.update(|s| {
                    if let Some(ref mut inner) = *s {
                        // Step the physics simulation
                        if !inner.is_converged() {
                            inner.step();
                        }
                    }
                });
                rd();
                // Continue loop continuously
                request_animation_frame(move || {
                    anim_loop(sim2, rd2);
                });
            }

            request_animation_frame(move || {
                anim_loop(sim_clone, redraw_clone);
            });
        }
    };

    // Zoom via wheel
    let on_wheel = {
        let zoom = zoom.clone();
        let redraw = redraw.clone();

        move |ev: WheelEvent| {
            ev.prevent_default();
            let factor = if ev.delta_y() > 0.0 { 0.92 } else { 1.08 };
            zoom.update(|v| *v = (*v * factor).clamp(0.3, 3.0));
            redraw();
        }
    };

    // Mouse down
    let on_mouse_down = {
        let is_panning = is_panning.clone();
        let last_mouse = last_mouse.clone();
        let dragged_idx = dragged_idx.clone();
        let highlight_idx = highlight_idx.clone();
        let simulation = simulation.clone();
        let zoom = zoom.clone();
        let pan_x = pan_x.clone();
        let pan_y = pan_y.clone();
        let filter_pages = filter_pages.clone();
        let filter_journals = filter_journals.clone();
        let canvas_ref = canvas_ref.clone();
        let redraw = redraw.clone();

        move |ev: MouseEvent| {
            let (x, y) = match get_mouse_pos_in_canvas(&ev) {
                Some(pos) => pos,
                None => return,
            };

            let canvas: HtmlCanvasElement = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };

            let renderer = match CanvasRenderer::new(canvas) {
                Ok(r) => r,
                Err(_) => return,
            };

            let sim = simulation.get();
            let sim = match sim.as_ref() {
                Some(s) => s,
                None => return,
            };

            let hit = renderer.hit_test(
                x,
                y,
                sim.nodes(),
                zoom.get(),
                pan_x.get(),
                pan_y.get(),
                filter_pages.get(),
                filter_journals.get(),
            );

            if let Some(idx) = hit {
                dragged_idx.set_value(Some(idx));
                highlight_idx.set_value(Some(idx));
            } else {
                is_panning.set_value(true);
            }

            last_mouse.set_value((x, y));
            redraw();
        }
    };

    // Mouse move
    let on_mouse_move = {
        let is_panning = is_panning.clone();
        let dragged_idx = dragged_idx.clone();
        let last_mouse = last_mouse.clone();
        let hovered_idx = hovered_idx.clone();
        let pan_x = pan_x.clone();
        let pan_y = pan_y.clone();
        let zoom = zoom.clone();
        let simulation = simulation.clone();
        let filter_pages = filter_pages.clone();
        let filter_journals = filter_journals.clone();
        let tooltip = tooltip.clone();
        let canvas_ref = canvas_ref.clone();
        let redraw = redraw.clone();

        move |ev: MouseEvent| {
            let (x, y) = match get_mouse_pos_in_canvas(&ev) {
                Some(pos) => pos,
                None => return,
            };
            let (lx, ly) = last_mouse.get_value();

            if is_panning.get_value() {
                pan_x.update(|v| *v += x - lx);
                pan_y.update(|v| *v += y - ly);
            } else if let Some(idx) = dragged_idx.get_value() {
                let canvas: HtmlCanvasElement = match canvas_ref.get() {
                    Some(c) => c,
                    None => return,
                };
                let renderer = match CanvasRenderer::new(canvas) {
                    Ok(r) => r,
                    Err(_) => return,
                };
                let (gx, gy) = renderer.screen_to_graph(x, y, zoom.get(), pan_x.get(), pan_y.get());
                simulation.update(|s| {
                    if let Some(ref mut inner) = *s {
                        inner.move_node(idx, gx, gy);
                    }
                });
            } else {
                let canvas: HtmlCanvasElement = match canvas_ref.get() {
                    Some(c) => c,
                    None => return,
                };
                let renderer = match CanvasRenderer::new(canvas) {
                    Ok(r) => r,
                    Err(_) => return,
                };

                let sim = simulation.get();
                if let Some(ref sim) = sim.as_ref() {
                    let hit = renderer.hit_test(
                        x,
                        y,
                        sim.nodes(),
                        zoom.get(),
                        pan_x.get(),
                        pan_y.get(),
                        filter_pages.get(),
                        filter_journals.get(),
                    );
                    hovered_idx.set_value(hit);
                    if let Some(idx) = hit {
                        let node = &sim.nodes()[idx];
                        tooltip.set_value(Some((x, y, node.name.clone())));
                    } else {
                        tooltip.set_value(None);
                    }
                }
            }

            last_mouse.set_value((x, y));
            redraw();
        }
    };

    // Mouse up
    let on_mouse_up = {
        let is_panning = is_panning.clone();
        let dragged_idx = dragged_idx.clone();

        move |_ev: MouseEvent| {
            is_panning.set_value(false);
            dragged_idx.set_value(None);
        }
    };

    // Click
    let on_click = {
        let highlight_idx = highlight_idx.clone();
        let simulation = simulation.clone();
        let zoom = zoom.clone();
        let pan_x = pan_x.clone();
        let pan_y = pan_y.clone();
        let filter_pages = filter_pages.clone();
        let filter_journals = filter_journals.clone();
        let on_node_click = on_node_click.clone();
        let canvas_ref = canvas_ref.clone();
        let redraw = redraw.clone();

        move |ev: MouseEvent| {
            let (x, y) = match get_mouse_pos_in_canvas(&ev) {
                Some(pos) => pos,
                None => return,
            };

            let canvas: HtmlCanvasElement = match canvas_ref.get() {
                Some(c) => c,
                None => return,
            };

            let renderer = match CanvasRenderer::new(canvas) {
                Ok(r) => r,
                Err(_) => return,
            };

            let sim = simulation.get();
            let sim = match sim.as_ref() {
                Some(s) => s,
                None => return,
            };

            if let Some(idx) = renderer.hit_test(
                x,
                y,
                sim.nodes(),
                zoom.get(),
                pan_x.get(),
                pan_y.get(),
                filter_pages.get(),
                filter_journals.get(),
            ) {
                let node = &sim.nodes()[idx];
                on_node_click.run(node.id.clone());
            }

            highlight_idx.set_value(None);
            redraw();
        }
    };

    // Mouse leave
    let on_mouse_leave = {
        let hovered_idx = hovered_idx.clone();
        let tooltip = tooltip.clone();
        let redraw = redraw.clone();

        move |_ev: MouseEvent| {
            hovered_idx.set_value(None);
            tooltip.set_value(None);
            redraw();
        }
    };

    // Zoom controls
    let zoom_in = {
        let zoom = zoom.clone();
        let redraw = redraw.clone();
        move |_| {
            zoom.update(|v| *v = (*v * 1.2).min(3.0));
            redraw();
        }
    };

    let zoom_out = {
        let zoom = zoom.clone();
        let redraw = redraw.clone();
        move |_| {
            zoom.update(|v| *v = (*v * 0.8).max(0.3));
            redraw();
        }
    };

    let zoom_reset = {
        let zoom = zoom.clone();
        let pan_x = pan_x.clone();
        let pan_y = pan_y.clone();
        let redraw = redraw.clone();
        move |_| {
            zoom.set(1.0);
            pan_x.set(0.0);
            pan_y.set(0.0);
            redraw();
        }
    };

    // Filter toggles
    let toggle_pages = {
        let filter_pages = filter_pages.clone();
        let redraw = redraw.clone();
        move |_| {
            filter_pages.update(|v| *v = !*v);
            redraw();
        }
    };

    let toggle_journals = {
        let filter_journals = filter_journals.clone();
        let redraw = redraw.clone();
        move |_| {
            filter_journals.update(|v| *v = !*v);
            redraw();
        }
    };

    // Re-layout
    let re_layout = {
        let simulation = simulation.clone();
        let redraw = redraw.clone();
        move |_| {
            simulation.update(|s| {
                if let Some(ref mut inner) = *s {
                    inner.randomize_positions();
                    inner.run();
                }
            });
            redraw();
        }
    };

    // Start animation immediately (canvas ref will be available when events fire)
    _start_animation();

    view! {
        <div class="force-graph" data-testid="force-graph">
            {/* Controls */}
            <div class="graph-controls" data-testid="graph-controls">
                <div class="control-group">
                    <button
                        class="btn-filter"
                        class:active={filter_pages}
                        data-testid="graph-filter-pages"
                        on:click={toggle_pages}
                    >
                        "Pages"
                    </button>
                    <button
                        class="btn-filter"
                        class:active={filter_journals}
                        data-testid="graph-filter-journals"
                        on:click={toggle_journals}
                    >
                        "Journals"
                    </button>
                </div>

                <input
                    type="text"
                    class="graph-search"
                    placeholder="Buscar nodos..."
                    data-testid="graph-search-input"
                    on:input={move |ev| {
                        let val = event_target_value(&ev);
                        search_query.set(val);
                        redraw();
                    }}
                />

                <div class="control-group">
                    <button class="zoom-btn" data-testid="zoom-in" on:click={zoom_in}>"+"</button>
                    <button class="zoom-btn" data-testid="zoom-reset" on:click={zoom_reset}>"⟲"</button>
                    <button class="zoom-btn" data-testid="zoom-out" on:click={zoom_out}>"−"</button>
                    <button class="btn-secondary" data-testid="re-layout" on:click={re_layout}>"Reset"</button>
                </div>
            </div>

            {/* Canvas */}
            <canvas
                id="graph-canvas"
                class="graph-canvas"
                data-testid="graph-canvas"
                width="800"
                height="600"
                node_ref={canvas_ref}
                on:wheel={on_wheel}
                on:mousedown={on_mouse_down}
                on:mousemove={on_mouse_move}
                on:mouseup={on_mouse_up}
                on:mouseleave={on_mouse_leave}
                on:click={on_click}
            />

            {/* Tooltip */}
            {move || {
                tooltip.get_value().map(|(x, y, name)| {
                    view! {
                        <div
                            class="graph-tooltip"
                            data-testid="graph-tooltip"
                            style:position="absolute"
                            style:left={format!("{}px", x + 10.0)}
                            style:top={format!("{}px", y - 10.0)}
                        >
                            {name}
                        </div>
                    }
                })
            }}

            {/* Legend */}
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
        </div>
    }
}
