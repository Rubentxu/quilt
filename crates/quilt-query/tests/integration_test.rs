//! Integration tests for query parser and executor
//!
//! These tests use real in-memory SQLite to verify the full query pipeline.

use sqlx::SqlitePool;
use uuid::Uuid;

/// Sets up an in-memory SQLite database with the schema needed for query tests.
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
            "order" REAL NOT NULL DEFAULT 0,
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
            tags TEXT NOT NULL DEFAULT '[]'
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
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create pages table");

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

/// Seeds test data and returns (page_id, todo_block_id, done_block_id).
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
    .bind("A todo task")
    .bind("todo")
    .bind("a")
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert todo block");

    // Insert done block (no priority)
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

/// Executes a query and returns the number of matching rows.
async fn execute_query(pool: &SqlitePool, sql: &str, params: &[String]) -> u32 {
    let mut query = sqlx::query(sql);
    for param in params {
        query = query.bind(param);
    }
    let rows = query.fetch_all(pool).await.expect("query failed");
    rows.len() as u32
}

#[tokio::test]
async fn test_e2e_task_query() {
    let pool = setup_test_db().await;
    let (_page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Parse and build SQL using query service
    let service = quilt_application::query_service::QueryService::new();
    let result = service.prepare("(task todo)", 100).expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 todo block");
}

#[tokio::test]
async fn test_e2e_priority_query() {
    let pool = setup_test_db().await;
    let (_page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();
    let result = service.prepare("(priority a)", 100).expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find exactly 1 block with priority 'a'"
    );
}

#[tokio::test]
async fn test_e2e_and_query() {
    let pool = setup_test_db().await;
    let (_page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // Query: todo AND priority a - should find the todo block with priority a
    let result = service
        .prepare("(and (task todo) (priority a))", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find exactly 1 block matching both conditions"
    );

    // Query: todo AND done - should find 0 blocks (nothing is both)
    let result2 = service
        .prepare("(and (task todo) (task done))", 100)
        .expect("query failed");

    let result_count2 = execute_query(&pool, &result2.sql, &result2.params).await;
    assert_eq!(
        result_count2, 0,
        "Should find 0 blocks matching both todo and done"
    );
}

#[tokio::test]
async fn test_e2e_page_query() {
    let pool = setup_test_db().await;
    let (_page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();
    let result = service
        .prepare("(page \"test-page\")", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 2, "Should find 2 blocks on test-page");
}

#[tokio::test]
async fn test_e2e_page_ref_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    // Insert a block that contains a page reference
    let page_ref_block_id = Uuid::new_v4();
    let page_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(page_id.to_string())
    .bind("referenced-page")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert referenced page");

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(page_ref_block_id.to_string())
    .bind(page_id.to_string())
    .bind("Check out [[test-page]] for details")
    .bind("todo")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert page ref block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let service = quilt_application::query_service::QueryService::new();
    let result = service.prepare("[[test-page]]", 100).expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find 1 block containing [[test-page]]"
    );
}

#[tokio::test]
async fn test_e2e_fts_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    let result = service
        .prepare("(full-text-search \"todo\")", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find 1 block matching FTS query for 'todo'"
    );

    // Query for something not in the content
    let result2 = service
        .prepare("(full-text-search \"nonexistent\")", 100)
        .expect("query failed");

    let result_count2 = execute_query(&pool, &result2.sql, &result2.params).await;
    assert_eq!(
        result_count2, 0,
        "Should find 0 blocks matching 'nonexistent'"
    );
}

#[tokio::test]
async fn test_e2e_no_results() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // Query for a marker that doesn't exist
    let result = service
        .prepare("(task cancelled)", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 0,
        "Should find 0 blocks with 'cancelled' marker"
    );
}

#[tokio::test]
async fn test_e2e_between_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // Use a wide range that should include all blocks
    // Note: pest has a limitation with unquoted integers, using quoted integers
    let result = service
        .prepare("(between \"0\" \"9999999999999\")", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 2,
        "Should find all 2 blocks with wide between range"
    );

    // Use a narrow range that should exclude all blocks
    let result2 = service
        .prepare("(between \"9999999999998\" \"9999999999999\")", 100)
        .expect("query failed");

    let result_count2 = execute_query(&pool, &result2.sql, &result2.params).await;
    assert_eq!(
        result_count2, 0,
        "Should find 0 blocks with narrow between range"
    );
}

#[tokio::test]
async fn test_e2e_or_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // Query: todo OR done - should find 2 blocks
    let result = service
        .prepare("(or (task todo) (task done))", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 2, "Should find 2 blocks with todo OR done");
}

#[tokio::test]
async fn test_e2e_not_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // Query: NOT done - should find 1 block (the todo block)
    let result = service
        .prepare("(not (task done))", 100)
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find 1 block that is NOT done");
}

#[tokio::test]
async fn test_e2e_self_ref() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let service = quilt_application::query_service::QueryService::new();

    // SelfRef should match all blocks (1 = 1 is always true)
    let result = service.prepare("self", 100).expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 2, "SelfRef should match all blocks");
}
