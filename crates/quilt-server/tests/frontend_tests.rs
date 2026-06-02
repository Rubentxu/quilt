//! Integration tests for frontend asset serving handlers.
//!
//! Covers: serve_index_html (fallback), placeholder_html,
//! content-type headers, and status codes.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use quilt_server::handlers::frontend::serve_index_html;

#[tokio::test]
async fn test_serve_index_html_returns_html_content_type() {
    let response = serve_index_html().await.into_response();
    assert_eq!(response.status(), StatusCode::OK);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.contains("text/html"),
        "expected text/html, got: {}",
        content_type
    );
}

#[tokio::test]
async fn test_serve_index_html_returns_body() {
    let response = serve_index_html().await.into_response();
    let body = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let body_str = String::from_utf8_lossy(&body);

    // Should contain Quilt branding (either from file or placeholder)
    assert!(
        body_str.contains("Quilt"),
        "body should contain 'Quilt', got: {}",
        &body_str[..body_str.len().min(200)]
    );
}

#[tokio::test]
async fn test_serve_index_html_is_deterministic() {
    let r1 = serve_index_html().await.into_response();
    let r2 = serve_index_html().await.into_response();

    assert_eq!(r1.status(), r2.status());
}

#[test]
fn test_placeholder_html_is_valid_html() {
    // Access placeholder_html via serve_index_html (which falls back to it
    // when wasm_assets/index.html doesn't exist in test environment)
    // We verify the response contains valid HTML structure
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = rt.block_on(async {
        serve_index_html().await.into_response()
    });

    let body = rt.block_on(async {
        axum::body::to_bytes(response.into_body(), 4096).await.unwrap()
    });
    let html = String::from_utf8_lossy(&body);

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<html"));
    assert!(html.contains("</html>"));
}
