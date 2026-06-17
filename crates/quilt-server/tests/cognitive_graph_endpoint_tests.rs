//! Integration tests for `GET /api/v1/cognitive/graph` (Cognitive Graph).
//!
//! Tests stand up the full Axum router with an in-memory SQLite DB
//! and exercise the handler end-to-end.
//!
//! Contract:
//! 1. 200 with valid auth — returns CognitiveGraphResponse shape
//! 2. Empty graph — empty nodes/edges/clusters arrays
//! 3. Auth required — 401 without Bearer token
//! 4. Content shape: nodes, edges, clusters, frontierNodes, gapNodes, generatedAt

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use serde_json::Value;
use std::sync::Once;
use tower::util::ServiceExt;

mod helpers;
use helpers::build_test_app_state;

const TEST_KEY: &str = "test-key-cg2-123";

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
async fn cognitive_graph_requires_auth() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cognitive/graph")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn cognitive_graph_returns_empty_on_cold_graph() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/graph")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 4096).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // All sections should be empty on a cold graph
    assert!(
        json["nodes"].as_array().unwrap().is_empty(),
        "nodes should be empty on cold graph"
    );
    assert!(
        json["edges"].as_array().unwrap().is_empty(),
        "edges should be empty on cold graph"
    );
    assert!(
        json["clusters"].as_array().unwrap().is_empty(),
        "clusters should be empty on cold graph"
    );
    assert!(
        json["frontierNodes"].as_array().unwrap().is_empty(),
        "frontierNodes should be empty on cold graph"
    );
    assert!(
        json["gapNodes"].as_array().unwrap().is_empty(),
        "gapNodes should be empty on cold graph"
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
async fn cognitive_graph_response_has_correct_top_level_fields() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/graph")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 4096).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // Verify all expected top-level fields are present
    assert!(json.get("nodes").is_some(), "should have nodes");
    assert!(json.get("edges").is_some(), "should have edges");
    assert!(json.get("clusters").is_some(), "should have clusters");
    assert!(
        json.get("frontierNodes").is_some(),
        "should have frontierNodes"
    );
    assert!(json.get("gapNodes").is_some(), "should have gapNodes");
    assert!(json.get("generatedAt").is_some(), "should have generatedAt");

    // Verify field types
    assert!(json["nodes"].is_array(), "nodes should be array");
    assert!(json["edges"].is_array(), "edges should be array");
    assert!(json["clusters"].is_array(), "clusters should be array");
    assert!(
        json["frontierNodes"].is_array(),
        "frontierNodes should be array"
    );
    assert!(json["gapNodes"].is_array(), "gapNodes should be array");
    assert!(
        json["generatedAt"].is_string(),
        "generatedAt should be string"
    );

    Ok(())
}
