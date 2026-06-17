//! Integration tests for `GET /api/v1/cognitive/decay` (Decay Monitor).
//!
//! Tests stand up the full Axum router with an in-memory SQLite DB
//! and exercise the handler end-to-end.
//!
//! Contract:
//! 1. 200 with valid auth — returns DecayMonitorResponse shape
//! 2. Empty graph — empty alerts array, zero counts
//! 3. Auth required — 401 without Bearer token
//! 4. Content shape: alerts, totalAlerts, countsBySeverity, generatedAt

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use serde_json::Value;
use std::sync::Once;
use tower::util::ServiceExt;

mod helpers;
use helpers::build_test_app_state;

const TEST_KEY: &str = "test-key-123";

static INIT: Once = Once::new();

fn init_auth() {
    INIT.call_once(|| {
        quilt_server::middleware::auth::init(TEST_KEY.to_string());
    });
}

fn auth_header(mut req: Request<Body>) -> Request<Body> {
    req.headers_mut().insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {TEST_KEY}")).unwrap(),
    );
    req
}

/// Build the app with a fresh in-memory DB.
async fn empty_app() -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    Ok(quilt_server::routes::create_app(state))
}

#[tokio::test]
async fn decay_endpoint_requires_auth() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cognitive/decay")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn decay_endpoint_returns_empty_on_cold_graph() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/decay")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 4096).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // All sections should be empty
    assert!(
        json["alerts"].as_array().unwrap().is_empty(),
        "alerts should be empty on cold graph"
    );
    assert_eq!(
        json["totalAlerts"].as_u64().unwrap(),
        0,
        "totalAlerts should be 0 on cold graph"
    );
    assert_eq!(
        json["countsBySeverity"]["low"].as_u64().unwrap(),
        0,
        "low should be 0"
    );
    assert_eq!(
        json["countsBySeverity"]["medium"].as_u64().unwrap(),
        0,
        "medium should be 0"
    );
    assert_eq!(
        json["countsBySeverity"]["high"].as_u64().unwrap(),
        0,
        "high should be 0"
    );

    // generatedAt should be a valid RFC3339 string
    let generated_at = json["generatedAt"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(generated_at).is_ok(),
        "generatedAt should be valid RFC3339: {}",
        generated_at
    );

    Ok(())
}

#[tokio::test]
async fn decay_endpoint_response_has_correct_top_level_fields() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/decay")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 4096).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // Verify all expected top-level fields are present
    assert!(json.get("alerts").is_some(), "should have alerts");
    assert!(json.get("totalAlerts").is_some(), "should have totalAlerts");
    assert!(
        json.get("countsBySeverity").is_some(),
        "should have countsBySeverity"
    );
    assert!(json.get("generatedAt").is_some(), "should have generatedAt");

    // Verify field types
    assert!(json["alerts"].is_array(), "alerts should be array");
    assert!(json["totalAlerts"].is_u64(), "totalAlerts should be number");
    assert!(
        json["countsBySeverity"].is_object(),
        "countsBySeverity should be object"
    );
    assert!(
        json["generatedAt"].is_string(),
        "generatedAt should be string"
    );

    Ok(())
}
