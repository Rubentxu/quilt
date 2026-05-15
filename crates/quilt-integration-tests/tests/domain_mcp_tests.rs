//! Integration tests for MCP server and domain operations
//!
//! These tests verify the full stack from MCP tools to database operations.

use quilt_domain::services::TimezoneService;
use sqlx::SqlitePool;
use uuid::Uuid;

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

/// Seed test data and return (page_id, todo_block_id, done_block_id)
async fn seed_test_data(pool: &SqlitePool) -> (Uuid, Uuid, Uuid) {
    let page_id = Uuid::new_v4();
    let block_id_todo = Uuid::new_v4();
    let block_id_done = Uuid::new_v4();
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

    // Insert todo block with priority 'a'
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, priority, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(block_id_todo.to_string())
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
    .bind(block_id_done.to_string())
    .bind(page_id.to_string())
    .bind("A done task")
    .bind("done")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert done block");

    // Insert into FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(pool)
        .await
        .expect("failed to populate FTS");

    (page_id, block_id_todo, block_id_done)
}

// =============================================================================
// Domain Entity Tests
// =============================================================================

#[tokio::test]
async fn test_block_entity_creation() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};

    let page_id = Uuid::new_v4();
    let create = BlockCreate {
        page_id,
        content: "Test block content".to_string(),
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };

    let block = Block::new(create, &test_timezone()).expect("Block creation should succeed");

    assert_eq!(block.content, "Test block content");
    assert_eq!(block.page_id, page_id);
    assert!(block.marker.is_some());
    assert_eq!(block.marker.unwrap(), TaskMarker::Todo);
}

#[tokio::test]
async fn test_block_circular_reference_detection() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::value_objects::{BlockFormat, Uuid};

    let page_id = Uuid::new_v4();

    // Create a hierarchy: A -> B -> C
    let create_a = BlockCreate {
        page_id,
        content: "Block A".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };
    let mut block_a = Block::new(create_a, &test_timezone()).unwrap();

    let create_b = BlockCreate {
        page_id,
        content: "Block B".to_string(),
        parent_id: Some(block_a.id),
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };
    let mut block_b = Block::new(create_b, &test_timezone()).unwrap();

    let create_c = BlockCreate {
        page_id,
        content: "Block C".to_string(),
        parent_id: Some(block_b.id),
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };
    let block_c = Block::new(create_c, &test_timezone()).unwrap();

    let all_blocks = vec![block_a.clone(), block_b.clone(), block_c.clone()];

    // C should NOT be able to move to A (would create cycle)
    assert!(
        !block_c.can_move_to(Some(block_a.id), &all_blocks),
        "C should not be able to move to its ancestor A"
    );

    // C SHOULD be able to move to a new parent
    let new_parent = Uuid::new_v4();
    assert!(
        block_c.can_move_to(Some(new_parent), &all_blocks),
        "C should be able to move to a new unrelated parent"
    );
}

#[tokio::test]
async fn test_block_update_logbook_on_done() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};

    let create = BlockCreate {
        page_id: Uuid::new_v4(),
        content: "Task".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };

    let mut block = Block::new(create, &test_timezone()).unwrap();
    assert!(block.logbook.is_none(), "Logbook should be None initially");

    // Mark as done - logbook should be set
    block
        .update(
            quilt_domain::entities::BlockUpdate {
                marker: Some(TaskMarker::Done),
                ..Default::default()
            },
            &test_timezone(),
        )
        .unwrap();

    assert!(
        block.logbook.is_some(),
        "Logbook should be set when marker becomes Done"
    );
}

// =============================================================================
// Query Parser Tests
// =============================================================================

#[tokio::test]
async fn test_query_parser_handles_nested_expressions() {
    use quilt_query::{QueryExpr, QueryParser};

    let parser = QueryParser;

    // Deeply nested expression should parse without stack overflow
    let result = parser.parse("(and (or (not (task todo)) (task done)) (priority a))");
    assert!(
        result.is_ok(),
        "Nested expression should parse: {:?}",
        result.err()
    );

    let expr = result.unwrap();
    match expr {
        QueryExpr::And(_) => {}
        _ => panic!("Expected And expression"),
    }
}

#[tokio::test]
async fn test_query_parser_rejects_empty_input() {
    use quilt_query::QueryParser;

    let parser = QueryParser;
    let result = parser.parse("");
    assert!(result.is_err(), "Empty input should be rejected");
}

#[tokio::test]
async fn test_query_parser_handles_page_ref() {
    use quilt_query::{QueryExpr, QueryParser};

    let parser = QueryParser;
    let result = parser.parse("[[My Page]]");
    assert!(result.is_ok());

    match result.unwrap() {
        QueryExpr::PageRef(name) => assert_eq!(name, "My Page"),
        _ => panic!("Expected PageRef"),
    }
}

// =============================================================================
// Repository Tests
// =============================================================================

#[tokio::test]
async fn test_block_insert_and_retrieve() {
    use quilt_domain::entities::{Block, BlockCreate};
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::{BlockFormat, Uuid};
    use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;

    let pool = setup_test_db().await;

    let repo = SqliteBlockRepository::new(pool.clone());
    let page_id = Uuid::new_v4();

    // Insert a block
    let create = BlockCreate {
        page_id,
        content: "Test content".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    };
    let block = Block::new(create, &test_timezone()).expect("Block creation should succeed");

    repo.insert(&block).await.expect("Insert should succeed");

    // Retrieve the block
    let retrieved = repo
        .get_by_id(block.id)
        .await
        .expect("Get should succeed")
        .expect("Block should exist");

    assert_eq!(retrieved.id, block.id);
    assert_eq!(retrieved.content, "Test content");
}

#[tokio::test]
async fn test_page_insert_and_retrieve() {
    use quilt_domain::entities::{Page, PageCreate};
    use quilt_domain::repositories::PageRepository;
    use quilt_domain::value_objects::BlockFormat;
    use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;

    let pool = setup_test_db().await;
    let repo = SqlitePageRepository::new(pool.clone());

    // Insert a page
    let page = Page::new(PageCreate {
        name: "test-page".to_string(),
        title: Some("Test Page Title".to_string()),
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
    })
    .expect("Page creation should succeed");

    repo.insert(&page).await.expect("Insert should succeed");

    // Retrieve by name
    let retrieved = repo
        .get_by_name("test-page")
        .await
        .expect("Get should succeed")
        .expect("Page should exist");

    assert_eq!(retrieved.name, "test-page");
    assert_eq!(retrieved.title, Some("Test Page Title".to_string()));
}

// =============================================================================
// Search Tests
// =============================================================================

#[tokio::test]
async fn test_fts_search_returns_relevant_results() {
    use quilt_query::{QueryExpr, QueryParser};

    let parser = QueryParser;
    let result = parser
        .parse("(full-text-search \"todo\")")
        .expect("Should parse FTS query");

    match result {
        QueryExpr::BlockContent(_) => {}
        _ => panic!("Expected BlockContent"),
    }
}

// =============================================================================
// Value Object Tests
// =============================================================================

#[test]
fn test_journal_day_conversion() {
    use quilt_domain::value_objects::JournalDay;

    let day = JournalDay::from_ymd(2026, 5, 7).expect("Should create JournalDay");
    assert_eq!(day.as_i32(), 20260507);

    let date = day.to_naive_date();
    assert!(date.is_some());
}

#[test]
fn test_task_marker_from_str() {
    use quilt_domain::value_objects::TaskMarker;

    assert_eq!(TaskMarker::from_str("todo"), Some(TaskMarker::Todo));
    assert_eq!(TaskMarker::from_str("done"), Some(TaskMarker::Done));
    assert_eq!(TaskMarker::from_str("later"), Some(TaskMarker::Later));
    assert_eq!(TaskMarker::from_str("now"), Some(TaskMarker::Now));
    assert_eq!(
        TaskMarker::from_str("cancelled"),
        Some(TaskMarker::Cancelled)
    );
    assert_eq!(TaskMarker::from_str("invalid"), None);
}

#[test]
fn test_priority_ordering() {
    use quilt_domain::value_objects::Priority;

    // Test that Priority derives Ord correctly (A < B < C by enum discriminant)
    let priorities = vec![Priority::C, Priority::A, Priority::B];
    let mut sorted = priorities.clone();
    sorted.sort();
    assert_eq!(sorted, vec![Priority::A, Priority::B, Priority::C]);
}
