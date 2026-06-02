//! Integration tests for health check endpoint.
//!
//! Covers: health_check returns 200 with status "ok".

use quilt_server::handlers::health::health_check;

#[tokio::test]
async fn test_health_check_returns_ok() {
    let response = health_check().await;
    assert_eq!(response.status, "ok");
}

#[tokio::test]
async fn test_health_check_has_status_field() {
    let response = health_check().await;
    // The struct must have the 'status' field
    assert!(!response.status.is_empty());
}

#[tokio::test]
async fn test_health_check_is_deterministic() {
    let r1 = health_check().await;
    let r2 = health_check().await;
    assert_eq!(r1.status, r2.status);
}
