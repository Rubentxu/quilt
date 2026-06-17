//! HTTP integration test for the Graph Space validation endpoint
//! (ADR-0030, Slice B).
//!
//! Asserts:
//! - 200 with a typed body when the layout is valid
//! - 422 with a structured `GRAPH_INVALID` body when validation fails
//! - 400 when `graphPath` is missing or relative

use std::path::PathBuf;
use std::sync::Once;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use quilt_platform::init;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::TempDir;
use tower::ServiceExt;

/// `auth::init` is a `OnceLock`, so it must only be called once across
/// the whole test binary. Use a `Once` to gate it from any number of
/// concurrent tests.
static AUTH_INIT: Once = Once::new();

/// Spin up a minimal router with the `/api/v1/graphs/validate` route.
/// Auth is set up once via `auth::init("test-api-key")`; tests that
/// exercise a non-auth path don't need to send the header.
fn test_app() -> axum::Router {
    use axum::Router;
    use quilt_server::handlers;
    use quilt_server::middleware::auth::init;

    AUTH_INIT.call_once(|| {
        init("test-api-key".to_string());
    });

    Router::new().nest("/api/v1/graphs", handlers::graphs::routes())
}

fn seed_valid_graph(path: &std::path::Path) {
    let quilt_dir = path.join(".quilt");
    std::fs::create_dir_all(&quilt_dir).unwrap();
    let db_path = quilt_dir.join("quilt.db");
    let conn = Connection::open(&db_path).unwrap();
    conn.execute_batch("CREATE TABLE user_settings (id INTEGER PRIMARY KEY);")
        .unwrap();
}

#[tokio::test]
async fn validate_returns_200_for_valid_layout() {
    let tmp = TempDir::new().unwrap();
    seed_valid_graph(tmp.path());
    let graph_path: PathBuf = tmp.path().to_path_buf();

    let app = test_app();
    let body = serde_json::json!({ "graphPath": graph_path.display().to_string() });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graphs/validate")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-api-key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], true);
    assert!(json["dbPath"].as_str().unwrap().ends_with("quilt.db"));
}

#[tokio::test]
async fn validate_returns_422_for_missing_directory() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("does-not-exist");

    let app = test_app();
    let body = serde_json::json!({ "graphPath": missing.display().to_string() });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graphs/validate")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-api-key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "GRAPH_INVALID");
    assert_eq!(json["validationError"], "DirectoryMissing");
    assert!(json["path"].as_str().unwrap().ends_with("does-not-exist"));
}

#[tokio::test]
async fn validate_returns_422_for_missing_quilt_db() {
    let tmp = TempDir::new().unwrap();
    // .quilt/ exists but no quilt.db
    std::fs::create_dir_all(tmp.path().join(".quilt")).unwrap();

    let app = test_app();
    let body = serde_json::json!({ "graphPath": tmp.path().display().to_string() });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graphs/validate")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-api-key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "GRAPH_INVALID");
    assert_eq!(json["validationError"], "DatabaseMissing");
}

#[tokio::test]
async fn validate_returns_422_for_schema_incompatible() {
    let tmp = TempDir::new().unwrap();
    let quilt_dir = tmp.path().join(".quilt");
    std::fs::create_dir_all(&quilt_dir).unwrap();
    // Empty SQLite file (no user_settings table)
    Connection::open(quilt_dir.join("quilt.db")).unwrap();

    let app = test_app();
    let body = serde_json::json!({ "graphPath": tmp.path().display().to_string() });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graphs/validate")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-api-key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "GRAPH_INVALID");
    assert_eq!(json["validationError"], "SchemaIncompatible");
}

#[tokio::test]
async fn validate_returns_400_for_relative_path() {
    let app = test_app();
    let body = serde_json::json!({ "graphPath": "relative/path" });
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/graphs/validate")
                .header("content-type", "application/json")
                .header("authorization", "Bearer test-api-key")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Smoke test: the canonical bootstrap still works on a fresh graph dir.
#[tokio::test]
async fn init_graph_creates_layout_via_http_test() {
    let tmp = TempDir::new().unwrap();
    let _ = init::init_graph(tmp.path().to_path_buf()).expect("init_graph should succeed");
    let layout_path = tmp.path().join(".quilt").join("quilt.db");
    assert!(layout_path.exists());
}
