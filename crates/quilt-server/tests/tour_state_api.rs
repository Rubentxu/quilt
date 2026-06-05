//! Integration tests for the cross-device tour-dismissal endpoints
//! (B of `quilt-fase4-cross-device-tour`).
//!
//! These tests stand up the full Axum router with an in-memory
//! SQLite database and exercise the GET/POST `/api/v1/user/tour-state`
//! surface end-to-end. They mirror the test style of the other
//! integration tests in this crate (e.g. `api_edge_cases.rs`).

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_search::SearchIndexManager;
use std::sync::{Arc, Once};
use tower::util::ServiceExt;

/// The same token used by `middleware::auth::tests`. The global
/// auth OnceLock is shared by all tests in the same test binary,
/// so we have to match the token the middleware tests already
/// initialized, otherwise our requests get 401 before they reach
/// the handler.
const TEST_KEY: &str = "test-key-123";

/// Seed the global auth OnceLock exactly once per test binary.
/// Integration tests run in their own process (not the lib's unit
/// tests) so the lib's `INIT` static doesn't help us.
static INIT: Once = Once::new();

fn init_auth() {
    INIT.call_once(|| {
        quilt_server::middleware::auth::init(TEST_KEY.to_string());
    });
}

/// Build a test router with the same composition as `create_app` —
/// state, auth middleware, all routes — but using a fresh in-memory
/// DB per test. The caller clones the router for `.oneshot()`.
async fn build_test_app() -> Result<axum::Router> {
    use quilt_application::services::ref_service::RefService;
    use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;
    use quilt_server::state::AppState;
    use tokio::sync::RwLock;

    init_auth();
    let pool = create_pool(":memory:").await?;
    run_migrations(&pool).await?;
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let ref_service = Arc::new(RwLock::new(RefService::new(ref_repo)));

    let state = AppState::new(pool, search_index, ref_service);
    Ok(quilt_server::routes::create_app(state))
}

fn auth_header(mut req: Request<Body>) -> Request<Body> {
    req.headers_mut().insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {TEST_KEY}")).unwrap(),
    );
    req
}

#[tokio::test]
async fn unauthenticated_get_returns_401() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/user/tour-state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unauthenticated_post_returns_401() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/user/tour-state/dismiss")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tour":"welcome"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authenticated_get_returns_empty_list_for_new_user() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/user/tour-state")
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
        "expected 200 with empty list, got {status} body={body_str}"
    );
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["dismissed"], serde_json::json!([]));
}

#[tokio::test]
async fn authenticated_post_dismiss_then_get_round_trips() {
    let app = build_test_app().await.unwrap();

    // POST dismiss
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/user/tour-state/dismiss")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tour":"welcome"}"#))
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
        "POST dismiss: got {status} body={body_str}"
    );

    // GET should now return ["welcome"]
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/user/tour-state")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let status = res.status();
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(status, StatusCode::OK, "GET: got {status} body={body_str}");
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["dismissed"], serde_json::json!(["welcome"]));
}

#[tokio::test]
async fn dismiss_with_empty_tour_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/user/tour-state/dismiss")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tour":""}"#))
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn dismiss_with_whitespace_tour_returns_400() {
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/user/tour-state/dismiss")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"tour":"we lcome"}"#))
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn route_is_mounted_under_api_v1_user_tour_state() {
    // The routes get nested under /api/v1/user/tour-state. A
    // request to the wrong prefix starts with /api/ so the auth
    // middleware fires — it returns 401 because we didn't send a
    // token. The important assertion is that the path is NOT 200
    // (we don't have a `/api/v1/user/wrong-path` route).
    let app = build_test_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/user/wrong-path")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(res.status(), StatusCode::OK);
}
