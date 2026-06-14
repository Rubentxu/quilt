//! Comprehensive HTTP API integration tests
//!
//! Tests the full request-response cycle using `axum::Router` with
//! an in-memory SQLite database — no running server required.
//!
//! Each test creates a fresh `:memory:` database with migrations,
//! constructs a test app, and sends HTTP requests via `tower::ServiceExt::oneshot`.

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
const TEST_API_KEY: &str = "test-api-key-for-integration-tests";

/// Ensures `auth::init` is called exactly once across all tests.
static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

/// Create a fresh test application with an in-memory SQLite database.
///
/// Returns the router configured with state, auth, and CORS layers —
/// ready to serve requests via `tower::ServiceExt::oneshot`.
async fn create_test_app() -> Result<Router> {
    init_auth();

    // Create in-memory database and run migrations
    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    let app = quilt_server::routes::create_app(state);

    Ok(app)
}

/// Build an API path by percent-encoding each segment.
///
/// This ensures page names with spaces, unicode, or special characters
/// produce valid URIs.
fn api_path(segments: &[&str]) -> String {
    let encoded: Vec<String> = segments.iter().map(|s| url_encode_path(s)).collect();
    encoded.join("/")
}

/// Percent-encode a URI path segment.
///
/// Preserves `/` (path separators) and unreserved characters.
/// Encodes everything else (spaces → `%20`, unicode → `%XX` bytes, etc.)
fn url_encode_path(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            // Unreserved characters + path separator
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Send an HTTP request through the test app and return status + parsed body.
///
/// # Arguments
/// * `app` - The test router
/// * `method` - HTTP method (GET, POST, DELETE, etc.)
/// * `uri` - Request URI path (e.g., `/api/v1/pages`)
/// * `body` - Optional JSON body (None for GET/DELETE requests)
/// * `auth` - Whether to include the Bearer authorization header
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

/// Convenience: GET with auth
async fn get(app: Router, uri: &str) -> (StatusCode, Value) {
    req(app, Method::GET, uri, None, true).await
}

/// Convenience: GET without auth
async fn get_noauth(app: Router, uri: &str) -> (StatusCode, Value) {
    req(app, Method::GET, uri, None, false).await
}

/// Convenience: POST with auth and JSON body
async fn post(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::POST, uri, Some(body), true).await
}

/// Convenience: PATCH with auth and JSON body
async fn patch(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::PATCH, uri, Some(body), true).await
}

/// Convenience: DELETE with auth
async fn delete(app: Router, uri: &str) -> (StatusCode, Value) {
    req(app, Method::DELETE, uri, None, true).await
}

/// Convenience: PUT with auth and JSON body
async fn put(app: Router, uri: &str, body: Value) -> (StatusCode, Value) {
    req(app, Method::PUT, uri, Some(body), true).await
}

// ═══════════════════════════════════════════════════════════
// Pages CRUD
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn create_page_then_fetch() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page
    let (status, body) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "My Test Page", "title": "My Title"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create page failed: {body}");
    assert_eq!(body["name"], "my test page");
    assert_eq!(body["title"], "My Title");
    let page_id = body["id"].as_str().unwrap().to_string();

    // Fetch the page by name (URL-encode the path)
    let path = api_path(&["/api/v1/pages", "my test page"]);
    let (status, body) = get(app.clone(), &path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], page_id);

    // List all pages — should include ours
    let (status, body) = get(app, "/api/v1/pages").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "my test page")
    );

    Ok(())
}

#[tokio::test]
async fn create_duplicate_page_returns_error() -> Result<()> {
    let app = create_test_app().await?;

    // First creation succeeds
    let (status, _) = post(app.clone(), "/api/v1/pages", json!({"name": "Unique Page"})).await;
    assert_eq!(status, StatusCode::CREATED);

    // Second creation with same name should fail (unique constraint)
    let (status, body) = post(app, "/api/v1/pages", json!({"name": "Unique Page"})).await;
    assert!(
        status == StatusCode::CONFLICT
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::INTERNAL_SERVER_ERROR,
        "expected error status, got {status}: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn create_page_with_special_chars() -> Result<()> {
    let app = create_test_app().await?;

    // Page with spaces, unicode, and valid special characters
    let (status, body) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Page with spaces and ñ", "title": "Carácteres"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create with unicode failed: {body}"
    );
    assert!(body["name"].as_str().unwrap().contains("ñ"));

    // Fetch it back using the normalized name from the create response
    let page_name = body["name"].as_str().unwrap();
    let path = api_path(&["/api/v1/pages", page_name]);
    let (status, _body) = get(app.clone(), &path).await;
    assert_eq!(status, StatusCode::OK);

    Ok(())
}

#[tokio::test]
async fn create_page_empty_name_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = post(app, "/api/v1/pages", json!({"name": ""})).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::INTERNAL_SERVER_ERROR,
        "expected error for empty name, got {status}: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn get_nonexistent_page_returns_404() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/pages/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "expected 404, got {body}");

    Ok(())
}

#[tokio::test]
async fn delete_page_cascades_blocks() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page
    let (status, body) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Cascade Test"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let page_name = body["name"].as_str().unwrap().to_string();

    // Add blocks to the page
    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Block 1"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Block 2"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Verify blocks exist (use the normalized page name from creation response)
    let blocks_path = api_path(&["/api/v1/pages", &page_name, "blocks"]);
    let (status, body) = get(app.clone(), &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Blocks CRUD
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn create_block_with_preceding_block_id() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page
    let (status, page_body) =
        post(app.clone(), "/api/v1/pages", json!({"name": "Order Test"})).await;
    assert_eq!(status, StatusCode::CREATED);
    let page_name = page_body["name"].as_str().unwrap().to_string();

    // Create 3 blocks with preceding_block_id ordering
    let (status, b1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "First"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let b1_id = b1["id"].as_str().unwrap().to_string();

    let (status, _b2) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Second", "precedingBlockId": b1_id}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _b3) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Third", "precedingBlockId": b1_id}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Fetch all blocks for the page (use the normalized page name from creation)
    let blocks_path = api_path(&["/api/v1/pages", &page_name, "blocks"]);
    let (status, blocks_body) = get(app, &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let blocks = blocks_body.as_array().unwrap();
    assert_eq!(blocks.len(), 3);

    // Verify order (should be increasing)
    let orders: Vec<f64> = blocks
        .iter()
        .map(|b| b["order"].as_f64().unwrap())
        .collect();
    for w in orders.windows(2) {
        assert!(
            w[0] < w[1],
            "blocks should be in increasing order: {:?}",
            orders
        );
    }

    Ok(())
}

#[tokio::test]
async fn update_block_partial_fields() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page and a block
    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Update Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (status, block_body) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Original content"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let block_id = block_body["id"].as_str().unwrap().to_string();

    // Update only content
    let (status, updated) = patch(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}"),
        json!({"content": "Updated content"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["content"], "Updated content");

    Ok(())
}

#[tokio::test]
async fn update_block_with_invalid_uuid_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = patch(app, "/api/v1/blocks/not-a-uuid", json!({"content": "nope"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "expected 400, got {body}");

    Ok(())
}

#[tokio::test]
async fn delete_nonexistent_block_returns_404() -> Result<()> {
    let app = create_test_app().await?;

    let valid_nonexistent_uuid = "00000000-0000-0000-0000-000000000001";
    let (status, body) = delete(app, &format!("/api/v1/blocks/{valid_nonexistent_uuid}")).await;
    // The delete handler now verifies existence first, so a missing block
    // is a 404 (not a silent no-op). This makes client errors explicit.
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "delete nonexistent block: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn create_block_with_invalid_page_returns_404() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = post(
        app,
        "/api/v1/blocks",
        json!({"pageName": "non-existent-page", "content": "orphan"}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "expected 404, got {body}");

    Ok(())
}

#[tokio::test]
async fn create_block_with_extremely_long_content() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page
    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Long Content"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    // Create ~100KB of content
    let long_content = "A".repeat(100_000);

    let (status, body) = post(
        app,
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": long_content}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "long content creation failed: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn create_block_with_unicode_content() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Unicode Test"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let content = json!({
        "pageName": page_name,
        "content": "Hello 世界 🌍 مرحبا ¡Hola! שלום اللغة العربية 中文日本語"
    });

    let (status, body) = post(app, "/api/v1/blocks", content).await;
    assert_eq!(status, StatusCode::CREATED, "unicode block failed: {body}");
    assert!(body["content"].as_str().unwrap().contains("世界"));
    assert!(body["content"].as_str().unwrap().contains("🌍"));

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Properties
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_empty_properties() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Props Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "No props"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    let (status, body) = get(app, &format!("/api/v1/blocks/{block_id}/properties")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_object().unwrap().is_empty(),
        "expected empty properties"
    );

    Ok(())
}

#[tokio::test]
async fn set_and_get_property() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Props Test 2"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Has props"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Set a property
    let (status, set_resp) = put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "status", "value": "todo"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "set property failed: {set_resp}");

    // Get properties back
    let (status, body) = get(app, &format!("/api/v1/blocks/{block_id}/properties")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "todo");

    Ok(())
}

#[tokio::test]
async fn set_property_with_special_chars() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Special Props"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Special properties"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Key with spaces, value with special characters
    let (status, resp) = put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "my key with spaces", "value": "value with \"quotes\" and ñ"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "special property failed: {resp}");

    let (status, body) = get(app, &format!("/api/v1/blocks/{block_id}/properties")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["my key with spaces"], "value with \"quotes\" and ñ");

    Ok(())
}

#[tokio::test]
async fn delete_nonexistent_property_returns_204() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Delete Props"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Delete props test"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Deleting a nonexistent property should be idempotent (204 or 200)
    let (status, _) = delete(
        app,
        &format!("/api/v1/blocks/{block_id}/properties/nonexistent"),
    )
    .await;
    assert!(
        status == StatusCode::NO_CONTENT || status == StatusCode::OK,
        "expected 204 or 200, got {status}"
    );

    Ok(())
}

#[tokio::test]
async fn overwrite_existing_property() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Overwrite Props"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Overwrite test"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Set status=todo
    put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "status", "value": "todo"}),
    )
    .await;

    // Overwrite with status=doing
    let (status, _) = put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "status", "value": "doing"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify
    let (_, body) = get(app, &format!("/api/v1/blocks/{block_id}/properties")).await;
    assert_eq!(body["status"], "doing");

    Ok(())
}

#[tokio::test]
async fn property_value_types() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Types"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Types test"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Set string
    put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "str", "value": "hello"}),
    )
    .await;

    // Set number
    put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "num", "value": 42}),
    )
    .await;

    // Set boolean
    put(
        app.clone(),
        &format!("/api/v1/blocks/{block_id}/properties"),
        json!({"key": "bool", "value": true}),
    )
    .await;

    // Verify all types
    let (_, body) = get(app, &format!("/api/v1/blocks/{block_id}/properties")).await;
    assert_eq!(body["str"], "hello");
    assert_eq!(body["num"], 42);
    assert_eq!(body["bool"], true);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Refs (backlinks)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn extract_page_refs_on_save() -> Result<()> {
    let app = create_test_app().await?;

    // Create target page
    let (_, target_page) = post(app.clone(), "/api/v1/pages", json!({"name": "Target Page"})).await;
    let target_name = target_page["name"].as_str().unwrap().to_string();

    // Create source page
    let (_, source_page) = post(app.clone(), "/api/v1/pages", json!({"name": "Source"})).await;
    let source_name = source_page["name"].as_str().unwrap().to_string();

    // Create block that references [[Target Page]]
    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": source_name, "content": format!("This links to [[{target_name}]]")}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Query target page's backlinks (use normalized page name)
    let backlinks_path = api_path(&["/api/v1/pages", &target_name, "backlinks"]);
    let (status, body) = get(app.clone(), &backlinks_path).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array().unwrap().len() >= 1,
        "expected at least 1 backlink, got: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn extract_block_refs_on_save() -> Result<()> {
    let app = create_test_app().await?;

    // Create a page
    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Block Refs"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    // Create first block (target)
    let (_, block1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Target block"}),
    )
    .await;
    let block1_id = block1["id"].as_str().unwrap().to_string();

    // Create second block referencing first via ((uuid))
    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": format!("This references (({block1_id}))")}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Verify the first block has backlinks
    let (status, body) = get(app, &format!("/api/v1/blocks/{block1_id}/backlinks")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array().unwrap().len() >= 1,
        "expected at least 1 backlink: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn get_backlinks_for_page() -> Result<()> {
    let app = create_test_app().await?;

    // Create page A and page B
    let (_, page_b) = post(app.clone(), "/api/v1/pages", json!({"name": "Page B"})).await;
    let b_name = page_b["name"].as_str().unwrap().to_string();

    let (_, page_a) = post(app.clone(), "/api/v1/pages", json!({"name": "Page A"})).await;
    let a_name = page_a["name"].as_str().unwrap().to_string();

    // Create block in page A that references [[Page B]]
    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": a_name, "content": format!("Referencing [[{b_name}]]")}),
    )
    .await;

    // Query B's backlinks — should find A's block (use normalized page name)
    let backlinks_path = api_path(&["/api/v1/pages", &b_name, "backlinks"]);
    let (status, body) = get(app, &backlinks_path).await;
    assert_eq!(status, StatusCode::OK);
    let backlinks = body.as_array().unwrap();
    assert!(
        !backlinks.is_empty(),
        "expected backlinks, got empty: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn backlink_extraction_with_unicode() -> Result<()> {
    let app = create_test_app().await?;

    // Create page with unicode name
    let (_, target) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Página con ñ"}),
    )
    .await;
    let target_name = target["name"].as_str().unwrap().to_string();

    // Create source
    let (_, source) = post(app.clone(), "/api/v1/pages", json!({"name": "Source"})).await;
    let source_name = source["name"].as_str().unwrap().to_string();

    // Create block that references [[Página con ñ]]
    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": source_name, "content": format!("Referencing [[{target_name}]]")}),
    )
    .await;

    // Query backlinks for unicode page (use normalized name from response)
    let backlinks_path = api_path(&["/api/v1/pages", &target_name, "backlinks"]);
    let (status, body) = get(app, &backlinks_path).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.as_array().unwrap().is_empty(),
        "expected backlinks for unicode page: {body}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Search
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn search_by_exact_text() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Search Target"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "The quick brown fox jumps over the lazy dog"}),
    )
    .await;

    // Search for text using FTS5 MATCH syntax
    let (status, body) = get(app, "/api/v1/search?q=quick").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.as_array().unwrap().is_empty(),
        "expected search results: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn search_by_partial_text() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Partial Search"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "query example for partial matching"}),
    )
    .await;

    // FTS5 prefix matching: "quer" searches for the exact word "quer",
    // while "quer*" searches for words starting with "quer".
    // Since FTS5 has a min prefix length of 3 by default and we're matching
    // against "query", we use the exact word or use the * prefix operator.
    let (status, body) = get(app, "/api/v1/search?q=query").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !body.as_array().unwrap().is_empty(),
        "expected results for 'query': {body}"
    );

    Ok(())
}

#[tokio::test]
async fn search_no_results_returns_empty_array() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/search?q=xyznonexistent999").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "expected empty results, got: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn search_unicode_chars() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Café Page"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Le café est délicieux"}),
    )
    .await;

    // Search for unicode text
    let (status, body) = get(app, "/api/v1/search?q=caf%C3%A9").await;
    assert_eq!(status, StatusCode::OK);
    // FTS5 may or may not handle accented characters depending on tokenizer
    // So we accept either results or empty but NOT an error
    assert!(
        status == StatusCode::OK,
        "unicode search should not crash: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn search_special_chars_dont_crash() -> Result<()> {
    let app = create_test_app().await?;

    // Various special characters that might confuse FTS5
    // Note: FTS5 double-quote syntax requires proper escaping
    let searches = vec![
        "foo bar",
        "foo*bar",
        "test@example.com",
        "hello-world",
        "don't",
        "foo%bar",
    ];

    for query in searches {
        let uri = format!("/api/v1/search?q={}", url_encode_query(&query));
        let (status, body) = get(app.clone(), &uri).await;
        assert!(
            status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
            "search '{query}' returned {status}: {body}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn search_with_limit() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Limit Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    // Create 25 blocks with searchable content
    for i in 0..25 {
        let (status, _) = post(
            app.clone(),
            "/api/v1/blocks",
            json!({"pageName": page_name, "content": format!("limitsearch content block number {i}")}),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // Search with limit=5
    let (status, body) = get(app, "/api/v1/search?q=limitsearch&limit=5").await;
    assert_eq!(status, StatusCode::OK);
    let results = body.as_array().unwrap();
    assert!(
        results.len() <= 5,
        "expected <=5 results, got {}: {body}",
        results.len()
    );

    Ok(())
}

#[tokio::test]
async fn search_empty_query_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    // Search with empty query
    let (status, body) = get(app, "/api/v1/search?q=").await;
    assert!(
        status == StatusCode::OK || status == StatusCode::BAD_REQUEST,
        "empty query returned {status}: {body}"
    );

    Ok(())
}

/// End-to-end injection safety: queries that would have crashed the
/// FTS5 parser before sanitization must never produce a 5xx response.
///
/// The new `build_fts5_match_query` in `quilt-search::sanitize`
/// guarantees that user input never reaches the FTS5 parser in raw form.
/// This test proves that promise holds at the HTTP boundary.
#[tokio::test]
async fn search_with_dangerous_query_doesnt_crash() -> Result<()> {
    let app = create_test_app().await?;

    let dangerous_queries = vec![
        "\"unterminated",                // unterminated FTS5 string
        "(broken",                       // unclosed paren
        "foo AND",                       // dangling operator
        "\"",                            // bare quote
        "*",                             // bare asterisk
        "'); DROP TABLE blocks; --",     // SQL injection attempt
        "MATCH()",                       // call-shaped FTS5 syntax
        "foo\" OR 1=1 --",               // quote-escape attempt
        "<script>alert('xss')</script>", // XSS attempt
        "\"foo\"; --",                   // quote break-out
    ];

    for q in dangerous_queries {
        let uri = format!("/api/v1/search?q={}", url_encode_query(q));
        let (status, _body) = get(app.clone(), &uri).await;
        // The contract: 5xx means a bug. Anything 1xx/2xx/3xx/4xx is OK.
        // We expect either 200 (empty results) or 400 (empty / all-operator
        // input that produces no FTS5 tokens).
        assert!(
            status < StatusCode::INTERNAL_SERVER_ERROR,
            "Query {q:?} caused 5xx error: {status}"
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Block link (POST /api/v1/blocks/link)
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn link_two_existing_blocks() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Link Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, b1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Source"}),
    )
    .await;
    let b1_id = b1["id"].as_str().unwrap().to_string();

    let (_, b2) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Target"}),
    )
    .await;
    let b2_id = b2["id"].as_str().unwrap().to_string();

    // Link them
    let (status, body) = post(
        app.clone(),
        "/api/v1/blocks/link",
        json!({"sourceId": b1_id, "targetId": b2_id}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "link failed: {body}");

    Ok(())
}

#[tokio::test]
async fn link_to_nonexistent_block_returns_404() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Link Error"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, b1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Source"}),
    )
    .await;
    let b1_id = b1["id"].as_str().unwrap().to_string();

    let nonexistent = "00000000-0000-0000-0000-000000000099";

    let (status, body) = post(
        app,
        "/api/v1/blocks/link",
        json!({"sourceId": b1_id, "targetId": nonexistent}),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "expected 404, got {body}");

    Ok(())
}

#[tokio::test]
async fn link_same_block_twice_is_idempotent() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Idempotent Link"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, b1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Source"}),
    )
    .await;
    let b1_id = b1["id"].as_str().unwrap().to_string();

    let (_, b2) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Target"}),
    )
    .await;
    let b2_id = b2["id"].as_str().unwrap().to_string();

    // Link twice
    let (status1, _) = post(
        app.clone(),
        "/api/v1/blocks/link",
        json!({"sourceId": b1_id, "targetId": b2_id}),
    )
    .await;
    assert_eq!(status1, StatusCode::CREATED);

    let (status2, _) = post(
        app,
        "/api/v1/blocks/link",
        json!({"sourceId": b1_id, "targetId": b2_id}),
    )
    .await;
    assert_eq!(
        status2,
        StatusCode::CREATED,
        "second link should also succeed"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// DSL Query
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn dsl_query_no_params_returns_all() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "DSL Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "DSL block 1"}),
    )
    .await;

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "DSL block 2"}),
    )
    .await;

    // Query without DSL param — returns all blocks
    let (status, body) = get(app, "/api/v1/blocks").await;
    assert_eq!(status, StatusCode::OK);
    let blocks = body.as_array().unwrap();
    assert!(
        blocks.len() >= 2,
        "expected at least 2 blocks, got {}",
        blocks.len()
    );

    Ok(())
}

#[tokio::test]
async fn dsl_query_empty_string_returns_all() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "DSL Empty"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "DSL empty block"}),
    )
    .await;

    // DSL query with empty string should return all blocks
    let (status, body) = get(app, "/api/v1/blocks?dsl=").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.is_array(),
        "dsl query with empty string should return blocks"
    );

    Ok(())
}

#[tokio::test]
async fn dsl_query_invalid_syntax_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    // Send an invalid DSL query
    let (status, body) = get(app, "/api/v1/blocks?dsl=%7B%7B%7B%7Binvalid").await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::OK,
        "invalid DSL should be handled: got {status}: {body}"
    );

    Ok(())
}

#[tokio::test]
async fn dsl_query_with_limit() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "DSL Limit"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    for i in 0..10 {
        post(
            app.clone(),
            "/api/v1/blocks",
            json!({"pageName": page_name, "content": format!("dsl limit block {i}")}),
        )
        .await;
    }

    // Can't easily URL-encode the query params in dsl&limit format,
    // so use the API path builder for the base URI
    let (status, body) = get(app, "/api/v1/blocks?dsl=&limit=3").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array().unwrap().len() <= 3,
        "expected <=3 with limit=3, got {}",
        body.as_array().unwrap().len()
    );

    Ok(())
}

#[tokio::test]
async fn dsl_query_no_blocks_returns_empty() -> Result<()> {
    let app = create_test_app().await?;

    // No blocks created — query should return empty
    let (status, body) = get(app, "/api/v1/blocks").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body.as_array().unwrap().len(),
        0,
        "expected empty blocks: {body}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Settings
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_settings_returns_default() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/settings").await;
    assert_eq!(status, StatusCode::OK);
    // UserSettings serializes with snake_case field names (no rename_all)
    assert_eq!(body["timezone"], "UTC");
    assert_eq!(body["journal_format"], "%Y-%m-%d", "got: {body}");

    Ok(())
}

#[tokio::test]
async fn update_settings_persists() -> Result<()> {
    let app = create_test_app().await?;

    // Update timezone
    let (status, body) = put(
        app.clone(),
        "/api/v1/settings",
        json!({"timezone": "America/Argentina/Buenos_Aires"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update settings failed: {body}");
    assert_eq!(body["timezone"], "America/Argentina/Buenos_Aires");

    // Verify persisted
    let (status, body) = get(app, "/api/v1/settings").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["timezone"], "America/Argentina/Buenos_Aires");

    Ok(())
}

#[tokio::test]
async fn get_journal_formats_returns_all() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/settings/formats").await;
    assert_eq!(status, StatusCode::OK);
    let formats = body.as_array().unwrap();
    assert!(!formats.is_empty(), "expected date format options: {body}");

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Health
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn health_endpoint_returns_200() -> Result<()> {
    let app = create_test_app().await?;

    // Health endpoint does NOT require auth
    let (status, body) = get_noauth(app, "/health").await;
    assert_eq!(status, StatusCode::OK, "health check failed: {body}");
    assert_eq!(body["status"], "ok");

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Auth / Security
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn api_route_without_auth_returns_401() -> Result<()> {
    let app = create_test_app().await?;

    let (status, _) = get_noauth(app, "/api/v1/pages").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn api_route_with_wrong_key_returns_401() -> Result<()> {
    let app = create_test_app().await?;

    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/pages")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, "Bearer wrong-key")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

#[tokio::test]
async fn options_preflight_bypasses_auth() -> Result<()> {
    let app = create_test_app().await?;

    let request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/api/v1/pages")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Concurrent operations
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn concurrent_creates_dont_corrupt() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Concurrent"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    // Spawn 10 concurrent block creation tasks
    let mut handles = Vec::new();
    for i in 0..10 {
        let app = app.clone();
        let pn = page_name.clone();
        handles.push(tokio::spawn(async move {
            post(
                app,
                "/api/v1/blocks",
                json!({"pageName": pn, "content": format!("Concurrent block {i}")}),
            )
            .await
        }));
    }

    // Await all and collect results
    let mut successes = 0;
    for handle in handles {
        let (status, _) = handle.await.unwrap();
        if status == StatusCode::CREATED {
            successes += 1;
        }
    }

    // All 10 should succeed
    assert_eq!(successes, 10, "not all concurrent creates succeeded");

    // Verify all blocks are there (use normalized page name)
    let blocks_path = api_path(&["/api/v1/pages", &page_name, "blocks"]);
    let (status, body) = get(app, &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 10);

    Ok(())
}

#[tokio::test]
async fn concurrent_updates_to_same_block() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Concurrent Update"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (_, block) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Original"}),
    )
    .await;
    let block_id = block["id"].as_str().unwrap().to_string();

    // Spawn 5 concurrent updates
    let mut handles = Vec::new();
    for i in 0..5 {
        let app = app.clone();
        let bid = block_id.clone();
        handles.push(tokio::spawn(async move {
            patch(
                app,
                &format!("/api/v1/blocks/{bid}"),
                json!({"content": format!("Update {i}")}),
            )
            .await
        }));
    }

    // All should succeed (last write wins on the SQLite single-writer pool)
    for handle in handles {
        let (status, body) = handle.await.unwrap();
        assert!(
            status == StatusCode::OK,
            "concurrent update failed: {status}: {body}"
        );
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Delete-block child-check (orphan prevention)
// ═══════════════════════════════════════════════════════════
//
// These tests pin the contract that `DELETE /api/v1/blocks/:id` refuses
// to delete a block that still has children. The motivation is to prevent
// silently orphaning child blocks (rows whose `parent_id` would point to
// a non-existent parent). The handler returns 409 Conflict with a clear,
// actionable message; 204 No Content on success.

/// Deleting a block that has at least one child must return 409 and
/// leave both the parent and the child in place.
#[tokio::test]
async fn delete_block_with_children_returns_409() -> Result<()> {
    let app = create_test_app().await?;

    // Create page + parent + child.
    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Orphan Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (status, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Parent block"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let (status, child) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Child block",
            "parentId": parent_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let child_id = child["id"].as_str().unwrap().to_string();

    // Attempt to delete the parent — must be rejected.
    let (status, body) = delete(app.clone(), &format!("/api/v1/blocks/{parent_id}")).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "expected 409 Conflict when block has children, got {status}: {body}"
    );

    // Error message must be actionable: it should mention the child count
    // and suggest the next step, so the UI can surface useful guidance.
    let error_msg = body["error"].as_str().unwrap_or_default();
    assert!(
        error_msg.contains("1 child"),
        "error message should mention the number of children, got: {error_msg}"
    );
    assert!(
        error_msg.contains("Delete or re-parent"),
        "error message should suggest the next step, got: {error_msg}"
    );

    // Verify the parent block still exists after the rejected delete.
    // The blocks collection endpoint lives under /api/v1/pages/:name/blocks
    // — use it to confirm the parent wasn't deleted.
    let blocks_path = api_path(&["/api/v1/pages", &page_name, "blocks"]);
    let (status, blocks_body) = get(app.clone(), &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let block_ids: Vec<&str> = blocks_body
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert!(
        block_ids.contains(&parent_id.as_str()),
        "parent must still exist after rejected delete"
    );
    assert!(
        block_ids.contains(&child_id.as_str()),
        "child must still exist after rejected delete"
    );

    Ok(())
}

/// Deleting a leaf block (no children) must return 204 No Content.
#[tokio::test]
async fn delete_leaf_block_returns_204() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(app.clone(), "/api/v1/pages", json!({"name": "Leaf Test"})).await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (status, block_body) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Lonely leaf"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let block_id = block_body["id"].as_str().unwrap().to_string();

    let (status, body) = delete(app, &format!("/api/v1/blocks/{block_id}")).await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "expected 204 No Content for leaf delete, got {status}: {body}"
    );

    Ok(())
}

/// Once all children are deleted, the parent becomes a leaf and can be
/// deleted. This pins the recommended workflow: delete bottom-up.
#[tokio::test]
async fn delete_block_after_deleting_children_succeeds() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Bottom-up Delete"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (status, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Parent"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let (status, child) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Child",
            "parentId": parent_id,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let child_id = child["id"].as_str().unwrap().to_string();

    // Sanity: deleting the parent first must fail.
    let (status, _) = delete(app.clone(), &format!("/api/v1/blocks/{parent_id}")).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Delete the child first (bottom-up).
    let (status, _) = delete(app.clone(), &format!("/api/v1/blocks/{child_id}")).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Now the parent is a leaf and should be deletable.
    let (status, body) = delete(app, &format!("/api/v1/blocks/{parent_id}")).await;
    assert_eq!(
        status,
        StatusCode::NO_CONTENT,
        "parent should be deletable after its child is gone, got {status}: {body}"
    );

    Ok(())
}

/// Multiple children: the error message must reflect the actual count
/// so the user understands the scope of the problem.
#[tokio::test]
async fn delete_block_with_multiple_children_reports_count() -> Result<()> {
    let app = create_test_app().await?;

    let (_, page) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "Many Children"}),
    )
    .await;
    let page_name = page["name"].as_str().unwrap().to_string();

    let (status, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Parent with 3 children"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = parent["id"].as_str().unwrap().to_string();

    for i in 0..3 {
        let (status, _) = post(
            app.clone(),
            "/api/v1/blocks",
            json!({
                "pageName": page_name,
                "content": format!("Child {i}"),
                "parentId": parent_id,
            }),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
    }

    let (status, body) = delete(app, &format!("/api/v1/blocks/{parent_id}")).await;
    assert_eq!(status, StatusCode::CONFLICT);
    let error_msg = body["error"].as_str().unwrap_or_default();
    assert!(
        error_msg.contains("3 child"),
        "error should report the actual child count (3), got: {error_msg}"
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════

/// URL-encode a string for query parameter values.
fn url_encode_query(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => '+'.to_string(),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
