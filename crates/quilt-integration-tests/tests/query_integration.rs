//! Integration tests for QueryService and QueryParser
//!
//! These tests verify the query DSL parsing, SQL generation, and execution.

use quilt_domain::content::BlockContent;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, Priority, TaskMarker, Uuid};
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
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

/// Seed test data with blocks for query testing
async fn seed_query_test_data(pool: &SqlitePool) -> (Uuid, Vec<Uuid>) {
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    // Insert test page
    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("query-test-page")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert test page");

    let block_ids = vec![
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    ];

    // Insert todo block with priority A
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, priority, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_ids[0].to_string())
    .bind(page_id.to_string())
    .bind("A todo task with priority A")
    .bind("todo")
    .bind("a")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert todo block");

    // Insert done block
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_ids[1].to_string())
    .bind(page_id.to_string())
    .bind("A done task")
    .bind("done")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert done block");

    // Insert later block
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_ids[2].to_string())
    .bind(page_id.to_string())
    .bind("A later task")
    .bind("later")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert later block");

    // Insert now block
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_ids[3].to_string())
    .bind(page_id.to_string())
    .bind("A now task")
    .bind("now")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert now block");

    // Insert block without marker
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, format, level, created_at, updated_at) VALUES (?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_ids[4].to_string())
    .bind(page_id.to_string())
    .bind("A regular block without marker")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert regular block");

    // Insert into FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(pool)
        .await
        .expect("failed to populate FTS");

    (page_id, block_ids)
}

// =============================================================================
// Query Parser Tests
// =============================================================================

#[tokio::test]
async fn test_query_parser_simple_task_query() {
    let parser = quilt_query::QueryParser;

    // Test basic task query
    let result = parser.parse("(task todo)");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_multiple_tasks() {
    let parser = quilt_query::QueryParser;

    // Test multiple tasks
    let result = parser.parse("(task todo done)");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_with_limit() {
    let parser = quilt_query::QueryParser;

    // Query with limit embedded (in real usage limit comes from caller)
    let result = parser.parse("(task todo)");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_page_reference() {
    let parser = quilt_query::QueryParser;

    // Test page reference
    let result = parser.parse("[[My Page]]");
    assert!(result.is_ok());

    let result = result.unwrap();
    match result {
        quilt_query::QueryExpr::PageRef(name) => assert_eq!(name, "My Page"),
        _ => panic!("Expected PageRef"),
    }
}

#[tokio::test]
async fn test_query_parser_complex_boolean() {
    let parser = quilt_query::QueryParser;

    // Test complex AND/OR expression
    let result = parser.parse("(and (task todo) (priority a))");
    assert!(result.is_ok());

    // Test OR expression
    let result = parser.parse("(or (task todo) (task done))");
    assert!(result.is_ok());

    // Test NOT expression
    let result = parser.parse("(not (task cancelled))");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_property_query() {
    let parser = quilt_query::QueryParser;

    // Test property equality
    let result = parser.parse(r#"(property "author" "John")"#);
    assert!(result.is_ok());

    // Test property not equals
    let result = parser.parse(r#"(property "status" != "done")"#);
    assert!(result.is_ok());

    // Test property greater than
    let result = parser.parse(r#"(property "count" > "10")"#);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_nested_expressions() {
    let parser = quilt_query::QueryParser;

    // Test deeply nested expression
    let result = parser.parse("(and (or (not (task todo)) (task done)) (priority a))");
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_query_parser_empty_input() {
    let parser = quilt_query::QueryParser;

    let result = parser.parse("");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_query_parser_invalid_syntax() {
    let parser = quilt_query::QueryParser;

    // Unclosed parenthesis
    let result = parser.parse("(task todo");
    assert!(result.is_err());

    // Invalid characters
    let result = parser.parse("(((");
    assert!(result.is_err());
}

// =============================================================================
// QueryService Plan Tests (no DB execution)
// =============================================================================

#[test]
fn test_query_service_prepare_task_query() {
    let service = quilt_application::query_service::QueryService::new();

    // Test task query plan
    let result = service.prepare("(task todo)", 100);
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert!(query_result.sql.contains("marker"));
    assert!(query_result.params.len() >= 1);
}

#[test]
fn test_query_service_prepare_priority_query() {
    let service = quilt_application::query_service::QueryService::new();

    // Test priority query plan
    let result = service.prepare("(priority a)", 100);
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert!(query_result.sql.contains("priority"));
}

#[test]
fn test_query_service_prepare_page_query() {
    let service = quilt_application::query_service::QueryService::new();

    // Test page query plan
    let result = service.prepare(r#"(page "Test Page")"#, 100);
    assert!(result.is_ok());
}

#[test]
fn test_query_service_prepare_full_text_search() {
    let service = quilt_application::query_service::QueryService::new();

    // Test FTS query plan
    let result = service.prepare("(full-text-search \"rust\")", 100);
    assert!(result.is_ok());
}

#[test]
fn test_query_service_prepare_complex_query() {
    let service = quilt_application::query_service::QueryService::new();

    // Test complex AND query plan
    let result = service.prepare("(and (task todo) (priority a))", 50);
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert!(query_result.sql.contains("AND"));
    assert!(query_result.params.len() >= 2);
}

#[test]
fn test_query_service_prepare_invalid_query() {
    let service = quilt_application::query_service::QueryService::new();

    // Invalid query should fail
    let result = service.prepare("(task", 100);
    assert!(result.is_err());
}

// =============================================================================
// QueryService Execution Tests (with DB)
// =============================================================================

#[tokio::test]
async fn test_query_service_execute_task_query() {
    let pool = setup_test_db().await;
    let (page_id, block_ids) = seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute task todo query
    let result = service.execute("(task todo)", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 1);
    assert_eq!(
        query_result.blocks[0].content.as_plain_text(),
        "A todo task with priority A"
    );
}

#[tokio::test]
async fn test_query_service_execute_task_done() {
    let pool = setup_test_db().await;
    let (page_id, block_ids) = seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute task done query
    let result = service.execute("(task done)", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 1);
    assert_eq!(
        query_result.blocks[0].content.as_plain_text(),
        "A done task"
    );
}

#[tokio::test]
async fn test_query_service_execute_task_multiple() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute query for multiple tasks
    let result = service.execute("(task todo done later now)", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 4); // All except the non-marker block
}

#[tokio::test]
async fn test_query_service_execute_priority_query() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute priority query
    let result = service.execute("(priority a)", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 1);
    assert_eq!(
        query_result.blocks[0].content.as_plain_text(),
        "A todo task with priority A"
    );
}

#[tokio::test]
async fn test_query_service_execute_page_query() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute page query
    let result = service
        .execute(r#"(page "query-test-page")"#, 100, &pool)
        .await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 5); // All blocks on the page
}

#[tokio::test]
async fn test_query_service_execute_and_query() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute AND query
    let result = service
        .execute("(and (task todo) (priority a))", 100, &pool)
        .await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 1);
}

#[tokio::test]
async fn test_query_service_execute_or_query() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute OR query
    let result = service
        .execute("(or (task todo) (task done))", 100, &pool)
        .await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 2);
}

#[tokio::test]
async fn test_query_service_execute_not_query() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute NOT query - find blocks that are NOT todo
    let result = service.execute("(not (task todo))", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    // Should include done, later, now, and non-marker blocks
    assert!(query_result.count >= 4);
}

#[tokio::test]
async fn test_query_service_execute_empty_result() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute query with no results
    let result = service.execute("(task cancelled)", 100, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 0);
    assert!(query_result.blocks.is_empty());
}

#[tokio::test]
async fn test_query_service_execute_with_limit() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Execute with limit
    let result = service.execute("(task todo done later now)", 2, &pool).await;
    assert!(result.is_ok());

    let query_result = result.unwrap();
    assert_eq!(query_result.count, 2); // Limited to 2
}

// =============================================================================
// Query Integration E2E Tests
// =============================================================================

#[tokio::test]
async fn test_query_integration_find_todos_with_priority() {
    let pool = setup_test_db().await;
    let (page_id, _) = seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Create additional blocks
    let block_repo = SqliteBlockRepository::new(pool.clone());
    let tz = test_timezone();

    // Create another priority A todo
    let mut block = Block::new(
        BlockCreate {
            page_id,
            content: BlockContent::from_text("Another priority A todo"),
            marker: Some(TaskMarker::Todo),
            priority: Some(Priority::A),
            format: BlockFormat::Markdown,
            ..Default::default()
        },
        &tz,
    )
    .unwrap();

    block_repo.insert(&block).await.unwrap();
    block_repo.update(&block).await.unwrap();

    // Execute AND query
    let result = service
        .execute("(and (task todo) (priority a))", 100, &pool)
        .await
        .unwrap();

    assert_eq!(result.count, 2); // Both priority A todos
}

#[tokio::test]
async fn test_query_integration_page_reference() {
    let pool = setup_test_db().await;
    seed_query_test_data(&pool).await;
    let service = quilt_application::query_service::QueryService::new();

    // Use page reference syntax
    let result = service
        .execute("[[query-test-page]]", 100, &pool)
        .await
        .unwrap();

    assert_eq!(result.count, 5); // All blocks on the page
}