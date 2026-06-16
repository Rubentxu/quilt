//! Integration tests for `GET/POST /api/v1/agents` (Agent Room V1).
//!
//! Tests stand up the full Axum router with an in-memory SQLite DB
//! and exercise the 4 handlers end-to-end. The agent room is wired
//! with the default registry (one type: `decay-annotator`).
//!
//! Contract:
//! 1. Auth — every route returns 401 without a Bearer.
//! 2. List — empty by default; includes the spawn result.
//! 3. Spawn — 201 with Queued DTO; 400 for unknown type.
//! 4. Get — 200 / 404.
//! 5. Cancel — 200, idempotent, transitions to Cancelled.
//! 6. Filter — `?status=`, `?type=` query params work.

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use serde_json::Value;
use std::sync::Once;
use tower::util::ServiceExt;

mod helpers;
use helpers::build_test_app_state_with_agents;

const TEST_KEY: &str = "test-key-123";

static INIT: std::sync::Once = std::sync::Once::new();

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

async fn json_body(res: axum::response::Response) -> Result<Value> {
    let body = axum::body::to_bytes(res.into_body(), 8192).await?;
    let json: Value = serde_json::from_slice(&body)?;
    Ok(json)
}

async fn empty_app() -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;
    let (state, _lc, _reg) = build_test_app_state_with_agents(pool).await;
    Ok(quilt_server::routes::create_app(state))
}

#[tokio::test]
async fn list_requires_auth() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/agents")
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn list_empty_on_cold_graph() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 0);
    assert!(json["agents"].as_array().unwrap().is_empty());
    Ok(())
}

#[tokio::test]
async fn spawn_unknown_type_400() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"agentType":"imaginary"}"#))?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    let json = json_body(res).await?;
    assert_eq!(json["code"], "BAD_REQUEST");
    Ok(())
}

#[tokio::test]
async fn spawn_then_list() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"agentType":"decay-annotator"}"#))?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::CREATED);
    let spawned = json_body(res).await?;
    assert_eq!(spawned["agentType"], "decay-annotator");
    assert_eq!(spawned["status"], "Queued");
    assert!(!spawned["id"].as_str().unwrap().is_empty());

    // List now has 1 entry.
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let listed = json_body(res).await?;
    assert_eq!(listed["total"].as_u64().unwrap(), 1);
    assert_eq!(listed["agents"][0]["id"], spawned["id"]);
    Ok(())
}

#[tokio::test]
async fn get_unknown_404() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents/agent-nonexistent")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let json = json_body(res).await?;
    assert_eq!(json["code"], "NOT_FOUND");
    Ok(())
}

#[tokio::test]
async fn cancel_running_transitions() -> Result<()> {
    // Build the lifecycle and registry manually so we can
    // promote a run to Running before cancelling. The HTTP
    // path alone (no time to wait for the worker) only gets
    // us as far as Queued.
    use quilt_analysis::agent_room::SpawnAgentRequest;
    use quilt_server::handlers::agent_room::build_lifecycle_and_registry;
    use std::sync::Arc;
    init_auth();

    let pool = create_pool(":memory:").await?;
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();
    let (lifecycle, registry) = build_lifecycle_and_registry(block_repo, page_repo);
    let known = registry.list_types();
    let dto = lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await
        .unwrap();
    let id = dto.id.clone();
    // Promote Queued → Running.
    let _ = lifecycle
        .try_promote_to_running(quilt_domain::value_objects::Uuid::parse_str(&id).unwrap());

    // Wire AppState with these so the handlers can find them.
    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(Arc::new(lifecycle)),
        Some(registry),
    );
    let app = quilt_server::routes::create_app(state);

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/agents/{id}/cancel"))
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["status"], "Cancelled");
    Ok(())
}

#[tokio::test]
async fn cancel_idempotent_on_completed() -> Result<()> {
    // Spawn, promote to Running, mark Completed, then
    // cancel — must return 200 and the status MUST stay
    // Completed.
    use quilt_analysis::agent_room::SpawnAgentRequest;
    use quilt_server::handlers::agent_room::build_lifecycle_and_registry;
    use std::sync::Arc;
    init_auth();

    let pool = create_pool(":memory:").await?;
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();
    let (lifecycle, registry) = build_lifecycle_and_registry(block_repo, page_repo);
    let known = registry.list_types();
    let dto = lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await
        .unwrap();
    let id = dto.id.clone();
    let uuid = quilt_domain::value_objects::Uuid::parse_str(&id).unwrap();
    let _ = lifecycle.try_promote_to_running(uuid);
    lifecycle.complete(uuid, "ok".to_string(), 0).await.unwrap();

    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(Arc::new(lifecycle)),
        Some(registry),
    );
    let app = quilt_server::routes::create_app(state);

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri(&format!("/api/v1/agents/{id}/cancel"))
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["status"], "Completed");
    Ok(())
}

#[tokio::test]
async fn filter_by_status() -> Result<()> {
    use quilt_analysis::agent_room::SpawnAgentRequest;
    use quilt_server::handlers::agent_room::build_lifecycle_and_registry;
    use std::sync::Arc;
    init_auth();

    let pool = create_pool(":memory:").await?;
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();
    let (lifecycle, registry) = build_lifecycle_and_registry(block_repo, page_repo);
    let known = registry.list_types();
    let dto = lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await
        .unwrap();
    let id = dto.id.clone();
    let uuid = quilt_domain::value_objects::Uuid::parse_str(&id).unwrap();
    let _ = lifecycle.try_promote_to_running(uuid);

    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(Arc::new(lifecycle)),
        Some(registry),
    );
    let app = quilt_server::routes::create_app(state);

    // Filter by Running.
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents?status=Running")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 1);
    assert_eq!(json["agents"][0]["status"], "Running");

    // Filter by Cancelled → empty.
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents?status=Cancelled")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 0);
    Ok(())
}

#[tokio::test]
async fn filter_by_context_page() -> Result<()> {
    use quilt_analysis::agent_room::SpawnAgentRequest;
    use quilt_server::handlers::agent_room::build_lifecycle_and_registry;
    use std::sync::Arc;
    init_auth();

    let pool = create_pool(":memory:").await?;
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();
    let (lifecycle, registry) = build_lifecycle_and_registry(block_repo, page_repo);
    let known = registry.list_types();

    // Spawn two agents with different context pages
    lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: Some("page/foo".to_string()),
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await?;

    lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: Some("page/bar".to_string()),
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await?;

    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(Arc::new(lifecycle)),
        Some(registry),
    );
    let app = quilt_server::routes::create_app(state);

    // Filter by page/foo
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents?context_page=page/foo")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 1);
    assert_eq!(json["agents"][0]["contextPage"].as_str().unwrap(), "page/foo");

    // Filter by nonexistent page → empty
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents?context_page=page/nonexistent")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 0);
    Ok(())
}

#[tokio::test]
async fn filter_by_context_page_null() -> Result<()> {
    use quilt_analysis::agent_room::SpawnAgentRequest;
    use quilt_server::handlers::agent_room::build_lifecycle_and_registry;
    use std::sync::Arc;
    init_auth();

    let pool = create_pool(":memory:").await?;
    let (state, _block_repo, _page_repo, _, _, _, _, _, _, _) =
        helpers::build_test_app_state_with_repos(pool).await;
    let block_repo = state.repos.block.clone();
    let page_repo = state.repos.page.clone();
    let (lifecycle, registry) = build_lifecycle_and_registry(block_repo, page_repo);
    let known = registry.list_types();

    // One with context_page, one without
    lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: Some("page/with".to_string()),
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await?;

    lifecycle
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await?;

    let state = quilt_server::state::AppState::new_with_repos_and_agents(
        state.repos.clone(),
        state.search_service.clone(),
        state.search_index.clone(),
        state.ref_service.clone(),
        state.services.clone(),
        state.projection_resolver.clone(),
        state.preset_registry.clone(),
        Some(Arc::new(lifecycle)),
        Some(registry),
    );
    let app = quilt_server::routes::create_app(state);

    // Filter by no context_page (null) → only matches the second agent
    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/agents?context_page=")
                .body(Body::empty())?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::OK);
    let json = json_body(res).await?;
    assert_eq!(json["total"].as_u64().unwrap(), 1);
    assert!(json["agents"][0]["contextPage"].is_null());
    Ok(())
}

#[tokio::test]
async fn list_includes_context_page_in_response() -> Result<()> {
    let app = empty_app().await?;
    let res = app
        .clone()
        .oneshot(auth_header(
            Request::builder()
                .method("POST")
                .uri("/api/v1/agents")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"agentType":"decay-annotator","contextPage":"page/test"}"#))?,
        ))
        .await?;
    assert_eq!(res.status(), StatusCode::CREATED);
    let spawned = json_body(res).await?;
    assert_eq!(spawned["agentType"], "decay-annotator");
    assert_eq!(spawned["contextPage"].as_str().unwrap(), "page/test");
    Ok(())
}
