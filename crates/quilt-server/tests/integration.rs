//! Integration tests for the Quilt HTTP server
//!
//! These tests require a running database and full server setup.

use anyhow::Result;
use quilt_infrastructure::database::sqlite::connection::create_pool;
use quilt_search::SearchIndexManager;
use std::sync::Arc;

/// Test NavigationEvent creation
#[test]
fn navigation_event_page() {
    use quilt_server::state::NavigationEvent;

    let event = NavigationEvent::page(Some("graph-1".to_string()), "Test Page".to_string());

    assert_eq!(event.event_type, "navigate-to");
    assert_eq!(event.target.target_type, "Page");
    assert_eq!(event.target.graph_id, Some("graph-1".to_string()));
    assert_eq!(event.target.page_name, "Test Page");
    assert_eq!(event.target.block_uuid, None);
}

/// Test NavigationEvent creation for block
#[test]
fn navigation_event_block() {
    use quilt_server::state::NavigationEvent;

    let event = NavigationEvent::block(
        Some("graph-1".to_string()),
        "Test Page".to_string(),
        "block-uuid-123".to_string(),
    );

    assert_eq!(event.event_type, "navigate-to");
    assert_eq!(event.target.target_type, "Block");
    assert_eq!(event.target.graph_id, Some("graph-1".to_string()));
    assert_eq!(event.target.page_name, "Test Page");
    assert_eq!(event.target.block_uuid, Some("block-uuid-123".to_string()));
}

/// Test that AppState can be created
#[tokio::test]
async fn app_state_creation() -> Result<()> {
    // Create an in-memory database for testing
    let pool = create_pool(":memory:").await?;

    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let ai_client: Arc<dyn quilt_cognitive::AIClient> =
        Arc::new(quilt_cognitive::ai_client::MockAIClient::new());

    let state = quilt_server::state::AppState::new(pool, search_index, ai_client);

    // Verify state fields are accessible
    assert!(state.navigation_tx.receiver_count() == 0);

    Ok(())
}

/// Test broadcast_navigation sends to subscribers
#[tokio::test]
async fn broadcast_navigation() -> Result<()> {
    use quilt_server::state::{AppState, NavigationEvent};
    use std::sync::Arc;

    let pool = create_pool(":memory:").await?;
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let ai_client: Arc<dyn quilt_cognitive::AIClient> =
        Arc::new(quilt_cognitive::ai_client::MockAIClient::new());

    let state = AppState::new(pool, search_index, ai_client);

    // Create a subscriber
    let mut rx = state.navigation_tx.subscribe();

    // Broadcast an event
    let event = NavigationEvent::page(None, "Test".to_string());
    state.broadcast_navigation(event)?;

    // Receive the event
    let received = rx.recv().await?;
    assert_eq!(received.target.page_name, "Test");

    Ok(())
}
