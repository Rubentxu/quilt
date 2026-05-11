//! Tauri desktop shell
//!
//! This module provides the Tauri 2 desktop application shell.
//!
//! The actual Tauri application is in `src-tauri/` directory.
//! This module delegates to the real implementation.

/// Entry point for Tauri application
///
/// This delegates to the real implementation in `quilt-tauri` crate.
pub fn run() {
    // The real Tauri app is in the src-tauri/ directory
    // which compiles to the `quilt` binary
    println!("Tauri shell is now in src-tauri/");
    println!("Run `cargo tauri dev` from the src-tauri/ directory");
}
