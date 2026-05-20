//! Build script for quilt-server
//!
//! Watches for changes in the frontend assets and triggers rebuilds.

use std::path::Path;

fn main() {
    // Tell Cargo to rerun this build script if the UI dist changes
    let ui_dist = Path::new("crates/quilt-ui/dist");
    if ui_dist.exists() {
        println!("cargo:rerun-if-changed={}", ui_dist.display());
    }

    // Also watch the wasm_assets directory
    let wasm_assets = Path::new("wasm_assets");
    if wasm_assets.exists() {
        println!("cargo:rerun-if-changed={}", wasm_assets.display());
    }
}
