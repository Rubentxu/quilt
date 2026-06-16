//! Integration tests for `GET /api/v1/graph/lens` (Graph Lens V1 —
//! subgraph endpoint).
//!
//! Tests stand up the full Axum router with an in-memory SQLite DB
//! and exercise the handler end-to-end — auth, focus parsing, depth
//! bounds, traversal correctness, and the empty-focus contract.
//!
//! Behaviour tested (TDD — these are the contract):
//!
//! 1. `focus=block:<uuid>` returns the focus block and N hops of
//!    children + forward refs.
//! 2. `focus=page:<name>` returns all root blocks of the page plus
//!    their first-hop connections.
//! 3. `focus=property:<key>` returns all blocks that have that
//!    property key (depth forced to 1).
//! 4. `depth` parameter is clamped to 1..=3.
//! 5. Invalid focus format → 400.
//! 6. Unknown block/page → 200 with empty graph (graceful).
//! 7. No `focus` → 200 with empty graph.
//! 8. Auth required.

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_domain::entities::{Block, BlockCreate, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
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

fn auth_header(mut req: Request<Body>) -> Request<Body> {
    req.headers_mut().insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {TEST_KEY}")).unwrap(),
    );
    req
}

/// Build the app with a fresh in-memory DB (no seeded data).
async fn empty_app() -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    Ok(quilt_server::routes::create_app(state))
}

/// Build the app, seed `n` sibling blocks on page `p`. Each block
/// is a root block (parent_id = None) so traversal from `page:p` at
/// depth 1 yields all of them.
async fn app_with_n_roots(n: u32) -> Result<(axum::Router, Uuid)> {
    init_auth();
    let pool = create_pool(":memory:").await?;

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

    for i in 0..n {
        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: format!("root-{i}"),
            parent_id: None,
            order: i as f64,
            marker: None,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            properties: HashMap::new(),
        })
        .unwrap();
        block_repo.insert(&block).await.unwrap();
    }

    Ok((quilt_server::routes::create_app(state), page.id.into()))
}

/// Build a tree: `root` with two children `c1`, `c2`, where `c1`
/// has its own child `gc1`. Returns ids: (root, c1, c2, gc1).
async fn app_with_tree() -> Result<(axum::Router, Vec<Uuid>)> {
    init_auth();
    let pool = create_pool(":memory:").await?;

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

    let mk = |content: &str, parent: Option<Uuid>, order: f64| {
        let content = content.to_string();
        let parent = parent.map(Into::into);
        async move {
            Block::new(BlockCreate {
                page_id: page.id,
                content,
                parent_id: parent,
                order,
                marker: None,
                format: BlockFormat::Markdown,
                block_type: BlockType::Paragraph,
                properties: HashMap::new(),
            })
            .unwrap()
        }
    };

    let root = mk("root", None, 0.0).await;
    let root_id: Uuid = root.id.into();
    block_repo.insert(&root).await.unwrap();

    let c1 = mk("c1", Some(root_id), 1.0).await;
    let c1_id: Uuid = c1.id.into();
    block_repo.insert(&c1).await.unwrap();

    let c2 = mk("c2", Some(root_id), 2.0).await;
    let c2_id: Uuid = c2.id.into();
    block_repo.insert(&c2).await.unwrap();

    let gc1 = mk("gc1", Some(c1_id), 1.0).await;
    let gc1_id: Uuid = gc1.id.into();
    block_repo.insert(&gc1).await.unwrap();

    Ok((
        quilt_server::routes::create_app(state),
        vec![root_id, c1_id, c2_id, gc1_id],
    ))
}

// ── Auth ────────────────────────────────────────────────────────────

#[tokio::test]
async fn get_lens_without_authorization_returns_401() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=block:00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn get_lens_with_wrong_bearer_returns_401() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=block:00000000-0000-0000-0000-000000000000")
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
async fn get_lens_no_focus_returns_200_empty_graph() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["focus"], serde_json::Value::Null);
    assert_eq!(json["depth"], serde_json::json!(1));
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

#[tokio::test]
async fn get_lens_unknown_block_returns_200_empty_graph() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=block:11111111-1111-1111-1111-111111111111&depth=2")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Graceful: focus echoed back, but no nodes/edges.
    assert_eq!(json["focus"], "block:11111111-1111-1111-1111-111111111111");
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

#[tokio::test]
async fn get_lens_unknown_page_returns_200_empty_graph() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=page:nonexistent")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["focus"], "page:nonexistent");
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

#[tokio::test]
async fn get_lens_invalid_focus_format_returns_400() {
    let app = empty_app().await.unwrap();
    // "garbage" has no recognized prefix (block:/page:/property:/tag:).
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=garbage")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_lens_invalid_block_uuid_returns_400() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=block:not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_lens_depth_out_of_range_returns_400() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=page:p&depth=99")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_lens_depth_zero_returns_400() {
    // depth=0 is below the contract floor (1..=3).
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=page:p&depth=0")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

// ── Block focus: traversal correctness ──────────────────────────────

#[tokio::test]
async fn get_lens_block_depth_1_returns_block_only() {
    let (app, ids) = app_with_tree().await.unwrap();
    let root = ids[0];
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/graph/lens?focus=block:{root}&depth=1"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // depth=1 from root → just the root. Children (c1, c2) NOT included.
    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1, "depth=1 should only return the focus block");
    assert_eq!(nodes[0]["id"], root.to_string());
    let edges = json["edges"].as_array().unwrap();
    assert!(edges.is_empty(), "depth=1 should produce no edges");
}

#[tokio::test]
async fn get_lens_block_depth_2_returns_block_and_children() {
    let (app, ids) = app_with_tree().await.unwrap();
    let root = ids[0];
    let c1 = ids[1];
    let c2 = ids[2];

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/graph/lens?focus=block:{root}&depth=2"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // depth=2 from root → root + c1 + c2 (children). NOT gc1.
    let node_ids: Vec<&str> = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(node_ids.len(), 3, "depth=2 should reach immediate children");
    assert!(node_ids.contains(&root.to_string().as_str()));
    assert!(node_ids.contains(&c1.to_string().as_str()));
    assert!(node_ids.contains(&c2.to_string().as_str()));

    // Two parent→child edges.
    let edges = json["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 2, "expected 2 parent→child edges");
    for edge in edges {
        assert_eq!(edge["from"], root.to_string());
        let to = edge["to"].as_str().unwrap();
        assert!(to == c1.to_string() || to == c2.to_string());
    }
}

#[tokio::test]
async fn get_lens_block_depth_3_returns_grandchildren() {
    let (app, ids) = app_with_tree().await.unwrap();
    let root = ids[0];
    let gc1 = ids[3];

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/graph/lens?focus=block:{root}&depth=3"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // depth=3 from root → root + c1 + c2 + gc1.
    let node_ids: Vec<&str> = json["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert_eq!(node_ids.len(), 4, "depth=3 should reach grandchildren");
    assert!(node_ids.contains(&gc1.to_string().as_str()));
}

// ── Page focus ─────────────────────────────────────────────────────

#[tokio::test]
async fn get_lens_page_returns_all_roots_and_edges() {
    let (app, _page_id) = app_with_n_roots(3).await.unwrap();

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=page:p&depth=1")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // 3 root blocks, no edges between roots (no parent→child).
    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 3);
    let edges = json["edges"].as_array().unwrap();
    assert!(edges.is_empty());
}

#[tokio::test]
async fn get_lens_page_node_includes_page_name() {
    let (app, _page_id) = app_with_n_roots(1).await.unwrap();

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=page:p&depth=1")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let node = &json["nodes"][0];
    assert_eq!(node["pageName"], "p");
}

// ── Property focus ─────────────────────────────────────────────────

#[tokio::test]
async fn get_lens_property_returns_blocks_with_that_key() -> Result<()> {
    init_auth();
    let pool = create_pool(":memory:").await?;

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

    // Two blocks with property `status`, one without.
    for (i, has_prop) in [true, true, false].iter().enumerate() {
        let mut props = HashMap::new();
        if *has_prop {
            props.insert(
                "status".to_string(),
                PropertyValue::string(if i == 0 { "draft" } else { "published" }),
            );
        }
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

    let app = quilt_server::routes::create_app(state);

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=property:status")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let nodes = json["nodes"].as_array().unwrap();
    assert_eq!(
        nodes.len(),
        2,
        "property:status should match the 2 blocks with that key"
    );
    for n in nodes {
        assert_eq!(n["pageName"], "p");
    }
    Ok(())
}

#[tokio::test]
async fn get_lens_property_empty_db_returns_empty_graph() {
    let app = empty_app().await.unwrap();
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/lens?focus=property:status")
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["nodes"], serde_json::json!([]));
    assert_eq!(json["edges"], serde_json::json!([]));
}

// ── Default depth ──────────────────────────────────────────────────

#[tokio::test]
async fn get_lens_no_depth_defaults_to_1() {
    let (app, ids) = app_with_tree().await.unwrap();
    let root = ids[0];

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/graph/lens?focus=block:{root}"))
                .body(Body::empty())
                .unwrap(),
        ))
        .await
        .unwrap();
    let body = axum::body::to_bytes(res.into_body(), 16384).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Default depth=1 → only the focus block.
    assert_eq!(json["depth"], serde_json::json!(1));
    assert_eq!(json["nodes"].as_array().unwrap().len(), 1);
}
