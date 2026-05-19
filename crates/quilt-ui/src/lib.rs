//! Quilt UI — Leptos-based frontend
//!
//! This crate provides the WASM frontend for Quilt using Leptos 0.7.
//! It communicates with the backend via Tauri IPC commands.

pub mod app;
pub mod bridge;
pub mod components;
pub mod pages;
pub mod wasm;

use wasm_bindgen::prelude::*;

/// Entry point called from JavaScript
#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook FIRST to catch any panics
    console_error_panic_hook::set_once();

    // Initialize logging
    if let Err(e) = console_log::init_with_level(log::Level::Debug) {
        web_sys::console::error_1(&format!("Failed to init logger: {}", e).into());
    }

    web_sys::console::log_1(&"Quilt UI starting...".into());

    // Get the app element
    let app_element = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("app"));

    match app_element {
        Some(element) => {
            web_sys::console::log_1(
                &format!("Found #app element: {:?}", element.tag_name()).into(),
            );

            // Convert Element to HtmlElement for mount_to
            match element.dyn_into::<web_sys::HtmlElement>() {
                Ok(html_element) => {
                    // Mount to the element instead of body for Tauri
                    let handle = leptos::mount::mount_to(html_element, || {
                        leptos::view! { <app::App /> }
                    });

                    web_sys::console::log_1(&"Quilt UI mounted successfully!".into());

                    // Forget the handle so the view stays mounted
                    handle.forget();
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("Could not convert element to HtmlElement: {:?}", e).into(),
                    );
                }
            }
        }
        None => {
            web_sys::console::error_1(&"#app element not found! Falling back to body.".into());
            // Fallback to body mount
            leptos::mount::mount_to_body(|| {
                leptos::view! { <app::App /> }
            });
        }
    }
}
