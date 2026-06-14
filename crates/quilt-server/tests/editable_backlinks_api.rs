//! Integration tests for the Editable Backlinks feature (ROADMAP Q028).
//!
//! Per Q028, backlinks should be editable: the user can override the
//! context snippet shown for a backlink with a custom value that is
//! stored per-reference (not per-block) and surfaces in the Backlinks
//! panel and the `/api/v1/pages/:name/backlinks` JSON shape.
//!
//! Endpoint under test:
//!   PUT /api/v1/references/:sourceBlockId?targetPage=<name>
//!
//! The PUT body is `{ "context": "<text>" }` and the response is the
//! updated `BacklinkDto`. To clear the override, send `context: null`
//! or `context: ""`.
//!
//! Tests in this file follow the same pattern as
//! `tests/api_edge_cases.rs` and `tests/property_keys_handler.rs`:
//! in-memory SQLite + full Axum router + Bearer auth.

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

/// Auth key used for all tests (must match what's initialized via `init()`).
const TEST_API_KEY: &str = "test-api-key-for-editable-backlinks";

/// Ensures `auth::init` is called exactly once across all tests.
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
    Ok(quilt_server::routes::create_app(state))
}

async fn req(
    app: Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    auth: bool,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if auth {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {}", TEST_API_KEY));
    }

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

async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
    req(app, Method::GET, uri, None, true).await
}

async fn post(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::POST, uri, Some(body), true).await
}

async fn put(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::PUT, uri, Some(body), true).await
}

async fn put_noauth(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::PUT, uri, Some(body), false).await
}

/// Test fixture: create two pages (`Source` and `Target`) and a block
/// in `Source` whose content contains `[[Target]]`. Returns the
/// `(source_page_name, target_page_name, source_block_id)` tuple.
async fn seed_backlink_fixture(
    app: Router,
    source: &str,
    target: &str,
) -> Result<(String, String, String)> {
    // Create target page
    let (_, target_page) = post(app.clone(), "/api/v1/pages", json!({ "name": target })).await;
    let target_name = target_page["name"].as_str().unwrap().to_string();

    // Create source page
    let (_, _) = post(app.clone(), "/api/v1/pages", json!({ "name": source })).await;

    // Create a block on Source that references [[Target]]
    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": source,
            "content": format!("Linking to [[{target_name}]]")
        }),
    )
    .await;
    let source_block_id = block["id"].as_str().unwrap().to_string();

    Ok((source.to_string(), target_name, source_block_id))
}

// ═══════════════════════════════════════════════════════════
// Auth gate
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn put_reference_context_without_authorization_returns_401() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, _tgt, block_id) =
        seed_backlink_fixture(app.clone(), "auth-src-1", "auth-tgt-1").await?;

    let (status, _) = put_noauth(
        app,
        &format!("/api/v1/references/{block_id}?targetPage=auth-tgt-1"),
        json!({ "context": "anything" }),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Happy path — set, read, clear
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn put_reference_context_sets_custom_context() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, target_name, block_id) =
        seed_backlink_fixture(app.clone(), "set-src", "set-tgt").await?;

    // PUT a custom context
    let (status, body) = put(
        app.clone(),
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": "My custom snippet about the target" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    assert_eq!(
        body["context"].as_str().unwrap(),
        "My custom snippet about the target"
    );

    // GET backlinks for the target page and confirm the override is
    // surfaced in the DTO and the panel will pick it up.
    let (status, body) = get(app, &format!("/api/v1/pages/{target_name}/backlinks")).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["context"].as_str().unwrap(),
        "My custom snippet about the target"
    );
    Ok(())
}

#[tokio::test]
async fn put_reference_context_clears_with_null() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, target_name, block_id) =
        seed_backlink_fixture(app.clone(), "clear-src", "clear-tgt").await?;

    // Set then clear
    let _ = put(
        app.clone(),
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": "Override text" }),
    )
    .await;

    let (status, body) = put(
        app.clone(),
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": null }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    // After clearing, the response DTO's `context` field falls back
    // to the source block's content snippet (the default), NOT the
    // override we previously set.
    let ctx = body["context"]
        .as_str()
        .expect("context should be a string");
    assert!(
        ctx.contains("Linking to"),
        "after clear, context should fall back to the source snippet, got: {ctx}"
    );
    assert!(
        !ctx.contains("Override text"),
        "after clear, the override must NOT appear in the response: {ctx}"
    );
    Ok(())
}

#[tokio::test]
async fn put_reference_context_clears_with_empty_string() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, target_name, block_id) =
        seed_backlink_fixture(app.clone(), "empty-src", "empty-tgt").await?;

    let _ = put(
        app.clone(),
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": "Override" }),
    )
    .await;

    let (status, body) = put(
        app,
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": "" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body={body}");
    // Empty string is treated as "clear" — the panel shows the default.
    let ctx = body["context"]
        .as_str()
        .expect("context should be a string");
    assert!(
        ctx.contains("Linking to"),
        "empty-string should also clear, falling back to default snippet, got: {ctx}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// 404 — unknown reference
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn put_reference_context_unknown_source_block_returns_404() -> Result<()> {
    let app = create_test_app().await?;
    // Source block ID is a valid UUID but no such block exists.
    let fake_block = "00000000-0000-0000-0000-000000000000";

    let (status, _) = put(
        app,
        &format!("/api/v1/references/{fake_block}?targetPage=missing-tgt"),
        json!({ "context": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn put_reference_context_unknown_target_page_returns_404() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, _tgt, block_id) =
        seed_backlink_fixture(app.clone(), "no-tgt-src", "no-tgt-tgt").await?;

    // The source block exists, but there is no ref from it to "missing-tgt"
    let (status, _) = put(
        app,
        &format!("/api/v1/references/{block_id}?targetPage=missing-tgt"),
        json!({ "context": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn put_reference_context_invalid_uuid_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let (status, _) = put(
        app,
        "/api/v1/references/not-a-uuid?targetPage=anything",
        json!({ "context": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn put_reference_context_missing_target_page_param_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, _tgt, block_id) =
        seed_backlink_fixture(app.clone(), "no-param-src", "no-param-tgt").await?;

    // No `?targetPage=` query string
    let (status, _) = put(
        app,
        &format!("/api/v1/references/{block_id}"),
        json!({ "context": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn put_reference_context_empty_target_page_param_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, _tgt, block_id) =
        seed_backlink_fixture(app.clone(), "empty-param-src", "empty-param-tgt").await?;

    // Empty query string value
    let (status, _) = put(
        app,
        &format!("/api/v1/references/{block_id}?targetPage="),
        json!({ "context": "x" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}

// ═══════════════════════════════════════════════════════════
// GET backlinks — `context` field shape
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_backlinks_returns_default_context_when_no_override() -> Result<()> {
    // When no custom context is set, the GET endpoint returns the
    // automatic snippet derived from the source block content.
    let app = create_test_app().await?;
    let (_src, target_name, _block_id) =
        seed_backlink_fixture(app.clone(), "default-src", "default-tgt").await?;

    let (status, body) = get(app, &format!("/api/v1/pages/{target_name}/backlinks")).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    // `context` field is present and equals the default snippet
    assert!(arr[0]["context"].is_string());
    assert!(
        arr[0]["context"].as_str().unwrap().contains("Linking to"),
        "expected default context to contain the source content, got: {}",
        arr[0]["context"]
    );
    Ok(())
}

#[tokio::test]
async fn get_backlinks_returns_custom_context_when_set() -> Result<()> {
    let app = create_test_app().await?;
    let (_src, target_name, block_id) =
        seed_backlink_fixture(app.clone(), "ctx-src", "ctx-tgt").await?;

    let _ = put(
        app.clone(),
        &format!("/api/v1/references/{block_id}?targetPage={target_name}"),
        json!({ "context": "A meaningful custom snippet" }),
    )
    .await;

    let (status, body) = get(app, &format!("/api/v1/pages/{target_name}/backlinks")).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["context"].as_str().unwrap(),
        "A meaningful custom snippet"
    );
    Ok(())
}
