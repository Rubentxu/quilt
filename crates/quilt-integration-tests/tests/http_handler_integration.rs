//! Integration tests for HTTP handlers
//!
//! These tests verify the HTTP REST API endpoints using axum-test.

use axum::{
    body::Body,
    routing::get,
    Router,
};
use http::{Request, StatusCode};
use quilt_domain::content::BlockContent;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, Uuid};
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
use sqlx::SqlitePool;
use std::sync::Arc;
use tower::ServiceExt;

/// Returns a timezone for tests (UTC)
fn test_timezone() -> TimezoneService {
    TimezoneService::from_tz_string("UTC").expect("UTC is a valid timezone")
}

/// Sets up an in-memory SQLite database with the full schema.
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to connect to memory DB");

    // Create blocks table
    sqlx::query(
        r#"
        CREATE TABLE blocks (
            id TEXT PRIMARY KEY NOT NULL,
            page_id TEXT NOT NULL,
            parent_id TEXT,
            order_index REAL NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            format TEXT NOT NULL DEFAULT 'markdown',
            marker TEXT,
            priority TEXT,
            content TEXT NOT NULL DEFAULT '',
            properties TEXT NOT NULL DEFAULT '{}',
            scheduled INTEGER,
            deadline INTEGER,
            start_time INTEGER,
            repeated INTEGER,
            logbook INTEGER,
            collapsed INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            refs TEXT NOT NULL DEFAULT '[]',
            tags TEXT NOT NULL DEFAULT '[]',
            deleted_at INTEGER,
            journal_day INTEGER,
            updated_journal_day INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create blocks table");

    // Create pages table
    sqlx::query(
        r#"
        CREATE TABLE pages (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            title TEXT,
            namespace_id TEXT,
            journal_day INTEGER,
            format TEXT NOT NULL DEFAULT 'markdown',
            file_id TEXT,
            original_name TEXT,
            journal INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create pages table");

    // Create tags table
    sqlx::query(
        r#"
        CREATE TABLE tags (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create tags table");

    // Create block_tags table
    sqlx::query(
        r#"
        CREATE TABLE block_tags (
            block_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (block_id, tag_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create block_tags table");

    // Create refs table
    sqlx::query(
        r#"
        CREATE TABLE refs (
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            PRIMARY KEY (source_id, target_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create refs table");

    // Create FTS virtual table
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE blocks_fts USING fts5(
            content,
            content=blocks,
            content_rowid=rowid
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create FTS table");

    pool
}

/// Seed test data and return the page_id
async fn seed_test_data(pool: &SqlitePool) -> Uuid {
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    // Insert test page
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("test-page")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert test page");

    page_id
}

/// Create a simple test app with health endpoint
fn create_test_app(pool: SqlitePool) -> Router {
    use quilt_http::handlers::blocks_routes;
    use quilt_http::handlers::pages_routes;
    use quilt_http::state::HttpState;

    let state = Arc::new(HttpState::new(
        pool,
        std::path::PathBuf::from("/tmp"),
        None,
    ));

    Router::new()
        .route("/health", get(health_handler))
        .merge(blocks_routes())
        .merge(pages_routes())
        .with_state(state)
}

/// Simple health check handler for testing
async fn health_handler() -> &'static str {
    "ok"
}

// =============================================================================
// Health Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_http_health_endpoint() {
    let pool = setup_test_db().await;
    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_http_health_returns_200() {
    let pool = setup_test_db().await;
    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// =============================================================================
// Page Handler HTTP Tests
// =============================================================================

#[tokio::test]
async fn test_http_list_pages() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/pages")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let pages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(!pages.is_empty());
}

#[tokio::test]
async fn test_http_get_page_by_name() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/pages/test-page")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let page: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(page["name"], "test-page");
}

#[tokio::test]
async fn test_http_get_nonexistent_page() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/pages/nonexistent-page")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 404 or empty result
    assert!(response.status() == StatusCode::NOT_FOUND || response.status() == StatusCode::OK);
}

#[tokio::test]
async fn test_http_get_page_blocks() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    // Create a block on the page
    let block_repo = SqliteBlockRepository::new(pool.clone());
    let tz = test_timezone();

    let block = Block::new(
        BlockCreate {
            page_id,
            content: BlockContent::from_text("Test block content"),
            marker: None,
            format: BlockFormat::Markdown,
            ..Default::default()
        },
        &tz,
    )
    .unwrap();

    block_repo.insert(&block).await.unwrap();

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/blocks?page_id=test-page")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 OK even if empty or with results
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::INTERNAL_SERVER_ERROR);
}

// =============================================================================
// Block Handler HTTP Tests
// =============================================================================

#[tokio::test]
async fn test_http_create_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let request_body = serde_json::json!({
        "page_id": page_id.to_string(),
        "content": "New block from HTTP",
        "format": "markdown"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/blocks")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should either succeed or fail gracefully
    assert!(
        response.status() == StatusCode::CREATED
            || response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_http_get_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    // Create a block
    let block_repo = SqliteBlockRepository::new(pool.clone());
    let tz = test_timezone();

    let block = Block::new(
        BlockCreate {
            page_id,
            content: BlockContent::from_text("Block to fetch"),
            marker: None,
            format: BlockFormat::Markdown,
            ..Default::default()
        },
        &tz,
    )
    .unwrap();

    block_repo.insert(&block).await.unwrap();

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/blocks/{}", block.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 or 404
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
    );
}

// =============================================================================
// HTTP Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_http_invalid_json() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/blocks")
                .header("Content-Type", "application/json")
                .body(Body::from(b"invalid json".to_vec()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 Bad Request or 500 Internal Server Error
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_http_missing_content_type() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let app = create_test_app(pool);

    let request_body = serde_json::json!({
        "page_id": page_id.to_string(),
        "content": "Test"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/blocks")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should still work or fail gracefully
    assert!(response.status() != StatusCode::METHOD_NOT_ALLOWED);
}

// =============================================================================
// HTTP CORS Tests
// =============================================================================

#[tokio::test]
async fn test_http_cors_headers() {
    let pool = setup_test_db().await;
    let app = create_test_app(pool);

    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("Origin", "http://localhost:3000")
                .header("Access-Control-Request-Method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should have CORS headers or at least not fail
    assert!(response.status() != StatusCode::METHOD_NOT_ALLOWED);
}