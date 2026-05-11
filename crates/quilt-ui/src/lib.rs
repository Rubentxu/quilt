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
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();

    leptos::mount::mount_to_body(|| {
        leptos::view! { <app::App /> }
    });
}
