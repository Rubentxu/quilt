//! Integration tests for the comments feature (ADR-0003).
//!
//! Comments are regular child blocks with a `type: "comment"` property.
//! They may carry `resolved`, `created_by`, and `created_at` metadata.
//! Tests here exercise the full HTTP request/response cycle against
//! the `axum::Router` with an in-memory SQLite database.

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
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
const TEST_API_KEY: &str = "test-api-key-for-integration-tests";

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

/// Build a fresh page and return its normalized name.
async fn create_page(app: &Router, name: &str) -> String {
    let (status, body) = post(app.clone(), "/api/v1/pages", json!({"name": name})).await;
    assert_eq!(status, StatusCode::CREATED, "create page failed: {body}");
    body["name"].as_str().unwrap().to_string()
}

// ═══════════════════════════════════════════════════════════
// Comments
// ═══════════════════════════════════════════════════════════

/// Creating a block with the standard comment metadata should succeed
/// and persist all properties. The block should be returned with the
/// properties map populated so the frontend can render the comment
/// thread without a second round-trip.
#[tokio::test]
async fn create_block_with_comment_properties() -> Result<()> {
    let app = create_test_app().await?;
    let page_name = create_page(&app, "Comments Page").await;

    // Create a regular block to attach the comment to
    let (status, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Important decision"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = parent["id"].as_str().unwrap().to_string();

    // Create a comment as a child of the parent block
    let (status, comment) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Why did we choose this approach?",
            "parentId": parent_id,
            "properties": {
                "type": "comment",
                "resolved": "false",
                "created_by": "agent-007",
                "created_at": "2024-01-15T10:30:00Z"
            }
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "comment create failed: {comment}"
    );
    let comment_id = comment["id"].as_str().unwrap().to_string();

    // The returned block must include all properties
    let props = comment["properties"]
        .as_object()
        .expect("properties is object");
    assert_eq!(props.get("type").and_then(|v| v.as_str()), Some("comment"));
    assert_eq!(
        props.get("resolved").and_then(|v| v.as_str()),
        Some("false")
    );
    assert_eq!(
        props.get("created_by").and_then(|v| v.as_str()),
        Some("agent-007")
    );
    assert_eq!(
        props.get("created_at").and_then(|v| v.as_str()),
        Some("2024-01-15T10:30:00Z")
    );

    // The comment must be a child of the parent block
    assert_eq!(
        comment["parentId"].as_str(),
        Some(parent_id.as_str()),
        "comment should be a child of the parent block"
    );

    // Fetch the page's blocks — both should be present and the
    // properties map should round-trip
    let (_, blocks) = get(
        app.clone(),
        &format!("/api/v1/pages/{}/blocks", urlencoded(&page_name)),
    )
    .await;
    let arr = blocks.as_array().unwrap();
    assert_eq!(arr.len(), 2, "expected parent + comment, got {blocks}");

    // Find the comment in the response and verify its properties
    let comment_from_list = arr
        .iter()
        .find(|b| b["id"] == comment_id)
        .expect("comment not in list");
    let comment_props = comment_from_list["properties"]
        .as_object()
        .expect("comment has properties object");
    assert_eq!(
        comment_props.get("type").and_then(|v| v.as_str()),
        Some("comment")
    );
    assert_eq!(
        comment_props.get("created_by").and_then(|v| v.as_str()),
        Some("agent-007")
    );

    // Verify via the per-block properties endpoint too
    let (status, props_body) = get(app, &format!("/api/v1/blocks/{comment_id}/properties")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(props_body["type"], "comment");
    assert_eq!(props_body["resolved"], "false");

    Ok(())
}

/// Toggling the `resolved` property of a comment should flip the value
/// and persist across reads — this is the core of "resolve / unresolve".
#[tokio::test]
async fn resolve_and_unresolve_comment() -> Result<()> {
    let app = create_test_app().await?;
    let page_name = create_page(&app, "Resolve Comments").await;

    // Create parent + comment
    let (_, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Decision"}),
    )
    .await;
    let parent_id = parent["id"].as_str().unwrap().to_string();

    let (_, comment) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Question",
            "parentId": parent_id,
            "properties": {"type": "comment", "resolved": "false"}
        }),
    )
    .await;
    let comment_id = comment["id"].as_str().unwrap().to_string();

    // Initially false
    let (_, initial) = get(
        app.clone(),
        &format!("/api/v1/blocks/{comment_id}/properties"),
    )
    .await;
    assert_eq!(initial["resolved"], "false");

    // Resolve the comment
    let (status, _) = put(
        app.clone(),
        &format!("/api/v1/blocks/{comment_id}/properties"),
        json!({"key": "resolved", "value": "true"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify it's now true
    let (_, after_resolve) = get(
        app.clone(),
        &format!("/api/v1/blocks/{comment_id}/properties"),
    )
    .await;
    assert_eq!(after_resolve["resolved"], "true");

    // Unresolve
    let (status, _) = put(
        app.clone(),
        &format!("/api/v1/blocks/{comment_id}/properties"),
        json!({"key": "resolved", "value": "false"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify it's false again
    let (_, after_unresolve) = get(app, &format!("/api/v1/blocks/{comment_id}/properties")).await;
    assert_eq!(after_unresolve["resolved"], "false");

    Ok(())
}

/// Replies are nested comments: a comment that has a child which is
/// itself a `type: comment` block. The threaded structure should be
/// preserved when reading the page's blocks.
#[tokio::test]
async fn nested_comment_replies() -> Result<()> {
    let app = create_test_app().await?;
    let page_name = create_page(&app, "Threaded Comments").await;

    // Parent block
    let (_, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "Design doc"}),
    )
    .await;
    let parent_id = parent["id"].as_str().unwrap().to_string();

    // First-level comment
    let (_, c1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Should we add caching?",
            "parentId": parent_id,
            "properties": {"type": "comment", "resolved": "false"}
        }),
    )
    .await;
    let c1_id = c1["id"].as_str().unwrap().to_string();

    // Reply to the first comment
    let (status, reply) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Yes — see ADR-0042",
            "parentId": c1_id,
            "properties": {"type": "comment", "resolved": "false"}
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "reply create failed: {reply}");
    let reply_id = reply["id"].as_str().unwrap().to_string();

    // Reply's parent should be the first comment, not the original block
    assert_eq!(reply["parentId"].as_str(), Some(c1_id.as_str()));

    // Fetch all blocks on the page
    let (_, blocks) = get(
        app,
        &format!("/api/v1/pages/{}/blocks", urlencoded(&page_name)),
    )
    .await;
    let arr = blocks.as_array().unwrap();
    assert_eq!(
        arr.len(),
        3,
        "expected parent + comment + reply, got {blocks}"
    );

    // Build a parent-id index and verify the tree structure
    let by_id: std::collections::HashMap<&str, &Value> =
        arr.iter().map(|b| (b["id"].as_str().unwrap(), b)).collect();

    assert_eq!(by_id[parent_id.as_str()]["parentId"], Value::Null);
    assert_eq!(
        by_id[c1_id.as_str()]["parentId"].as_str(),
        Some(parent_id.as_str())
    );
    assert_eq!(
        by_id[reply_id.as_str()]["parentId"].as_str(),
        Some(c1_id.as_str())
    );

    // All three should have properties on the response
    for (id, label) in [
        (parent_id.as_str(), "parent"),
        (c1_id.as_str(), "comment"),
        (reply_id.as_str(), "reply"),
    ] {
        let props = by_id[id]["properties"]
            .as_object()
            .unwrap_or_else(|| panic!("{label} has no properties object"));
        if label != "parent" {
            assert_eq!(
                props.get("type").and_then(|v| v.as_str()),
                Some("comment"),
                "{label} should be a comment"
            );
        }
    }

    Ok(())
}

/// Creating a block with an unsupported property value (e.g. raw `null`
/// at the top level) should not crash the server — unsupported values
/// are skipped but the block is still created.
#[tokio::test]
async fn create_block_skips_unsupported_property_values() -> Result<()> {
    let app = create_test_app().await?;
    let page_name = create_page(&app, "Mixed Properties").await;

    // null is the only "unsupported" JSON value (PropertyValue::from_json
    // returns None for null/objects). The valid `type` and `resolved`
    // properties should still be persisted.
    let (status, body) = post(
        app,
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Block",
            "properties": {
                "type": "comment",
                "resolved": "false",
                "tag": null
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "block create failed: {body}");

    let props = body["properties"]
        .as_object()
        .expect("properties object on response");
    // The supported ones round-trip
    assert_eq!(props.get("type").and_then(|v| v.as_str()), Some("comment"));
    assert_eq!(
        props.get("resolved").and_then(|v| v.as_str()),
        Some("false")
    );
    // The unsupported null is dropped
    assert!(!props.contains_key("tag"));

    Ok(())
}

/// End-to-end comment workflow: add a comment, reply, resolve, and
/// verify all state is preserved.
#[tokio::test]
async fn comment_full_lifecycle() -> Result<()> {
    let app = create_test_app().await?;
    let page_name = create_page(&app, "Comment Lifecycle").await;

    // Parent block
    let (_, parent) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": page_name, "content": "TODO: refactor auth"}),
    )
    .await;
    let parent_id = parent["id"].as_str().unwrap().to_string();

    // Step 1: Add a comment
    let (status, comment) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "Use JWT here",
            "parentId": parent_id,
            "properties": {
                "type": "comment",
                "resolved": "false",
                "created_by": "alice"
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let comment_id = comment["id"].as_str().unwrap().to_string();

    // Step 2: Reply to the comment
    let (status, reply) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": page_name,
            "content": "+1, refresh tokens are good too",
            "parentId": comment_id,
            "properties": {
                "type": "comment",
                "resolved": "false",
                "created_by": "bob"
            }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let reply_id = reply["id"].as_str().unwrap().to_string();

    // Step 3: Resolve the original comment
    let (status, _) = put(
        app.clone(),
        &format!("/api/v1/blocks/{comment_id}/properties"),
        json!({"key": "resolved", "value": "true"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // Verify final state via page blocks
    let (_, blocks) = get(
        app,
        &format!("/api/v1/pages/{}/blocks", urlencoded(&page_name)),
    )
    .await;
    let arr = blocks.as_array().unwrap();
    assert_eq!(arr.len(), 3, "expected 3 blocks, got {blocks}");

    let by_id: std::collections::HashMap<&str, &Value> =
        arr.iter().map(|b| (b["id"].as_str().unwrap(), b)).collect();

    // Original comment is resolved, reply is not
    let c_resolved = by_id[comment_id.as_str()]["properties"]["resolved"]
        .as_str()
        .unwrap();
    assert_eq!(c_resolved, "true", "comment should be resolved");
    let r_resolved = by_id[reply_id.as_str()]["properties"]["resolved"]
        .as_str()
        .unwrap();
    assert_eq!(r_resolved, "false", "reply should still be unresolved");

    Ok(())
}

/// URL-encode a string for a path segment. Used to handle page names
/// with spaces and special characters.
fn urlencoded(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}
