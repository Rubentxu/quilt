//! Quilt UI — Leptos 0.8 WASM frontend
//!
//! Logseq-like PKM interface that communicates with the Quilt backend
//! via HTTP API (the MCP server exposes a REST layer for the UI).

pub mod app;
pub mod bridge;
pub mod components;
pub mod editor;
pub mod outliner;
pub mod pages;
pub mod parser;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    leptos::mount::mount_to_body(|| {
        leptos::view! { <app::App /> }
    });
}
