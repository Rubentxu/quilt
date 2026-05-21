//! Integration tests for BlockCommand and BlockQuery handlers
//!
//! These tests verify the full stack from command/query handlers through to the database.

use quilt_domain::content::BlockContent;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::repositories::BlockRepository;
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Returns a timezone for tests (UTC)
fn test_timezone() -> TimezoneService {
    TimezoneService::from_tz_string("UTC").expect("UTC is a valid timezone")
}

/// Sets up an in-memory SQLite database with the full schema.
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("failed to connect to memory DB");

    // Create blocks table
    sqlx::query(
        r#"
        CREATE TABLE blocks (
            id TEXT PRIMARY KEY NOT NULL,
            page_id TEXT NOT NULL,
            parent_id TEXT,
            order_index REAL NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            format TEXT NOT NULL DEFAULT 'markdown',
            marker TEXT,
            priority TEXT,
            content TEXT NOT NULL DEFAULT '',
            properties TEXT NOT NULL DEFAULT '{}',
            scheduled INTEGER,
            deadline INTEGER,
            start_time INTEGER,
            repeated INTEGER,
            logbook INTEGER,
            collapsed INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            refs TEXT NOT NULL DEFAULT '[]',
            tags TEXT NOT NULL DEFAULT '[]',
            deleted_at INTEGER,
            journal_day INTEGER,
            updated_journal_day INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create blocks table");

    // Create pages table
    sqlx::query(
        r#"
        CREATE TABLE pages (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            title TEXT,
            namespace_id TEXT,
            journal_day INTEGER,
            format TEXT NOT NULL DEFAULT 'markdown',
            file_id TEXT,
            original_name TEXT,
            journal INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create pages table");

    // Create tags table
    sqlx::query(
        r#"
        CREATE TABLE tags (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create tags table");

    // Create block_tags table
    sqlx::query(
        r#"
        CREATE TABLE block_tags (
            block_id TEXT NOT NULL,
            tag_id TEXT NOT NULL,
            PRIMARY KEY (block_id, tag_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create block_tags table");

    // Create refs table
    sqlx::query(
        r#"
        CREATE TABLE refs (
            source_id TEXT NOT NULL,
            target_id TEXT NOT NULL,
            PRIMARY KEY (source_id, target_id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create refs table");

    // Create FTS virtual table
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE blocks_fts USING fts5(
            content,
            content=blocks,
            content_rowid=rowid
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create FTS table");

    pool
}

/// Seed test data and return the page_id
async fn seed_test_data(pool: &SqlitePool) -> Uuid {
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    // Insert test page
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("test-page")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert test page");

    page_id
}

// =============================================================================
// BlockCommand Handler Tests
// =============================================================================

#[tokio::test]
async fn test_block_command_create_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Test create
    let result = command
        .create(page_id, "Test block content".to_string(), None)
        .await;

    assert!(result.is_ok());
    let block = result.unwrap();
    assert_eq!(block.page_id, page_id);
    assert_eq!(block.content.as_plain_text(), "Test block content");
}

#[tokio::test]
async fn test_block_command_create_with_parent() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create parent block first
    let parent = command
        .create(page_id, "Parent block".to_string(), None)
        .await
        .unwrap();

    // Create child block
    let child = command
        .create(page_id, "Child block".to_string(), Some(parent.id))
        .await
        .unwrap();

    assert_eq!(child.parent_id, Some(parent.id));
    assert_eq!(child.level, parent.level + 1);
}

#[tokio::test]
async fn test_block_command_update_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Original content".to_string(), None)
        .await
        .unwrap();

    // Update the block
    let update = quilt_domain::entities::BlockUpdate {
        content: Some(BlockContent::from_text("Updated content")),
        ..Default::default()
    };

    let result = command.update(block.id, update).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().content.as_plain_text(), "Updated content");
}

#[tokio::test]
async fn test_block_command_set_marker() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Task block".to_string(), None)
        .await
        .unwrap();

    // Set marker to Todo
    let result = command.set_marker(block.id, TaskMarker::Todo).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().marker, Some(TaskMarker::Todo));

    // Change marker to Done
    let result = command.set_marker(block.id, TaskMarker::Done).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().marker, Some(TaskMarker::Done));
}

#[tokio::test]
async fn test_block_command_delete_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Block to delete".to_string(), None)
        .await
        .unwrap();

    // Delete the block
    let result = command.delete(block.id).await;
    assert!(result.is_ok());

    // Verify it's gone
    let repo = SqliteBlockRepository::new(pool.clone());
    let retrieved = repo.get_by_id(block.id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_block_command_hard_delete_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Block to hard delete".to_string(), None)
        .await
        .unwrap();

    // Hard delete the block
    let result = command.hard_delete(block.id).await;
    assert!(result.is_ok());

    // Verify it's completely gone
    let repo = SqliteBlockRepository::new(pool.clone());
    let retrieved = repo.get_by_id(block.id).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
async fn test_block_command_restore_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Block to restore".to_string(), None)
        .await
        .unwrap();

    // Delete the block
    command.delete(block.id).await.unwrap();

    // Restore the block
    let result = command.restore(block.id).await;
    assert!(result.is_ok());

    // Verify it's back
    let repo = SqliteBlockRepository::new(pool.clone());
    let retrieved = repo.get_by_id(block.id).await.unwrap();
    assert!(retrieved.is_some());
}

// =============================================================================
// BlockQuery Handler Tests
// =============================================================================

#[tokio::test]
async fn test_block_query_get_block() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let query = quilt_application::queries::BlockQuery::new(Arc::new(repo), Arc::new(page_repo));

    // Create a block
    let block = quilt_application::commands::BlockCommand::new(
        Arc::new(SqliteBlockRepository::new(pool.clone())),
        Arc::new(test_timezone()),
    )
    .create(page_id, "Query test block".to_string(), None)
    .await
    .unwrap();

    // Query the block
    let result = query.get(block.id).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn test_block_query_get_with_children() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let query = quilt_application::queries::BlockQuery::new(Arc::new(repo.clone()), Arc::new(page_repo));

    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // Create parent and child blocks
    let parent = command
        .create(page_id, "Parent".to_string(), None)
        .await
        .unwrap();

    command
        .create(page_id, "Child".to_string(), Some(parent.id))
        .await
        .unwrap();

    // Query with children
    let result = query.get_with_children(parent.id).await;
    assert!(result.is_ok());
    let blocks = result.unwrap();
    assert_eq!(blocks.len(), 2); // Parent and child
}

#[tokio::test]
async fn test_block_query_get_by_page() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let query = quilt_application::queries::BlockQuery::new(Arc::new(repo), Arc::new(page_repo));

    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(SqliteBlockRepository::new(pool.clone())),
        Arc::new(test_timezone()),
    );

    // Create multiple blocks
    command
        .create(page_id, "Block 1".to_string(), None)
        .await
        .unwrap();
    command
        .create(page_id, "Block 2".to_string(), None)
        .await
        .unwrap();

    // Query by page
    let result = query.get_by_page(page_id).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

#[tokio::test]
async fn test_block_query_search() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let query = quilt_application::queries::BlockQuery::new(Arc::new(repo), Arc::new(page_repo));

    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(SqliteBlockRepository::new(pool.clone())),
        Arc::new(test_timezone()),
    );

    // Create blocks with searchable content
    command
        .create(page_id, "Rust programming language".to_string(), None)
        .await
        .unwrap();
    command
        .create(page_id, "Python scripting".to_string(), None)
        .await
        .unwrap();

    // Search for Rust
    let result = query.search("Rust", 10).await;
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[tokio::test]
async fn test_block_query_check_exists() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(
        pool.clone(),
    );
    let query = quilt_application::queries::BlockQuery::new(Arc::new(repo), Arc::new(page_repo));

    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(SqliteBlockRepository::new(pool.clone())),
        Arc::new(test_timezone()),
    );

    // Create a block
    let block = command
        .create(page_id, "Existing block".to_string(), None)
        .await
        .unwrap();

    // Check exists
    let result = query.check_exists(block.id).await;
    assert!(result.is_ok());
    assert!(result.unwrap());

    // Check non-existent
    let result = query.check_exists(Uuid::new_v4()).await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

// =============================================================================
// Block Handler E2E Flow Tests
// =============================================================================

#[tokio::test]
async fn test_block_handler_full_flow() {
    let pool = setup_test_db().await;
    let page_id = seed_test_data(&pool).await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let command = quilt_application::commands::BlockCommand::new(
        Arc::new(repo),
        Arc::new(test_timezone()),
    );

    // 1. Create initial block
    let block1 = command
        .create(page_id, "First task".to_string(), None)
        .await
        .unwrap();

    // 2. Create child blocks
    let block2 = command
        .create(page_id, "Second task".to_string(), Some(block1.id))
        .await
        .unwrap();

    // 3. Mark second task as done
    let updated = command
        .set_marker(block2.id, TaskMarker::Done)
        .await
        .unwrap();

    assert_eq!(updated.marker, Some(TaskMarker::Done));

    // 4. Verify block1 still exists and has children
    let repo = SqliteBlockRepository::new(pool.clone());
    let retrieved = repo.get_by_id(block1.id).await.unwrap().unwrap();

    let children = repo.get_children(block1.id).await.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].id, block2.id);
}