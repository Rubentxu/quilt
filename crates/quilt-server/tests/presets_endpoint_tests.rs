//! Integration tests for the presets endpoint
//!
//! Tests `GET /api/v1/presets`

mod helpers;

use anyhow::Result;
use axum::http::StatusCode;
use axum::Router;
use quilt_infrastructure::database::sqlite::connection::create_pool;
use std::collections::HashMap;
use std::sync::Once;
use tower::ServiceExt;

// ── Auth setup ────────────────────────────────────────────────────────────────

/// Auth key used for all tests
const TEST_API_KEY: &str = "test-token";

/// Ensures `auth::init` is called exactly once across all tests.
static INIT_AUTH: Once = Once::new();

fn init_auth() {
    INIT_AUTH.call_once(|| {
        quilt_server::middleware::auth::init(TEST_API_KEY.to_string());
    });
}

// ── Test scenarios from spec.md ──────────────────────────────────────────────

/// Build a test app router
async fn build_test_router() -> Result<Router> {
    init_auth();

    let pool = create_pool(":memory:").await?;
    let state = helpers::build_test_app_state(pool).await;
    let app = quilt_server::routes::create_app(state);
    Ok(app)
}

/// Scenario 1: 200 with 9 presets
#[tokio::test]
async fn presets_endpoint_returns_9_presets() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK");

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    let presets = json.get("presets").and_then(|p| p.as_array());
    assert!(presets.is_some(), "Expected 'presets' array in response");

    let presets = presets.unwrap();
    assert_eq!(presets.len(), 9, "Expected 9 V1 presets, got: {}", presets.len());

    Ok(())
}

/// Scenario 2 & 3: Each preset has id, label, description, requiredArgs, keywords
#[tokio::test]
async fn presets_endpoint_preset_shape() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    let presets = json.get("presets").and_then(|p| p.as_array()).unwrap();

    for preset in presets {
        assert!(preset.get("id").is_some(), "Each preset should have an 'id' field");
        assert!(preset.get("label").is_some(), "Each preset should have a 'label' field");
        assert!(preset.get("description").is_some(), "Each preset should have a 'description' field");
        assert!(preset.get("requiredArgs").is_some(), "Each preset should have a 'requiredArgs' field");
        assert!(preset.get("requiredArgs").and_then(|a| a.as_array()).is_some(),
            "requiredArgs should be an array");
        assert!(preset.get("keywords").is_some(), "Each preset should have a 'keywords' field");
    }

    Ok(())
}

/// Scenario 3: requiredArgs distribution
#[tokio::test]
async fn presets_endpoint_required_args_distribution() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    let presets: HashMap<String, Vec<String>> = json.get("presets")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    let id = p.get("id")?.as_str()?.to_string();
                    let args = p.get("requiredArgs")
                        .and_then(|a| a.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|a| a.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();
                    Some((id, args))
                })
                .collect()
        })
        .unwrap_or_default();

    // Status presets (TODO, DOING, WAITING, DONE, NOW) should have empty requiredArgs
    let empty_args: Vec<String> = vec![];
    for preset_id in ["/TODO", "/DOING", "/WAITING", "/DONE", "/NOW"] {
        let args = presets.get(preset_id).unwrap_or(&empty_args);
        assert!(args.is_empty(), "Preset {} should have empty requiredArgs, got: {:?}", preset_id, args);
    }

    // /Scheduled and /Deadline should require "date"
    for preset_id in ["/Scheduled", "/Deadline"] {
        let args = presets.get(preset_id).unwrap_or(&empty_args);
        assert!(args.contains(&"date".to_string()),
            "Preset {} should require 'date' arg, got: {:?}", preset_id, args);
    }

    // /Video and /Image should require "url"
    for preset_id in ["/Video", "/Image"] {
        let args = presets.get(preset_id).unwrap_or(&empty_args);
        assert!(args.contains(&"url".to_string()),
            "Preset {} should require 'url' arg, got: {:?}", preset_id, args);
    }

    Ok(())
}

/// Scenario 4: 401 without Authorization
#[tokio::test]
async fn presets_endpoint_401_without_auth() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED,
        "Expected 401 without Authorization header");

    Ok(())
}

/// Scenario 5: Cache-Control header
#[tokio::test]
async fn presets_endpoint_cache_header() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let cache_header = response
        .headers()
        .get("cache-control")
        .and_then(|v| v.to_str().ok());
    assert_eq!(cache_header, Some("private, max-age=300"),
        "Expected cache-control header to be 'private, max-age=300'");

    Ok(())
}

/// Scenario 6: Response has count field
#[tokio::test]
async fn presets_endpoint_response_has_count() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    let count = json.get("count").and_then(|c| c.as_i64()).unwrap_or(-1);
    assert_eq!(count, 9, "Expected count to be 9, got: {}", count);

    Ok(())
}

/// Scenario 7: ?version=v2 is silently ignored (V1 list returned)
#[tokio::test]
async fn presets_endpoint_version_param_ignored() -> Result<()> {
    let app = build_test_router().await?;

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/api/v1/presets?version=v2")
                .method("GET")
                .header("Authorization", "Bearer test-token")
                .body(axum::body::Body::empty())?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK, "Expected 200 OK even with ?version=v2");

    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024).await?;
    let json: serde_json::Value = serde_json::from_slice(&body)?;

    // Should still return 9 presets (V1)
    let presets = json.get("presets").and_then(|p| p.as_array()).unwrap();
    assert_eq!(presets.len(), 9, "Expected 9 V1 presets even with ?version=v2");

    Ok(())
}
