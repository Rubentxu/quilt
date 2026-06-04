//! Integration tests for the Templates system (ADR-0003).
//!
//! A template is a regular page whose name starts with `template/`.
//! The `POST /api/v1/pages/from-template` endpoint clones the template's
//! block tree into a new page, substituting `{{var}}` / `${var}` placeholders.
//!
//! These tests are self-contained: they copy the in-memory DB + Axum helpers
//! from `api_edge_cases.rs` so they can run independently (cargo runs each
//! integration test file as a separate binary).

use anyhow::Result;
use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use std::sync::{Arc, Once};
use tokio::sync::RwLock;
use tower::ServiceExt;

use quilt_application::services::ref_service::RefService;
use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;
use quilt_search::SearchIndexManager;
use quilt_server::handlers::pages::substitute_placeholders;

// ═══════════════════════════════════════════════════════════
//  Test harness (mirrors `api_edge_cases.rs` helpers)
// ═══════════════════════════════════════════════════════════

const TEST_API_KEY: &str = "test-api-key-for-templates";

static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

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

fn api_path(segments: &[&str]) -> String {
    let encoded: Vec<String> = segments.iter().map(|s| url_encode_path(s)).collect();
    encoded.join("/")
}

fn url_encode_path(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

/// Build a page-name path segment with `/` percent-encoded as `%2F`.
///
/// This is required when the page name contains a `/` (e.g. template pages
/// like `template/foo`): axum's `:name` pattern only matches a single path
/// segment, so the slash must be encoded or the route won't resolve.
fn url_encode_page_name(name: &str) -> String {
    let mut encoded = String::with_capacity(name.len());
    for byte in name.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            b'/' => encoded.push_str("%2F"),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

// ═══════════════════════════════════════════════════════════
//  Template detection / page naming
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn template_named_page_can_be_created() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = post(app, "/api/v1/pages", json!({"name": "template/daily-note"})).await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "template page should be creatable: {body}"
    );
    assert_eq!(body["name"], "template/daily-note");
    Ok(())
}

#[tokio::test]
async fn non_template_slash_page_name_rejected() -> Result<()> {
    let app = create_test_app().await?;

    let (status, _body) = post(app, "/api/v1/pages", json!({"name": "my/nested/page"})).await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::INTERNAL_SERVER_ERROR,
        "regular page with '/' should be rejected, got {status}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════
//  create_page_from_template — happy paths
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn create_page_from_basic_template() -> Result<()> {
    let app = create_test_app().await?;

    // 1. Create a template page
    let (status, _tpl) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/basic"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let template_name = "template/basic".to_string();

    // 2. Add some blocks to the template
    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": template_name, "content": "First block"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": template_name, "content": "Second block"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // 3. Create a new page from the template
    let (status, body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": template_name,
            "pageName": "My New Page",
        }),
    )
    .await;

    assert_eq!(
        status,
        StatusCode::CREATED,
        "from-template create failed: {body}"
    );
    assert_eq!(body["blocksCreated"], 2);
    assert_eq!(body["page"]["name"], "my new page");

    // 4. The new page should have the cloned blocks
    let blocks_path = api_path(&["/api/v1/pages", "my new page", "blocks"]);
    let (status, blocks_body) = get(app, &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let blocks = blocks_body.as_array().unwrap();
    assert_eq!(
        blocks.len(),
        2,
        "cloned page should have both template blocks"
    );

    let contents: Vec<&str> = blocks
        .iter()
        .map(|b| b["content"].as_str().unwrap())
        .collect();
    assert!(contents.contains(&"First block"));
    assert!(contents.contains(&"Second block"));

    Ok(())
}

#[tokio::test]
async fn template_blocks_clone_preserves_tree_structure() -> Result<()> {
    let app = create_test_app().await?;

    // Create template + a parent block + two child blocks
    let (status, _tpl) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/tree"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, parent_body) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": "template/tree", "content": "Parent"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let parent_id = parent_body["id"].as_str().unwrap().to_string();

    let (_status, _child1) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/tree",
            "content": "Child 1",
            "parentId": parent_id,
        }),
    )
    .await;

    let (_status, _child2) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/tree",
            "content": "Child 2",
            "parentId": parent_id,
        }),
    )
    .await;

    // Clone
    let (status, body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/tree",
            "pageName": "Cloned Tree",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "clone failed: {body}");
    assert_eq!(body["blocksCreated"], 3);

    // Verify the cloned tree
    let blocks_path = api_path(&["/api/v1/pages", "cloned tree", "blocks"]);
    let (status, blocks_body) = get(app, &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let blocks = blocks_body.as_array().unwrap();
    assert_eq!(blocks.len(), 3);

    // The cloned parent should be a root block (parentId = null) and
    // there should be two children referencing it.
    let roots: Vec<&Value> = blocks.iter().filter(|b| b["parentId"].is_null()).collect();
    assert_eq!(roots.len(), 1, "expected exactly one root block");
    let cloned_parent_id = roots[0]["id"].as_str().unwrap();

    let children: Vec<&Value> = blocks
        .iter()
        .filter(|b| b["parentId"].as_str() == Some(cloned_parent_id))
        .collect();
    assert_eq!(
        children.len(),
        2,
        "expected two children under the cloned parent"
    );
    let child_contents: Vec<&str> = children
        .iter()
        .map(|b| b["content"].as_str().unwrap())
        .collect();
    assert!(child_contents.contains(&"Child 1"));
    assert!(child_contents.contains(&"Child 2"));

    // Cloned blocks must have new UUIDs (not the originals).
    assert_ne!(cloned_parent_id, parent_id);

    Ok(())
}

#[tokio::test]
async fn template_placeholders_replaced() -> Result<()> {
    let app = create_test_app().await?;

    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/vars"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/vars",
            "content": "Title: {{title}} | Date: {{date}} | Greeting: ${name}",
        }),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/vars",
            "content": "Author: {{author}}",
        }),
    )
    .await;

    let (status, body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/vars",
            "pageName": "Placeholder Demo",
            "title": "Placeholder Demo",
            "variables": {"author": "Ada"},
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "clone failed: {body}");

    let blocks_path = api_path(&["/api/v1/pages", "placeholder demo", "blocks"]);
    let (status, blocks_body) = get(app, &blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let blocks = blocks_body.as_array().unwrap();
    assert_eq!(blocks.len(), 2);

    let contents: Vec<String> = blocks
        .iter()
        .map(|b| b["content"].as_str().unwrap().to_string())
        .collect();
    // Built-in variables: {{title}}, {{date}}, ${name} all become the page name / today.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert!(
        contents.iter().any(|c| c.contains(&format!(
            "Title: placeholder demo | Date: {today} | Greeting: placeholder demo"
        ))),
        "expected built-in substitution; got {contents:?}"
    );
    // User-supplied variable.
    assert!(
        contents.iter().any(|c| c == "Author: Ada"),
        "expected user var substitution; got {contents:?}"
    );

    Ok(())
}

#[tokio::test]
async fn unknown_placeholders_left_intact() -> Result<()> {
    let app = create_test_app().await?;

    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/partial"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/partial",
            "content": "Known: {{title}} | Unknown: {{foo}}",
        }),
    )
    .await;

    let (status, _body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/partial",
            "pageName": "Partial Page",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let blocks_path = api_path(&["/api/v1/pages", "partial page", "blocks"]);
    let (_status, blocks_body) = get(app, &blocks_path).await;
    let blocks = blocks_body.as_array().unwrap();
    let content = blocks[0]["content"].as_str().unwrap();
    assert_eq!(content, "Known: partial page | Unknown: {{foo}}");

    Ok(())
}

#[tokio::test]
async fn from_template_does_not_mutate_template() -> Result<()> {
    let app = create_test_app().await?;

    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/isolation"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({"pageName": "template/isolation", "content": "Template body {{title}}"}),
    )
    .await;

    // Clone once
    let (status, _body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/isolation",
            "pageName": "First",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Clone twice — template should still be untouched
    let (status, _body) = post(
        app.clone(),
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/isolation",
            "pageName": "Second",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Template page should still have exactly one block with the raw placeholder.
    // Note: the `/` in the page name must be URL-encoded for axum's `:name`
    // route segment to match the entire `template/isolation` string.
    let tpl_blocks_path = format!(
        "/api/v1/pages/{}/blocks",
        url_encode_page_name("template/isolation")
    );
    let (status, tpl_body) = get(app, &tpl_blocks_path).await;
    assert_eq!(status, StatusCode::OK);
    let blocks = tpl_body.as_array().unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0]["content"], "Template body {{title}}");

    Ok(())
}

// ═══════════════════════════════════════════════════════════
//  create_page_from_template — error paths
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn create_from_nonexistent_template_returns_404() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = post(
        app,
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/does-not-exist",
            "pageName": "Anything",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "expected 404, got {body}");
    Ok(())
}

#[tokio::test]
async fn create_from_non_template_page_returns_400() -> Result<()> {
    let app = create_test_app().await?;

    // Create a regular page (no `template/` prefix)
    let (status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "regular-page"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    // Attempt to use it as a template → 400
    let (status, body) = post(
        app,
        "/api/v1/pages/from-template",
        json!({
            "templateName": "regular-page",
            "pageName": "Anything",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "expected 400, got {body}");
    Ok(())
}

#[tokio::test]
async fn create_from_template_with_existing_target_page_returns_error() -> Result<()> {
    let app = create_test_app().await?;

    // Template
    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/dup-target"}),
    )
    .await;

    // Pre-existing target page
    let (status, _) = post(app.clone(), "/api/v1/pages", json!({"name": "Cloned"})).await;
    assert_eq!(status, StatusCode::CREATED);

    // Second creation should fail (page name uniqueness)
    let (status, body) = post(
        app,
        "/api/v1/pages/from-template",
        json!({
            "templateName": "template/dup-target",
            "pageName": "Cloned",
        }),
    )
    .await;
    assert!(
        status == StatusCode::CONFLICT
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::INTERNAL_SERVER_ERROR,
        "expected uniqueness error, got {status}: {body}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════
//  Route precedence: /from-template must NOT be matched by /:name
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn from_template_literal_does_not_collide_with_page_name_route() -> Result<()> {
    // Without proper route ordering, `GET /api/v1/pages/from-template` would
    // resolve to the page-by-name handler and 404 (no such page). With the
    // literal route registered first, the wrong method returns 405.
    let app = create_test_app().await?;

    let (status, _body) = get(app, "/api/v1/pages/from-template").await;
    // Either: 405 Method Not Allowed (literal route, wrong verb) or 404
    // (no page named "from-template"). Either is acceptable as long as we
    // don't get a successful page payload.
    assert!(
        status == StatusCode::METHOD_NOT_ALLOWED || status == StatusCode::NOT_FOUND,
        "expected 405 or 404, got {status}"
    );
    Ok(())
}

// ═══════════════════════════════════════════════════════════
//  Unit tests for the placeholder helper
// ═══════════════════════════════════════════════════════════

#[test]
fn substitute_builtins_use_page_name() {
    let out = substitute_placeholders("Hello {{title}} / ${name}", None, "demo");
    assert_eq!(out, "Hello demo / demo");
}

#[test]
fn substitute_date_is_iso_today() {
    let out = substitute_placeholders("Today: {{date}}", None, "x");
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    assert_eq!(out, format!("Today: {today}"));
}

#[test]
fn substitute_user_variables_override() {
    use std::collections::HashMap;
    let mut vars = HashMap::new();
    vars.insert("author".to_string(), "Grace".to_string());
    let out = substitute_placeholders("By {{author}} & ${author}", Some(&vars), "x");
    assert_eq!(out, "By Grace & Grace");
}

#[test]
fn substitute_user_variables_preserve_unknown_placeholders() {
    use std::collections::HashMap;
    let mut vars = HashMap::new();
    vars.insert("author".to_string(), "Grace".to_string());
    let out = substitute_placeholders("{{author}} and {{ghost}}", Some(&vars), "x");
    assert_eq!(out, "Grace and {{ghost}}");
}

#[test]
fn substitute_ignores_empty_variable_keys() {
    use std::collections::HashMap;
    let mut vars = HashMap::new();
    vars.insert(String::new(), "should not appear".to_string());
    let out = substitute_placeholders("Hello {{title}}", Some(&vars), "demo");
    assert_eq!(out, "Hello demo");
}

// ═══════════════════════════════════════════════════════════
//  GET /api/v1/templates — ADR-0007
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn list_templates_returns_empty_when_no_template_pages() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/templates").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body.as_array().unwrap().is_empty(),
        "expected empty array, got {body}"
    );
    Ok(())
}

#[tokio::test]
async fn list_templates_returns_card_metadata() -> Result<()> {
    let app = create_test_app().await?;

    // Create a template page with card-shape metadata
    let (status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/reference"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/reference",
            "content": "",
            "properties": {
                "card-shape": "reference",
                "icon": "🔗",
            },
        }),
    )
    .await;

    let (status, body) = get(app, "/api/v1/templates").await;
    assert_eq!(status, StatusCode::OK);
    let templates = body.as_array().unwrap();
    assert_eq!(templates.len(), 1, "expected one template, got {body}");

    let t = &templates[0];
    assert_eq!(t["name"], "reference");
    assert_eq!(t["full_name"], "template/reference");
    assert_eq!(t["block_count"], 1);
    assert_eq!(t["card_shape"], "reference");
    assert_eq!(t["icon"], "🔗");
    Ok(())
}

#[tokio::test]
async fn list_templates_filters_only_template_pages() -> Result<()> {
    let app = create_test_app().await?;

    // Mix of regular and template pages
    post(app.clone(), "/api/v1/pages", json!({"name": "regular"})).await;
    post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/meeting-notes"}),
    )
    .await;
    post(app.clone(), "/api/v1/pages", json!({"name": "templated"})).await; // not a template
    post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/bare"}),
    )
    .await;

    let (status, body) = get(app, "/api/v1/templates").await;
    assert_eq!(status, StatusCode::OK);
    let templates = body.as_array().unwrap();
    assert_eq!(
        templates.len(),
        2,
        "expected only the two template/ pages, got {body}"
    );

    let names: Vec<&str> = templates
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"meeting-notes"));
    assert!(names.contains(&"bare"));
    assert!(!names.contains(&"regular"));
    assert!(!names.contains(&"templated"));
    Ok(())
}

#[tokio::test]
async fn list_templates_defaults_to_inline_shape_when_no_card_shape() -> Result<()> {
    let app = create_test_app().await?;

    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/loose"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/loose",
            "content": "no metadata here",
        }),
    )
    .await;

    let (status, body) = get(app, "/api/v1/templates").await;
    assert_eq!(status, StatusCode::OK);
    let t = &body.as_array().unwrap()[0];
    assert_eq!(t["name"], "loose");
    assert_eq!(
        t["card_shape"], "inline",
        "missing card-shape should default to inline"
    );
    assert!(t["icon"].is_null());
    Ok(())
}

// ═══════════════════════════════════════════════════════════
//  GET /api/v1/templates/:name/schema — ADR-0007
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_template_schema_not_found() -> Result<()> {
    let app = create_test_app().await?;

    let (status, body) = get(app, "/api/v1/templates/does-not-exist/schema").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "NOT_FOUND");
    Ok(())
}

#[tokio::test]
async fn get_template_schema_returns_full_metadata() -> Result<()> {
    let app = create_test_app().await?;

    // Create template page with metadata and a user-facing block
    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/contact"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/contact",
            "content": "",
            "properties": {
                "card-shape": "reference",
                "icon": "👤",
            },
        }),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/contact",
            "content": "Alice Doe",
            "properties": {
                "name": "Alice Doe",
                "email": "alice@example.com",
                "priority": 1,
            },
        }),
    )
    .await;

    let (status, body) = get(app, "/api/v1/templates/contact/schema").await;
    assert_eq!(status, StatusCode::OK, "expected 200, got {status}: {body}");

    assert_eq!(body["name"], "contact");
    assert_eq!(body["card_shape"], "reference");
    assert_eq!(body["icon"], "👤");
    assert_eq!(body["block_count"], 2);

    // Properties exclude reserved keys (card-shape, icon, cssclass)
    let properties = body["properties"].as_array().unwrap();
    let keys: Vec<&str> = properties
        .iter()
        .map(|p| p["key"].as_str().unwrap())
        .collect();
    assert!(keys.contains(&"name"));
    assert!(keys.contains(&"email"));
    assert!(keys.contains(&"priority"));
    assert!(!keys.contains(&"card-shape"));
    assert!(!keys.contains(&"icon"));

    // Type hints are preserved
    let name_prop = properties.iter().find(|p| p["key"] == "name").unwrap();
    assert_eq!(name_prop["type"], "string");
    assert_eq!(name_prop["value"], "Alice Doe");

    let priority_prop = properties.iter().find(|p| p["key"] == "priority").unwrap();
    assert_eq!(priority_prop["type"], "integer");
    Ok(())
}

#[tokio::test]
async fn get_template_schema_reserves_block_level_keys() -> Result<()> {
    let app = create_test_app().await?;

    let (_status, _) = post(
        app.clone(),
        "/api/v1/pages",
        json!({"name": "template/clean"}),
    )
    .await;
    let (_status, _) = post(
        app.clone(),
        "/api/v1/blocks",
        json!({
            "pageName": "template/clean",
            "content": "test",
            "properties": {
                "template": "other",
                "type": "reference",
                "collapsed": true,
                "author": "claude",
            },
        }),
    )
    .await;

    let (status, body) = get(app, "/api/v1/templates/clean/schema").await;
    assert_eq!(status, StatusCode::OK);

    let properties = body["properties"].as_array().unwrap();
    let keys: Vec<&str> = properties
        .iter()
        .map(|p| p["key"].as_str().unwrap())
        .collect();
    assert!(
        !keys.contains(&"template"),
        "reserved key 'template' should be excluded"
    );
    assert!(
        !keys.contains(&"type"),
        "reserved key 'type' should be excluded"
    );
    assert!(
        !keys.contains(&"collapsed"),
        "reserved key 'collapsed' should be excluded"
    );
    assert!(keys.contains(&"author"));
    Ok(())
}
