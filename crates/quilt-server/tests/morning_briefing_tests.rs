//! Integration tests for `GET /api/v1/cognitive/morning-briefing` (Morning Briefing).
//!
//! Tests stand up the full Axum router with an in-memory SQLite DB
//! and exercise the handler end-to-end.
//!
//! Contract:
//! 1. 200 with valid auth — returns MorningBriefingDto shape
//! 2. Empty graph — all sections empty, daysSinceLastJournal = 0
//! 3. Auth required — 401 without Bearer token
//! 4. Content shape: agendaItems, decayAlerts, serendipityHighlights, generatedAt, daysSinceLastJournal

use anyhow::Result;
use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode};
use quilt_infrastructure::database::sqlite::connection::create_pool;
use serde::Deserialize;
use serde_json::Value;
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

/// Build the app with a fresh in-memory DB.
async fn empty_app() -> Result<axum::Router> {
    init_auth();
    let pool = create_pool(":memory:").await?;
    let state = build_test_app_state(pool).await;
    Ok(quilt_server::routes::create_app(state))
}

/// Helper: extract JSON from a successful response body.
async fn json_from_response(res: axum::response::Response) -> Result<Value> {
    let body = axum::body::to_bytes(res.into_body(), 2048).await?;
    let json: Value = serde_json::from_slice(&body)?;
    Ok(json)
}

#[derive(Deserialize)]
struct MorningBriefingResponse {
    agenda_items: Vec<Value>,
    decay_alerts: Vec<Value>,
    serendipity_highlights: Vec<Value>,
    generated_at: String,
    days_since_last_journal: i64,
}

#[tokio::test]
async fn morning_briefing_requires_auth() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/cognitive/morning-briefing")
                .body(Body::empty())?,
        )
        .await?;

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn morning_briefing_returns_empty_on_cold_graph() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/morning-briefing")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 2048).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // All sections should be empty arrays
    assert!(
        json["agendaItems"].as_array().unwrap().is_empty(),
        "agenda should be empty on cold graph"
    );
    assert!(
        json["decayAlerts"].as_array().unwrap().is_empty(),
        "decay alerts should be empty on cold graph"
    );
    assert!(
        json["serendipityHighlights"].as_array().unwrap().is_empty(),
        "serendipity highlights should be empty on cold graph"
    );

    // generatedAt should be present and be a valid ISO string
    let generated_at = json["generatedAt"].as_str().unwrap();
    assert!(
        chrono::DateTime::parse_from_rfc3339(generated_at).is_ok(),
        "generatedAt should be valid RFC3339: {}",
        generated_at
    );

    // daysSinceLastJournal should be 0 on a cold graph
    assert_eq!(
        json["daysSinceLastJournal"].as_i64().unwrap(),
        0,
        "daysSinceLastJournal should be 0 on cold graph"
    );

    Ok(())
}

#[tokio::test]
async fn morning_briefing_response_has_correct_top_level_fields() -> Result<()> {
    let app = empty_app().await?;

    let res = app
        .oneshot(auth_header(
            Request::builder()
                .uri("/api/v1/cognitive/morning-briefing")
                .body(Body::empty())?,
        ))
        .await?;

    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), 2048).await?;
    let json: Value = serde_json::from_slice(&body)?;

    // Verify all expected fields are present
    assert!(json.get("agendaItems").is_some(), "should have agendaItems");
    assert!(json.get("decayAlerts").is_some(), "should have decayAlerts");
    assert!(
        json.get("serendipityHighlights").is_some(),
        "should have serendipityHighlights"
    );
    assert!(json.get("generatedAt").is_some(), "should have generatedAt");
    assert!(
        json.get("daysSinceLastJournal").is_some(),
        "should have daysSinceLastJournal"
    );

    // Verify field types
    assert!(
        json["agendaItems"].is_array(),
        "agendaItems should be array"
    );
    assert!(
        json["decayAlerts"].is_array(),
        "decayAlerts should be array"
    );
    assert!(
        json["serendipityHighlights"].is_array(),
        "serendipityHighlights should be array"
    );
    assert!(
        json["generatedAt"].is_string(),
        "generatedAt should be string"
    );
    assert!(
        json["daysSinceLastJournal"].is_number(),
        "daysSinceLastJournal should be number"
    );

    Ok(())
}
