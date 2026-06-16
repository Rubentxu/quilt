//! Integration tests for `GET /api/v1/properties/keys` (T7 of
//! property-keys-endpoint).
//!
//! These tests stand up the full Axum router with an in-memory
//! SQLite DB and exercise the handler end-to-end — auth, validation,
//! pagination, JSON shape, and the empty-DB contract. They follow
//! the same pattern as `tour_state_api.rs` and `api_edge_cases.rs`.
//!
//! Auth: every request carries `Authorization: Bearer <TEST_KEY>`.
//! The token must match what the global auth `OnceLock` was seeded
//! with. We match `middleware::auth::tests`'s key.

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_domain::entities::{Block, BlockCreate, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use std::collections::HashMap;
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

async fn build_test_app() -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    Ok(quilt_server::routes::create_app(state))
}

fn auth_header(mut req: Request<Body>) -> Request<Body> {
    req.headers_mut().insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {TEST_KEY}")).unwrap(),
    );
    req
}

// Helper: build the app, seed `n` blocks on a single page with one
// distinct property key per block, return the router.
async fn app_with_n_keys(n: u32) -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;

    let (state, block_repo, page_repo, _, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;

    // Create a page to host the blocks.
    let page = quilt_domain::entities::Page::new(PageCreate {
        name: "p".to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: HashMap::new(),
    })
    .unwrap();
    page_repo.insert(&page).await.unwrap();

    // Insert N blocks, each with a single distinct property key.
    for i in 0..n {
        let key = format!("key_{i:03}");
        let mut props = HashMap::new();
        props.insert(key.clone(), PropertyValue::string("v"));
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: format!("b{i}"),
            parent_id: None,
            order: i as f64,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: props,
        })
        .unwrap();
        block_repo.insert(&block).await.unwrap();
    }

    Ok(quilt_server::routes::create_app(state))
}

// ── Auth ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_keys_without_authorization_returns_401() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_keys_with_wrong_bearer_returns_401() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys")
                .header("authorization", "Bearer wrong-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ── Empty / shape ───────────────────────────────────────────────────

#[tokio::test]
async fn get_keys_empty_db_returns_empty_array_null_cursor() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(
        status,
        StatusCode::OK,
        "expected 200, got {status} body={body_str}"
    );
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["keys"], serde_json::json!([]));
    assert_eq!(json["nextCursor"], serde_json::Value::Null);
}

#[tokio::test]
async fn get_keys_blocks_with_no_properties_returns_empty_array() {
    // App with one block whose properties map is empty.
    init_auth();
    let pool = create_pool(":memory:").await.unwrap();

    let (state, block_repo, page_repo, _, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;

    let page = quilt_domain::entities::Page::new(PageCreate {
        name: "p".to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: HashMap::new(),
    })
    .unwrap();
    page_repo.insert(&page).await.unwrap();

    let block = Block::new(BlockCreate {
        page_id: page.id,
        content: "x".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: HashMap::new(),
    })
    .unwrap();
    block_repo.insert(&block).await.unwrap();

    let app = quilt_server::routes::create_app(state);

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["keys"], serde_json::json!([]));
    assert_eq!(json["nextCursor"], serde_json::Value::Null);
}

// ── Happy path / pagination ─────────────────────────────────────────

#[tokio::test]
async fn get_keys_default_limit_returns_50_and_sets_next_cursor() {
    let app = app_with_n_keys(75).await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let keys = json["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 50, "default limit should be 50");
    assert_eq!(keys[0], "key_000");
    assert_eq!(keys[49], "key_049");

    // nextCursor is the 50th key (because more pages exist).
    let cursor = json["nextCursor"].as_str().unwrap();
    assert_eq!(cursor, "key_049");
}

#[tokio::test]
async fn get_keys_with_limit_10_slices_to_10() {
    let app = app_with_n_keys(30).await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?limit=10")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let keys = json["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 10);
    assert_eq!(keys[0], "key_000");
    assert_eq!(keys[9], "key_009");
    assert_eq!(json["nextCursor"].as_str().unwrap(), "key_009");
}

#[tokio::test]
async fn get_keys_cursor_paginates_forward() {
    let app = app_with_n_keys(30).await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?cursor=key_009&limit=10")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let keys = json["keys"].as_array().unwrap();
    // Strictly > key_009 → starts at key_010
    assert_eq!(keys[0], "key_010");
    assert_eq!(keys[9], "key_019");
    assert_eq!(json["nextCursor"].as_str().unwrap(), "key_019");
}

#[tokio::test]
async fn get_keys_last_page_returns_null_cursor() {
    // 25 keys, limit=20 → page 1 returns 20 keys + cursor;
    // page 2 returns the last 5 keys + null cursor.
    let app = app_with_n_keys(25).await.unwrap();

    // Page 1
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?limit=20")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let cursor = json["nextCursor"].as_str().unwrap().to_string();
    assert_eq!(json["keys"].as_array().unwrap().len(), 20);

    // Page 2
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/properties/keys?cursor={cursor}&limit=20"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status2 = res.status();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status2, StatusCode::OK, "body={body_str}");
    let keys = json["keys"].as_array().unwrap();
    assert_eq!(keys.len(), 5, "5 keys remain after first page");
    assert_eq!(json["nextCursor"], serde_json::Value::Null);
}

#[tokio::test]
async fn get_keys_cursor_past_end_returns_empty_null_cursor() {
    let app = app_with_n_keys(3).await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?cursor=zzz&limit=10")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 65536).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["keys"], serde_json::json!([]));
    assert_eq!(json["nextCursor"], serde_json::Value::Null);
}

// ── Validation errors ───────────────────────────────────────────────

#[tokio::test]
async fn get_keys_limit_zero_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?limit=0")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_keys_limit_above_100_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?limit=101")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_keys_limit_non_numeric_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?limit=abc")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    // Axum deserialization rejects automatically with 400 (or 422 in
    // some versions). Both are acceptable "client error" outcomes;
    // we accept anything that's a 4xx.
    let s = res.status();
    assert!(
        s == StatusCode::BAD_REQUEST || s == StatusCode::UNPROCESSABLE_ENTITY,
        "expected 4xx for non-numeric limit, got {s}"
    );
}

#[tokio::test]
async fn get_keys_empty_cursor_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/properties/keys?cursor=")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
