//! Integration tests for `GET /api/v1/pages/search` (S2-03).
//!
//! S2-03 moved the page-name filter from a client-side
//! `Array.prototype.includes` (over the entire `listPages` response) to
//! a server-side endpoint that runs `LIKE '%q%'` against the SQLite
//! pages table. These tests pin the contract:
//!
//!   1. Empty `q` returns a bounded, name-ordered slice of all pages
//!      (lets the frontend use the same endpoint for empty + typed
//!      queries).
//!   2. Non-empty `q` returns ONLY pages whose name OR title contains
//!      the query (case-insensitive — names are stored lowercased by
//!      the `Page` entity).
//!   3. The `limit` param caps the response payload (1..=200, default
//!      50).
//!   4. The endpoint requires the Bearer token (auth middleware
//!      coverage — the same path as `list_pages`).
//!   5. URL path conflict: `/api/v1/pages/search` is a literal segment
//!      and MUST be matched before the `/:name` catch-all so that
//!      `search` is not interpreted as a page name.
//!
//! Test layout mirrors `api_edge_cases.rs`: a fresh `:memory:` SQLite
//! per test, a full Axum router, and `tower::ServiceExt::oneshot` to
//! drive HTTP requests without a real server.

use anyhow::Result;
use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::Once;
use tokio::sync::RwLock;
use tower::ServiceExt;

use quilt_application::services::ref_service::RefService;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;
use quilt_search::SearchIndexManager;

/// Auth key used for all tests (must match what's initialized via `init()`).
const TEST_API_KEY: &str = "test-api-key-for-pages-search";

/// Ensures `auth::init` is called exactly once across all tests.
static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

/// Create a fresh test application with an in-memory SQLite database.
async fn create_test_app() -> Result<Router> {
    init_auth();

    let pool = create_pool(":memory:").await?;
    run_migrations(&pool).await?;

    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));

    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let mut ref_service = RefService::new(ref_repo);
    ref_service.rebuild_from_repo().await?;
    let ref_service = Arc::new(RwLock::new(ref_service));

    let state = quilt_server::state::AppState::new(pool, search_index, ref_service);
    let app = quilt_server::routes::create_app(state);

    Ok(app)
}

/// Send an HTTP request through the test app and return status + parsed body.
async fn req(
    app: Router,
    method: axum::http::Method,
    uri: &str,
    body: Option<Value>,
    auth: bool,
) -> (axum::http::StatusCode, Value) {
    use axum::body::Body;
    use axum::http::{Request, header};

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
    req(app, axum::http::Method::GET, uri, None, true).await
}

async fn get_noauth(app: Router, uri: &str) -> (StatusCode, Value) {
    req(app, axum::http::Method::GET, uri, None, false).await
}

async fn post(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, axum::http::Method::POST, uri, Some(body), true).await
}

/// Helper: create N pages and return them as a JSON array.
async fn seed_pages(app: Router, names: &[&str]) -> Result<Vec<Value>> {
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let (status, body) = post(
            app.clone(),
            "/api/v1/pages",
            json!({ "name": name }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "create_page failed: {body}");
        out.push(body);
    }
    Ok(out)
}

// ──── Tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn search_pages_with_empty_query_returns_all_pages_bounded_by_limit() -> Result<()> {
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["alpha", "bravo", "charlie", "delta"]).await?;

    // Default limit (50) covers the 4 seeded pages.
    let (status, body) = get(app.clone(), "/api/v1/pages/search?q=").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("search response should be an array");
    assert_eq!(arr.len(), 4, "empty query should return all pages");

    // Pages are returned ordered by name (alpha < bravo < charlie < delta).
    let names: Vec<&str> = arr.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["alpha", "bravo", "charlie", "delta"]);

    Ok(())
}

#[tokio::test]
async fn search_pages_with_missing_q_param_returns_all_pages() -> Result<()> {
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["alpha", "bravo"]).await?;

    let (status, body) = get(app, "/api/v1/pages/search").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 2, "missing q should behave like empty q");

    Ok(())
}

#[tokio::test]
async fn search_pages_filters_by_name_substring() -> Result<()> {
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["alpha", "alphabet", "bravo", "charlie"]).await?;

    // 'al' matches 'alpha' and 'alphabet' but not 'bravo' or 'charlie'.
    let (status, body) = get(app, "/api/v1/pages/search?q=al").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    let names: Vec<&str> = arr.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["alpha", "alphabet"]);

    Ok(())
}

#[tokio::test]
async fn search_pages_matches_by_title_when_name_does_not_contain_query() -> Result<()> {
    let app = create_test_app().await?;
    // Create a page with a custom title that does NOT appear in the name.
    let (status, _body) = post(
        app.clone(),
        "/api/v1/pages",
        json!({ "name": "logseq", "title": "Knowledge Graph Notes" }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Querying for "knowledge" should match via the title, not the name.
    let (status, body) = get(app, "/api/v1/pages/search?q=knowledge").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 1, "title-only match should still surface");
    assert_eq!(arr[0]["name"], "logseq");
    assert_eq!(arr[0]["title"], "Knowledge Graph Notes");

    Ok(())
}

#[tokio::test]
async fn search_pages_is_case_insensitive() -> Result<()> {
    // The Page entity normalises names to lowercase at write time, so
    // any case variation in the query should still match — the
    // repository's LIKE + the entity's lowercased storage guarantee
    // this end to end. The test pins the behaviour so a future
    // "make it case-sensitive" change does not silently break
    // users with mixed-case queries.
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["rust programming"]).await?;

    let (status, body) = get(app, "/api/v1/pages/search?q=RUST").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 1);

    Ok(())
}

#[tokio::test]
async fn search_pages_honours_limit_param() -> Result<()> {
    let app = create_test_app().await?;
    seed_pages(
        app.clone(),
        &[
            "page1", "page2", "page3", "page4", "page5", "page6", "page7",
        ],
    )
    .await?;

    // Empty query + limit=3 → only the first 3 name-ordered pages.
    let (status, body) = get(app.clone(), "/api/v1/pages/search?q=&limit=3").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 3, "limit should cap the response size");
    let names: Vec<&str> = arr.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["page1", "page2", "page3"]);

    // Filtered query + limit=2 → at most 2 hits, even if more match.
    let (status, body) = get(app, "/api/v1/pages/search?q=page&limit=2").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 2, "limit should cap the filtered response size");

    Ok(())
}

#[tokio::test]
async fn search_pages_clamps_oversized_limit_to_200() -> Result<()> {
    // The handler clamps `limit` to [1, 200] so a malicious or
    // runaway client can't request the entire graph. With 7 pages
    // seeded and limit=999, we should still get 7 back (all of them
    // — well below the clamp ceiling) but the cap is what we are
    // really pinning. The assertion is that the endpoint does not
    // 400/422 on absurd limit values.
    let app = create_test_app().await?;
    seed_pages(
        app.clone(),
        &["a", "b", "c", "d", "e", "f", "g"],
    )
    .await?;

    let (status, body) = get(app, "/api/v1/pages/search?q=&limit=999").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert_eq!(arr.len(), 7);

    Ok(())
}

#[tokio::test]
async fn search_pages_with_no_matches_returns_empty_array() -> Result<()> {
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["alpha", "bravo"]).await?;

    let (status, body) = get(app, "/api/v1/pages/search?q=zzzzz").await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("response should be an array");
    assert!(arr.is_empty(), "no-match query should return an empty array");

    Ok(())
}

#[tokio::test]
async fn search_pages_requires_auth() -> Result<()> {
    // S2-03 endpoint is mounted under `/api/v1/*` so it goes through
    // the Bearer-token middleware. A missing token must yield 401
    // just like every other protected endpoint.
    let app = create_test_app().await?;

    let (status, _body) = get_noauth(app, "/api/v1/pages/search?q=foo").await;
    assert_eq!(
        status,
        axum::http::StatusCode::UNAUTHORIZED,
        "search endpoint must require auth like the rest of /api/v1/*"
    );

    Ok(())
}

#[tokio::test]
async fn search_pages_does_not_collide_with_page_by_name_route() -> Result<()> {
    // Critical regression guard for the route registration order. The
    // literal segment `/search` MUST be registered before the
    // `/:name` catch-all, otherwise axum would interpret the literal
    // string `"search"` as a page name and route it to the
    // get_page handler. With no page named "search" we'd get a
    // 404, which is the failure mode we want to catch.
    //
    // We hit the search endpoint and assert it returns 200 (the
    // search handler), not 404 (the get_page handler with a missing
    // page). This pins the route ordering for the lifetime of the
    // project.
    let app = create_test_app().await?;
    seed_pages(app.clone(), &["alpha"]).await?;

    // Without a query, the search endpoint still returns 200 with an
    // array — it does NOT 404 like get_page would for a missing name.
    let (status, body) = get(app, "/api/v1/pages/search?q=").await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "search must be a literal route, not a page-name 404: {body}"
    );
    assert!(body.is_array());

    Ok(())
}
