//! Integration tests for the projection endpoint
//!
//! Tests `GET /api/v1/blocks/:id/projection`

mod helpers;

use anyhow::Result;
use axum::http::StatusCode;
use axum::Router;
use chrono::Utc;
use quilt_domain::entities::Block;
use quilt_domain::value_objects::{PropertyValue, Uuid};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use std::collections::HashMap;
use std::sync::Arc;
use tower::ServiceExt;

type TestBlock = Block;

/// Helper to create a test block
fn make_test_block(id: Uuid, properties: HashMap<String, PropertyValue>) -> TestBlock {
    TestBlock {
        id,
        page_id: Uuid::new_v4(),
        parent_id: None,
        order: 0.0,
        level: 1,
        format: quilt_domain::value_objects::BlockFormat::Markdown,
        block_type: quilt_domain::value_objects::BlockType::Paragraph,
        marker: None,
        priority: None,
        content: "Test block content".into(),
        properties,
        refs: vec![],
        tags: vec![],
        scheduled: None,
        deadline: None,
        start_time: None,
        repeated: None,
        logbook: None,
        completed_at: None,
        cancelled_at: None,
        collapsed: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Build a test app router
async fn build_test_router() -> Result<(Router, Arc<dyn quilt_domain::repositories::BlockRepository>)> {
    let pool = create_pool(":memory:").await?;
    let state = helpers::build_test_app_state(pool).await;
    let block_repo = state.repos.block.clone();
    let app = quilt_server::routes::create_app(state);
    Ok((app, block_repo))
}

// ── Test scenarios from spec.md ──────────────────────────────────────────────

/// Scenario 1: Default view on a plain block (empty decorations)
#[tokio::test]
async fn projection_endpoint_default_view_empty_decorations() -> Result<()> {
    let (app, block_repo) = build_test_router().await?;

    let block_id = Uuid::new_v4();
    let block = make_test_block(block_id, HashMap::new());

    // Insert the block into the repo
    block_repo.insert(&block).await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/blocks/{}/projection", block_id))
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    // Decorations should be empty for default view
    assert!(json.get("decorations").and_then(|d| d.as_array()).map_or(true, |a| a.is_empty()),
        "Expected empty decorations for plain block, got: {}", json);

    // Text should be present
    assert_eq!(json.get("text").and_then(|t| t.as_str()), Some("Test block content"));

    Ok(())
}

/// Scenario 2: Task view returns task-checkbox decoration
#[tokio::test]
async fn projection_endpoint_task_view_with_checkbox() -> Result<()> {
    let (app, block_repo) = build_test_router().await?;

    let block_id = Uuid::new_v4();
    let mut props = HashMap::new();
    props.insert("type".into(), PropertyValue::string("task"));
    props.insert("status".into(), PropertyValue::string("todo"));
    let block = make_test_block(block_id, props);

    block_repo.insert(&block).await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/blocks/{}/projection", block_id))
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    // Should have a task-checkbox decoration
    let decorations = json.get("decorations").and_then(|d| d.as_array());
    assert!(decorations.is_some(), "Expected decorations array");
    let decorations = decorations.unwrap();
    assert!(!decorations.is_empty(), "Expected at least one decoration for task block");
    let has_task_checkbox = decorations.iter().any(|d| {
        d.get("kind").and_then(|k| k.as_str()) == Some("task-checkbox")
    });
    assert!(has_task_checkbox, "Expected task-checkbox decoration, got: {}", json);

    Ok(())
}

/// Scenario 4: 404 on unknown UUID
#[tokio::test]
async fn projection_endpoint_404_unknown_uuid() -> Result<()> {
    let (app, _block_repo) = build_test_router().await?;

    let unknown_id = Uuid::new_v4();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/blocks/{}/projection", unknown_id))
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::NOT_FOUND, "Expected 404 for unknown UUID");

    Ok(())
}

/// Scenario 5: 400 on malformed UUID
#[tokio::test]
async fn projection_endpoint_400_malformed_uuid() -> Result<()> {
    let (app, _block_repo) = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/blocks/not-a-valid-uuid/projection")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST, "Expected 400 for malformed UUID");

    Ok(())
}

/// Scenario 6: Cache-Control header on success
#[tokio::test]
async fn projection_endpoint_cache_header_on_success() -> Result<()> {
    let (app, block_repo) = build_test_router().await?;

    let block_id = Uuid::new_v4();
    let block = make_test_block(block_id, HashMap::new());
    block_repo.insert(&block).await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/blocks/{}/projection", block_id))
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let cache_header = response
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok());
    assert_eq!(cache_header, Some("private, max-age=30"),
        "Expected cache-control header to be 'private, max-age=30'");

    Ok(())
}

/// Scenario 7: 401 without Authorization
#[tokio::test]
async fn projection_endpoint_401_without_auth() -> Result<()> {
    let (app, _block_repo) = build_test_router().await?;

    let block_id = Uuid::new_v4();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri(format!("/api/v1/blocks/{}/projection", block_id))
                .method("GET")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED,
        "Expected 401 without Authorization header");

    Ok(())
}

/// Scenario 8: Deterministic responses (concurrent requests return byte-equal)
#[tokio::test]
async fn projection_endpoint_deterministic_responses() -> Result<()> {
    let (app, block_repo) = build_test_router().await?;

    let block_id = Uuid::new_v4();
    let mut props = HashMap::new();
    props.insert("type".into(), PropertyValue::string("task"));
    props.insert("status".into(), PropertyValue::string("todo"));
    let block = make_test_block(block_id, props);
    block_repo.insert(&block).await?;

    // Make 10 sequential requests and collect bodies
    let mut bodies: Vec<Vec<u8>> = Vec::new();
    for _ in 0i32..10 {
        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri(format!("/api/v1/blocks/{}/projection", block_id))
                    .method("GET")
                    .header("Authorization", "Bearer test-token")
                    .body(axum::body::Body::empty())?,
            )
            .await?;

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?.to_vec();
        bodies.push(body);
    }

    // All responses should be byte-equal
    let first = &bodies[0];
    for (i, body) in bodies.iter().enumerate().skip(1) {
        assert_eq!(body, first, "Response {} should be byte-equal to first response", i);
    }

    Ok(())
}
