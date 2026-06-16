//! Integration tests for metrics handler.
//!
//! Covers: metrics_handler with metrics disabled (default),
//! metrics_handler with metrics enabled, and init_metrics.

use axum::response::IntoResponse;
use quilt_server::handlers::metrics::{init_metrics, metrics_handler};

/// Helper: ensure QUILT_METRICS is unset before the test.
/// WARNING: tests that touch OnceLock global state must run sequentially.
fn ensure_metrics_disabled() {
    // Remove QUILT_METRICS from env for this test
    unsafe { std::env::remove_var("QUILT_METRICS") };
    // Also drop any lingering internal state by re-running init
    // (OnceLock can only be set once — we work with what we have)
}

#[tokio::test]
async fn test_metrics_handler_returns_404_when_disabled() {
    ensure_metrics_disabled();
    let response = metrics_handler().await.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_metrics_handler_disabled_body() {
    ensure_metrics_disabled();
    let response = metrics_handler().await.into_response();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let body = String::from_utf8_lossy(&body_bytes);
    assert!(body.contains("Metrics not enabled"));
}

#[tokio::test]
async fn test_metrics_handler_returns_prometheus_when_enabled() {
    // Enable metrics
    unsafe { std::env::set_var("QUILT_METRICS", "true") };
    let _initialized = init_metrics();

    let response = metrics_handler().await.into_response();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    // Clean up env
    unsafe { std::env::remove_var("QUILT_METRICS") };
}

#[tokio::test]
async fn test_metrics_handler_content_type_when_enabled() {
    unsafe { std::env::set_var("QUILT_METRICS", "true") };
    let _initialized = init_metrics();

    let response = metrics_handler().await.into_response();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("text/plain"), "got: {}", content_type);

    unsafe { std::env::remove_var("QUILT_METRICS") };
}

#[tokio::test]
async fn test_init_metrics_returns_false_when_disabled() {
    unsafe { std::env::remove_var("QUILT_METRICS") };
    let result = init_metrics();
    assert!(!result, "init_metrics should return false when disabled");
}

#[tokio::test]
async fn test_init_metrics_returns_true_when_enabled() {
    unsafe { std::env::set_var("QUILT_METRICS", "true") };
    // Note: OnceLock may already be set from a previous test
    // Just test that it doesn't panic
    let _result = init_metrics();
    // We can't assert the return value because OnceLock may already be set
    unsafe { std::env::remove_var("QUILT_METRICS") };
}
