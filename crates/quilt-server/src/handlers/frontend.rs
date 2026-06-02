//! Frontend asset serving
//!
//! Serves the embedded Leptos SPA from the binary.

use axum::{
    body::Body,
    http::{header, StatusCode},
    response::IntoResponse,
};
use std::path::PathBuf;
use tracing::warn;

/// Get the assets directory path (relative to CARGO_MANIFEST_DIR)
fn assets_dir() -> PathBuf {
    PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("wasm_assets")
}

/// Serve the index.html
pub async fn serve_index_html() -> Response {
    let dir = assets_dir();
    let index_path = dir.join("index.html");

    match std::fs::read(&index_path) {
        Ok(content) => {
            let content_type = "text/html";
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, content_type)],
                Body::from(content),
            )
                .into_response()
        }
        Err(e) => {
            warn!("index.html not found at {:?}: {}", index_path, e);
            // Fallback to placeholder
            placeholder_html().into_response()
        }
    }
}

/// Serve static assets (JS, WASM, CSS, etc.) with SPA fallback.
///
/// If the requested file exists in wasm_assets, serve it.
/// Otherwise, serve index.html so Leptos router can handle client-side routing.
pub async fn serve_assets(axum::extract::Path(path): axum::extract::Path<String>) -> Response {
    let dir = assets_dir();
    let file_path = dir.join(&path);

    // Security: ensure path doesn't escape assets directory
    if !file_path.starts_with(&dir) {
        return (StatusCode::FORBIDDEN, "Path escape attempt").into_response();
    }

    // Check if the file exists and has a static asset extension
    let has_asset_extension = matches!(
        path.rsplit('.').next(),
        Some(
            "wasm"
                | "js"
                | "css"
                | "html"
                | "json"
                | "png"
                | "ico"
                | "svg"
                | "woff"
                | "woff2"
                | "ttf"
        )
    );

    if has_asset_extension {
        // Try to serve the static file
        match std::fs::read(&file_path) {
            Ok(content) => {
                let content_type = match path.rsplit('.').next() {
                    Some("wasm") => "application/wasm",
                    Some("js") => "application/javascript",
                    Some("css") => "text/css",
                    Some("html") => "text/html",
                    Some("json") => "application/json",
                    Some("png") => "image/png",
                    Some("ico") => "image/x-icon",
                    Some("svg") => "image/svg+xml",
                    Some("woff") => "font/woff",
                    Some("woff2") => "font/woff2",
                    Some("ttf") => "font/ttf",
                    _ => "application/octet-stream",
                };

                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, content_type)],
                    Body::from(content),
                )
                    .into_response()
            }
            Err(e) => {
                warn!("Asset not found: {:?}: {}", file_path, e);
                (StatusCode::NOT_FOUND, "Not found").into_response()
            }
        }
    } else {
        // SPA fallback: serve index.html for client-side routing
        serve_index_html().await
    }
}

/// Fallback placeholder HTML when no assets are embedded
fn placeholder_html() -> Response {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Quilt - Knowledge Graph</title>
    <style>
        body {
            font-family: system-ui, -apple-system, sans-serif;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            margin: 0;
            background: #1a1a2e;
            color: #eee;
        }
        .container { text-align: center; }
        h1 { color: #7b68ee; }
        p { color: #888; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Quilt Server</h1>
        <p>API available at /api/v1/*</p>
        <p>WebSocket available at /ws</p>
        <p>Build frontend with: cd crates/quilt-ui && trunk build</p>
    </div>
</body>
</html>"#;

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html")],
        Body::from(html),
    )
        .into_response()
}

use axum::response::Response;
