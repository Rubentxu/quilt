//! Integration tests for the annotation REST API (annotations-comments-unification).
//!
//! Full-stack HTTP tests against the live Axum router. The test
//! setup mirrors `comments_api.rs` — an in-memory SQLite, the
//! `RefService` initialized from a fresh repo, and a Bearer-token
//! initialized in the static `OnceLock`.
//!
//! # Coverage
//!
//! - CRUD round-trip: create → read → list → update status → delete
//! - Filter combinations: `block_id`, `status`, `scope`
//! - Auth: 401 without Bearer, 200 with Bearer
//! - 404 for unknown ids
//! - 400 for invalid UUIDs, unknown enum values, empty content
//! - Convenience: `GET /api/v1/blocks/:block_id/annotations`

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use std::sync::Once;
use tower::ServiceExt;

use quilt_domain::value_objects::Uuid;
use quilt_infrastructure::database::sqlite::connection::create_pool;

mod helpers;
use helpers::build_test_app_state;

/// Auth key used for all tests (must match what's initialized via `init()`).
const TEST_API_KEY: &str = "test-api-key-annotation-integration";

/// Ensures `auth::init` is called exactly once across all tests.
static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

/// Create a fresh test app with an in-memory SQLite + the annotation
/// routes mounted under `/api/v1/annotations` and the convenience
/// route under `/api/v1/blocks/:block_id/annotations`.
async fn create_test_app() -> Result<Router> {
    init_auth();

    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    let app = quilt_server::routes::create_app(state);

    Ok(app)
}

/// Send an HTTP request through the test app. Returns `(status, body)`.
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

async fn get(app: Router, uri: &str, auth: bool) -> (StatusCode, Value) {
    req(app, Method::GET, uri, None, auth).await
}

async fn post(app: Router, uri: &str, body: Value, auth: bool) -> (StatusCode, Value) {
    req(app, Method::POST, uri, Some(body), auth).await
}

async fn patch(app: Router, uri: &str, body: Value, auth: bool) -> (StatusCode, Value) {
    req(app, Method::PATCH, uri, Some(body), auth).await
}

async fn delete(app: Router, uri: &str, auth: bool) -> (StatusCode, Value) {
    req(app, Method::DELETE, uri, None, auth).await
}

/// Build a fresh page with one block, via the REST API so the
/// fixtures live in the SAME database the test app uses. Returns
/// `(page_id_str, block_id_str)`. The page name is normalized to
/// lowercase by the API, so the helper returns the normalized name
/// to avoid a `404 Page not found` when creating the block.
async fn create_page_and_block(app: &Router, page_name: &str) -> (String, String) {
    // Create the page via /api/v1/pages
    let (status, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": page_name}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create page failed: {page}");

    // Page names are normalized to lowercase — read the canonical
    // form out of the response body.
    let normalized_name = page["name"].as_str().unwrap().to_string();
    let page_id = page["id"].as_str().unwrap().to_string();

    // Create a block on that page via /api/v1/blocks
    let (status, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": normalized_name,
            "content": "annotation target"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create block failed: {block}");

    (page_id, block["id"].as_str().unwrap().to_string())
}

// ═══════════════════════════════════════════════════════════
// Auth
// ═══════════════════════════════════════════════════════════

/// Every `/api/v1/*` request requires Bearer auth. Without it the
/// auth middleware short-circuits with 401.
#[tokio::test]
async fn auth_required_for_annotation_endpoints() -> Result<()> {
    let app = create_test_app().await?;

    // POST without auth → 401
    let (status, _) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({"blockId": "00000000-0000-0000-0000-000000000000", "scope": "block", "authorType": "human", "authorName": "u", "content": "x"}),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // GET without auth → 401
    let (status, _) = get(app.clone(), "/api/v1/annotations?status=pending", false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // PATCH without auth → 401
    let (status, _) = patch(
        app.clone(),
        "/api/v1/annotations/00000000-0000-0000-0000-000000000000/status",
        json!({"status": "resolved", "resolvedBy": "x"}),
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // DELETE without auth → 401
    let (status, _) = delete(
        app,
        "/api/v1/annotations/00000000-0000-0000-0000-000000000000",
        false,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// CRUD round-trip
// ═══════════════════════════════════════════════════════════

/// Full lifecycle: create → read → update → delete. The annotation
/// must be visible at every stage with the correct fields.
#[tokio::test]
async fn annotation_crud_round_trip() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "CRUD Page").await;
    let url = "/api/v1/annotations".to_string();

    // 1. Create
    let (status, created) = post(
        app.clone(),
        &url,
        json!({
            "blockId": block_id,
            "scope": "block",
            "authorType": "human",
            "authorName": "alice",
            "content": "Please review this paragraph"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {created}");
    let id = created["id"].as_str().unwrap().to_string();
    assert_eq!(created["status"], "pending");
    assert_eq!(created["scope"], "block");
    assert_eq!(created["authorType"], "human");
    assert_eq!(created["authorName"], "alice");
    assert_eq!(created["content"], "Please review this paragraph");
    assert_eq!(created["blockId"], block_id);
    assert!(created["createdAt"].is_string());
    assert!(
        created.get("highlightStart").is_none(),
        "block scope must not serialize offsets"
    );

    // 2. Read one
    let (status, fetched) = get(app.clone(), &format!("/api/v1/annotations/{id}"), true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fetched["id"], id);
    assert_eq!(fetched["content"], "Please review this paragraph");

    // 3. Update status (resolve)
    let (status, updated) = patch(
        app.clone(),
        &format!("/api/v1/annotations/{id}/status"),
        json!({"status": "resolved", "resolvedBy": "claude"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "patch failed: {updated}");
    assert_eq!(updated["status"], "resolved");
    assert_eq!(updated["resolvedBy"], "claude");
    assert!(updated["resolvedAt"].is_string());

    // 4. Delete
    let (status, _) = delete(app.clone(), &format!("/api/v1/annotations/{id}"), true).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 5. Read after delete → 404
    let (status, _) = get(app, &format!("/api/v1/annotations/{id}"), true).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

/// Inline annotations require both offsets and the entity validates
/// `start < end`. Sending a bad pair must surface as 400.
#[tokio::test]
async fn inline_annotation_offsets_are_validated() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Inline Page").await;

    // Missing offsets → 400
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "inline",
            "authorType": "agent",
            "authorName": "claude",
            "content": "Typo here"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "missing offsets: {body}");

    // start >= end → 400
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "inline",
            "authorType": "agent",
            "authorName": "claude",
            "content": "Typo here",
            "highlightStart": 10,
            "highlightEnd": 5
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "start>end: {body}");

    // Valid offsets → 201
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "inline",
            "authorType": "agent",
            "authorName": "claude",
            "content": "Typo here",
            "highlightStart": 0,
            "highlightEnd": 4
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "valid inline: {body}");
    assert_eq!(body["scope"], "inline");
    assert_eq!(body["highlightStart"], 0);
    assert_eq!(body["highlightEnd"], 4);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Listing + filters
// ═══════════════════════════════════════════════════════════

/// Listing with a `block_id` filter must return only annotations
/// targeting that block.
#[tokio::test]
async fn list_filters_by_block_id() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_a, block_a) = create_page_and_block(&app, "A Page").await;
    let (_page_b, block_b) = create_page_and_block(&app, "B Page").await;

    // Create 2 on block A, 1 on block B
    for content in ["x", "y"] {
        let (status, _) = post(
            app.clone(),
            "/api/v1/annotations",
            json!({
                "blockId": block_a,
                "scope": "block",
                "authorType": "human",
                "authorName": "alice",
                "content": content
            }),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }
    let (status, _) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_b,
            "scope": "block",
            "authorType": "human",
            "authorName": "bob",
            "content": "z"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // List block A → 2
    let (status, list) = get(
        app.clone(),
        &format!("/api/v1/annotations?block_id={block_a}"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 2);

    // List block B → 1
    let (status, list) = get(
        app.clone(),
        &format!("/api/v1/annotations?block_id={block_b}"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // List with no filter → 3
    let (status, list) = get(app, "/api/v1/annotations", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 3);

    Ok(())
}

/// `?status=...` filter must restrict the result set to annotations
/// in that lifecycle state.
#[tokio::test]
async fn list_filters_by_status() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Status Page").await;

    let mut ids = Vec::new();
    for content in ["p1", "p2", "p3"] {
        let (_, body) = post(
            app.clone(),
            "/api/v1/annotations",
            json!({
                "blockId": block_id,
                "scope": "block",
                "authorType": "human",
                "authorName": "alice",
                "content": content
            }),
            true,
        )
        .await;
        ids.push(body["id"].as_str().unwrap().to_string());
    }

    // Resolve the first one
    let (status, _) = patch(
        app.clone(),
        &format!("/api/v1/annotations/{}/status", &ids[0]),
        json!({"status": "resolved", "resolvedBy": "x"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // status=pending → 2
    let (status, list) = get(app.clone(), "/api/v1/annotations?status=pending", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 2);

    // status=resolved → 1
    let (status, list) = get(app, "/api/v1/annotations?status=resolved", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    Ok(())
}

/// Combining `block_id` + `status` filters must AND them.
#[tokio::test]
async fn list_combines_block_id_and_status_filters() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Combined Page").await;

    // 2 pending
    for c in ["a", "b"] {
        let (status, _) = post(
            app.clone(),
            "/api/v1/annotations",
            json!({
                "blockId": block_id,
                "scope": "block",
                "authorType": "human",
                "authorName": "alice",
                "content": c
            }),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // Combined filter: block_id + pending → 2
    let (status, list) = get(
        app.clone(),
        &format!("/api/v1/annotations?block_id={block_id}&status=pending"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 2);

    // Combined filter: block_id + resolved → 0
    let (status, list) = get(
        app,
        &format!("/api/v1/annotations?block_id={block_id}&status=resolved"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 0);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Convenience: /api/v1/blocks/:block_id/annotations
// ═══════════════════════════════════════════════════════════

/// The block-scoped convenience route must return the same data as
/// the canonical `GET /api/v1/annotations?block_id=...`.
#[tokio::test]
async fn block_scoped_convenience_route_works() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Block Conv Page").await;

    for c in ["x", "y"] {
        let (status, _) = post(
            app.clone(),
            "/api/v1/annotations",
            json!({
                "blockId": block_id,
                "scope": "block",
                "authorType": "human",
                "authorName": "alice",
                "content": c
            }),
            true,
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // Convenience route
    let (status, list) = get(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/annotations"),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 2);

    // Invalid block UUID in path → 400
    let (status, _) = get(app, "/api/v1/blocks/not-a-uuid/annotations", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// 400 / 404 paths
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn invalid_uuid_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    // GET one with bad UUID
    let (status, _) = get(app.clone(), "/api/v1/annotations/not-a-uuid", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // PATCH with bad UUID
    let (status, _) = patch(
        app.clone(),
        "/api/v1/annotations/not-a-uuid/status",
        json!({"status": "resolved", "resolvedBy": "x"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // DELETE with bad UUID
    let (status, _) = delete(app, "/api/v1/annotations/not-a-uuid", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    Ok(())
}

#[tokio::test]
async fn unknown_id_returns_404() -> Result<()> {
    let app = create_test_app().await?;
    let unknown = Uuid::new_v4().to_string();

    // GET one
    let (status, _) = get(app.clone(), &format!("/api/v1/annotations/{unknown}"), true).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // PATCH on unknown id → 404 (NotFound variant)
    let (status, _) = patch(
        app.clone(),
        &format!("/api/v1/annotations/{unknown}/status"),
        json!({"status": "resolved", "resolvedBy": "x"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    Ok(())
}

#[tokio::test]
async fn invalid_enum_values_return_400() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Enum Page").await;

    // Bad scope
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "sideways",
            "authorType": "human",
            "authorName": "alice",
            "content": "x"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "bad scope: {body}");

    // Bad authorType
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "block",
            "authorType": "robot",
            "authorName": "alice",
            "content": "x"
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "bad authorType: {body}");

    // Empty content
    let (status, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "block",
            "authorType": "human",
            "authorName": "alice",
            "content": "   "
        }),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "empty content: {body}");

    // PATCH bad status
    let (_, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "block",
            "authorType": "human",
            "authorName": "alice",
            "content": "ok"
        }),
        true,
    )
    .await;
    let id = body["id"].as_str().unwrap();
    let (status, body) = patch(
        app,
        &format!("/api/v1/annotations/{id}/status"),
        json!({"status": "frobbed"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "bad status: {body}");

    Ok(())
}

#[tokio::test]
async fn resolve_without_resolved_by_returns_400() -> Result<()> {
    let app = create_test_app().await?;
    let (_page_id, block_id) = create_page_and_block(&app, "Resolve Page").await;

    // Create
    let (_, body) = post(
        app.clone(),
        "/api/v1/annotations",
        json!({
            "blockId": block_id,
            "scope": "block",
            "authorType": "human",
            "authorName": "alice",
            "content": "x"
        }),
        true,
    )
    .await;
    let id = body["id"].as_str().unwrap();

    // PATCH status=resolved without resolvedBy → 400
    let (status, body) = patch(
        app,
        &format!("/api/v1/annotations/{id}/status"),
        json!({"status": "resolved"}),
        true,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "no resolvedBy: {body}");

    Ok(())
}
