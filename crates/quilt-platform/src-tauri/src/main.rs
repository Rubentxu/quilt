//! Quilt Tauri application entry point
//!
//! This is the main entry point for the Quilt desktop application.

fn main() {
    quilt_desktop::run().expect("error running tauri application");
}
