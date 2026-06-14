//! Integration tests for property validation in block write paths.
//!
//! Verifies that PropertyValidator is wired into:
//! 1. POST /api/v1/blocks (create_block)
//! 2. PUT /api/v1/blocks/:id/properties (set_block_property)
//!
//! Invalid property values must return 400 Bad Request.

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use std::sync::Once;
use tower::ServiceExt;

use quilt_infrastructure::database::sqlite::connection::create_pool;

mod helpers;
use helpers::build_test_app_state;

const TEST_API_KEY: &str = "test-api-key-for-property-validation";
static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

async fn create_test_app() -> Result<Router> {
    init_auth();

    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    let app = quilt_server::routes::create_app(state);

    Ok(app)
}

async fn req(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", TEST_API_KEY));

    let request = if let Some(body_value) = body {
        builder.body(Body::from(body_value.to_string())).unwrap()
    } else {
        builder.body(Body::empty()).unwrap()
    };

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let body_bytes = axum::body::to_bytes(response.into_body(), 10_000_000)
        .await
        .unwrap();

    let json: Value = if body_bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body_bytes).unwrap_or(Value::Null)
    };

    (status, json)
}

async fn post(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::POST, uri, Some(body)).await
}

async fn put(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::PUT, uri, Some(body)).await
}

async fn create_page(app: &Router, name: &str) -> String {
    let (status, body) = post(app.clone(), "/api/v1/pages", json!({"name": name})).await;
    assert_eq!(status, StatusCode::CREATED, "create page failed: {body}");
    body["name"].as_str().unwrap().to_string()
}

async fn create_block(app: &Router, page: &str, content: &str) -> (StatusCode, Value) {
    post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page,
            "content": content
        }),
    )
    .await
}

// ═══════════════════════════════════════════════════════════
// Property validation on block creation
// ═══════════════════════════════════════════════════════════

/// Sending an integer value for a text property (status) should be rejected
/// with 400 Bad Request when PropertyValidator is wired.
#[tokio::test]
async fn create_block_invalid_property_type_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Validation Test Page").await;

    // The builtin "quilt.property/status" is a Text property.
    // Sending an integer should fail validation.
    let (status, body) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page,
            "content": "Test block",
            "properties": {
                "status": 42  // Invalid: integer for text property
            }
        }),
    )
    .await;

    // Currently the handler silently skips invalid values (tracing::warn).
    // After wiring PropertyValidator, this should return 400.
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400 for invalid property type, got {status}: {body}"
    );

    Ok(())
}

/// Sending a string to the deadline property (Date type) should be rejected.
#[tokio::test]
async fn create_block_invalid_date_string_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Date Validation Test").await;

    // The builtin "quilt.property/deadline" is a Date property.
    // Sending a plain string (not ISO format) should fail validation.
    let (status, body) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page,
            "content": "Deadline block",
            "properties": {
                "deadline": "not-a-date"  // Invalid: string for date property
            }
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400 for invalid date string, got {status}: {body}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Property validation on set_block_property
// ═══════════════════════════════════════════════════════════

/// Setting an invalid property value via PUT /api/v1/blocks/:id/properties
/// should return 400 Bad Request.
#[tokio::test]
async fn set_block_property_invalid_type_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Set Property Test").await;

    // First create a block
    let (status, block) = create_block(&app, &page, "Test block").await;
    assert_eq!(status, StatusCode::CREATED, "create block failed: {block}");

    let block_id = block["id"].as_str().unwrap();

    // Try to set "status" (text property) to an integer
    let (status, body) = put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({
            "key": "status",
            "value": 999  // Invalid: integer for text property
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400 for invalid property type on set, got {status}: {body}"
    );

    Ok(())
}

/// Setting deadline to an invalid string should return 400.
#[tokio::test]
async fn set_block_property_invalid_date_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Set Date Test").await;

    // First create a block
    let (status, block) = create_block(&app, &page, "Deadline block").await;
    assert_eq!(status, StatusCode::CREATED, "create block failed: {block}");

    let block_id = block["id"].as_str().unwrap();

    // Try to set "deadline" (date property) to a plain string
    let (status, body) = put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({
            "key": "deadline",
            "value": "tomorrow"  // Invalid: plain string for date property
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400 for invalid date string on set, got {status}: {body}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Valid property values should succeed
// ═══════════════════════════════════════════════════════════

/// Sending a valid string value for status should succeed.
#[tokio::test]
async fn create_block_valid_string_property_succeeds() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Valid Props Page").await;

    let (status, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page,
            "content": "Valid block",
            "properties": {
                "status": "todo"  // Valid: string for text property
            }
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "Valid property should succeed, got {status}: {block}"
    );

    Ok(())
}

/// Sending a valid ISO date string for deadline should succeed.
#[tokio::test]
async fn create_block_valid_date_property_succeeds() -> Result<()> {
    let app = create_test_app().await?;
    let page = create_page(&app, "Valid Date Page").await;

    let (status, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page,
            "content": "Date block",
            "properties": {
                "deadline": "2026-06-12"  // Valid: ISO date string
            }
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "Valid date property should succeed, got {status}: {block}"
    );

    Ok(())
}
