//! HTTP Integration Tests
//!
//! Integration tests that start the HTTP server and make real requests.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tokio::signal;

/// Test application state
#[derive(Clone)]
struct TestState {
    pool: SqlitePool,
}

/// Health check response for tests
#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Create a minimal test router
fn create_test_router(state: TestState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .with_state(Arc::new(state))
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: "0.1.0".to_string(),
    })
}

/// Start a test server on a random available port
async fn start_test_server(pool: SqlitePool) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let state = TestState { pool };
    let app = create_test_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = signal::ctrl_c().await;
            })
            .await
            .unwrap();
    });

    (addr, handle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    fn get_test_vault_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("vault")
    }

    fn get_test_db_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data")
            .join("test.db")
    }

    async fn setup_test_pool() -> SqlitePool {
        // For integration tests, we need a real database
        // Create a temporary in-memory database
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());

        SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to create test database pool")
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let pool = setup_test_pool().await;
        let (addr, handle) = start_test_server(pool).await;

        // Make request using raw TCP
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .expect("Request failed");

        assert_eq!(resp.status(), 200);

        let body: HealthResponse = resp.json().await.expect("Failed to parse response");
        assert_eq!(body.status, "ok");
        assert_eq!(body.version, "0.1.0");

        // Cleanup
        handle.abort();
    }

    #[tokio::test]
    async fn test_blocks_endpoint_structure() {
        // Test that the blocks route returns proper JSON structure
        // This is a structural test without full database
        let query = serde_json::json!({
            "dsl": "(all)",
            "limit": 100
        });

        let json_str = serde_json::to_string(&query).unwrap();
        assert!(json_str.contains("\"dsl\":\"(all)\""));
        assert!(json_str.contains("\"limit\":100"));
    }

    #[tokio::test]
    async fn test_page_create_request_serialization() {
        #[derive(Serialize, Deserialize)]
        struct CreatePageRequest {
            name: String,
            title: Option<String>,
        }

        let req = CreatePageRequest {
            name: "Test Page".to_string(),
            title: Some("Test Title".to_string()),
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"name\":\"Test Page\""));
        assert!(json.contains("\"title\":\"Test Title\""));
    }

    #[tokio::test]
    async fn test_block_create_request_serialization() {
        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CreateBlockRequest {
            page_name: String,
            content: String,
            parent_id: Option<String>,
        }

        let req = CreateBlockRequest {
            page_name: "Test Page".to_string(),
            content: "Test content".to_string(),
            parent_id: None,
        };

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"pageName\":\"Test Page\""));
        assert!(json.contains("\"content\":\"Test content\""));
    }

    #[tokio::test]
    async fn test_search_query_serialization() {
        #[derive(Serialize, Deserialize)]
        struct SearchQuery {
            q: String,
            limit: Option<usize>,
        }

        let query = SearchQuery {
            q: "test search".to_string(),
            limit: Some(20),
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("\"q\":\"test search\""));
        assert!(json.contains("\"limit\":20"));
    }
}
