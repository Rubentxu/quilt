//! Integration tests for navigate endpoints — requires a running
//! Quilt server at localhost:3737.
//!
//! Run via: just test-integration
//! (starts server container, runs tests, stops container)
//!
//! Each test is #[ignore] by default — un-ignore when server is
//! running, or use the justfile recipe.

/// Helper: build the API URL
fn api_url(path: &str) -> String {
    let base = std::env::var("QUILT_TEST_URL").unwrap_or_else(|_| "http://localhost:3737".into());
    format!("{}/api/v1{}", base, path)
}

// ── Health check (smoke test) ──────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_server_is_reachable() {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:3737/health"))
        .send()
        .await
        .expect("server not reachable — start with: just test-integration-start");

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ── Navigate to page ───────────────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_navigate_to_page_returns_page_dto() {
    let client = reqwest::Client::new();
    let resp = client
        .post(api_url("/navigate/page"))
        .json(&serde_json::json!({"page_name": "test-navigate"}))
        .send()
        .await
        .expect("POST /navigate/page failed");

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();

    // Should return page DTO with expected fields
    assert!(body.get("id").is_some());
    assert!(body.get("name").is_some());
}

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_navigate_to_page_returns_correct_name() {
    let client = reqwest::Client::new();
    let resp = client
        .post(api_url("/navigate/page"))
        .json(&serde_json::json!({"page_name": "my-navigate-test"}))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "my-navigate-test");
}

// ── Navigate to block ──────────────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_navigate_to_block_returns_ok() {
    let client = reqwest::Client::new();
    let resp = client
        .post(api_url("/navigate/block"))
        .json(&serde_json::json!({
            "page_name": "test",
            "block_uuid": "00000000-0000-0000-0000-000000000000"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

// ── Pages API ──────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_list_pages_endpoint() {
    let client = reqwest::Client::new();
    let resp = client.get(api_url("/pages")).send().await.unwrap();

    assert!(resp.status().is_success());
    let pages: Vec<serde_json::Value> = resp.json().await.unwrap();
    // Should be an array (possibly empty)
    assert!(pages.is_empty() || pages[0].get("name").is_some());
}
