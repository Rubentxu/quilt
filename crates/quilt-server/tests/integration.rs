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
    use quilt_application::services::ref_service::RefService;
    use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let pool = create_pool(":memory:").await?;
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let ref_service = Arc::new(RwLock::new(RefService::new(ref_repo)));

    let state = quilt_server::state::AppState::new(pool, search_index, ref_service);

    assert!(state.navigation_tx.receiver_count() == 0);
    Ok(())
}

/// Test broadcast_navigation sends to subscribers
#[tokio::test]
async fn broadcast_navigation() -> Result<()> {
    use quilt_application::services::ref_service::RefService;
    use quilt_infrastructure::database::sqlite::repositories::SqliteRefRepository;
    use quilt_server::state::{AppState, NavigationEvent};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let pool = create_pool(":memory:").await?;
    let search_index = Arc::new(SearchIndexManager::new(pool.clone()));
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let ref_service = Arc::new(RwLock::new(RefService::new(ref_repo)));

    let state = AppState::new(pool, search_index, ref_service);

    let mut rx = state.navigation_tx.subscribe();
    let event = NavigationEvent::page(None, "Test".to_string());
    state.broadcast_navigation(event)?;

    let received = rx.recv().await?;
    assert_eq!(received.target.page_name, "Test");

    Ok(())
}
