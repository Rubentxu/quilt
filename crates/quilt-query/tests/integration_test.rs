//! Integration tests for query parser and executor
//!
//! These tests use real in-memory SQLite to verify the full query pipeline.

use quilt_application::SearchUseCases;
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

    // Parse and build SQL using search use cases
    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(task todo)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 todo block");
}

#[tokio::test]
async fn test_e2e_priority_query() {
    let pool = setup_test_db().await;
    let (_page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(priority a)", 100)
        .await
        .expect("query failed");

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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // Query: todo AND priority a - should find the todo block with priority a
    let result = use_cases
        .query("(and (task todo) (priority a))", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find exactly 1 block matching both conditions"
    );

    // Query: todo AND done - should find 0 blocks (nothing is both)
    let result2 = use_cases
        .query("(and (task todo) (task done))", 100)
        .await
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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(page \"test-page\")", 100)
        .await
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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("[[test-page]]", 100)
        .await
        .expect("query failed");

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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    let result = use_cases
        .query("(full-text-search \"todo\")", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 1,
        "Should find 1 block matching FTS query for 'todo'"
    );

    // Query for something not in the content
    let result2 = use_cases
        .query("(full-text-search \"nonexistent\")", 100)
        .await
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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // Query for a marker that doesn't exist
    let result = use_cases
        .query("(task cancelled)", 100)
        .await
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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // Use a wide range that should include all blocks
    let result = use_cases
        .query("(between 0 9999999999999)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(
        result_count, 2,
        "Should find all 2 blocks with wide between range"
    );

    // Use a narrow range that should exclude all blocks
    let result2 = use_cases
        .query("(between 9999999999998 9999999999999)", 100)
        .await
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

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // Query: todo OR done - should find 2 blocks
    let result = use_cases
        .query("(or (task todo) (task done))", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 2, "Should find 2 blocks with todo OR done");
}

#[tokio::test]
async fn test_e2e_not_query() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // Query: NOT done - should find 1 block (the todo block)
    let result = use_cases
        .query("(not (task done))", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find 1 block that is NOT done");
}

#[tokio::test]
async fn test_e2e_self_ref() {
    let pool = setup_test_db().await;
    seed_test_data(&pool).await;

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();

    // SelfRef should match all blocks (1 = 1 is always true)
    let result = use_cases.query("self", 100).await.expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 2, "SelfRef should match all blocks");
}

// ─────────────────────────────────────────────────────────────────────────────
// T5: DSL Predicates for Journal Aggregation
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_e2e_scheduled_today() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert a block scheduled for today (at noon local time)
    let scheduled_block_id = Uuid::new_v4();
    let local_now = chrono::Local::now();
    let local_date = local_now.date_naive();
    let noon = local_date.and_hms_opt(12, 0, 0).unwrap();
    let offset_secs = local_now.offset().local_minus_utc();
    let utc_dt = noon - chrono::Duration::seconds(offset_secs as i64);
    let today_ms = utc_dt.and_utc().timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, scheduled, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(scheduled_block_id.to_string())
    .bind(page_id.to_string())
    .bind("A scheduled task")
    .bind("todo")
    .bind(today_ms)
    .bind(today_ms)
    .bind(today_ms)
    .execute(&pool)
    .await
    .expect("failed to insert scheduled block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(scheduled today)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 block scheduled for today");
}

#[tokio::test]
async fn test_e2e_deadline_today() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert a block with deadline at noon local time today (safe from date boundary issues)
    let deadline_block_id = Uuid::new_v4();
    let local_now = chrono::Local::now();
    let local_date = local_now.date_naive();
    // Use noon to avoid any date boundary issues
    let noon = local_date.and_hms_opt(12, 0, 0).unwrap();
    // Convert local naive datetime to UTC by subtracting the local offset
    let offset_secs = local_now.offset().local_minus_utc();
    let utc_dt = noon - chrono::Duration::seconds(offset_secs as i64);
    let today_ms = utc_dt.and_utc().timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, deadline, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(deadline_block_id.to_string())
    .bind(page_id.to_string())
    .bind("A deadline task")
    .bind("todo")
    .bind(today_ms)
    .bind(today_ms)
    .bind(today_ms)
    .execute(&pool)
    .await
    .expect("failed to insert deadline block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(deadline today)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 block with deadline today");
}

#[tokio::test]
async fn test_e2e_overdue() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert an overdue block (deadline yesterday, not done/cancelled)
    let overdue_block_id = Uuid::new_v4();
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
    let yesterday_ms = yesterday.timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, deadline, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(overdue_block_id.to_string())
    .bind(page_id.to_string())
    .bind("An overdue task")
    .bind("todo")
    .bind(yesterday_ms)
    .bind(yesterday_ms)
    .bind(yesterday_ms)
    .execute(&pool)
    .await
    .expect("failed to insert overdue block");

    // Insert a done block that is also past deadline (should NOT appear in overdue)
    let done_overdue_block_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, deadline, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(done_overdue_block_id.to_string())
    .bind(page_id.to_string())
    .bind("Done but overdue")
    .bind("done")
    .bind(yesterday_ms)
    .bind(yesterday_ms)
    .bind(yesterday_ms)
    .execute(&pool)
    .await
    .expect("failed to insert done-but-overdue block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(overdue)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 overdue block (not done)");
}

#[tokio::test]
async fn test_e2e_in_progress() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert a doing block
    let doing_block_id = Uuid::new_v4();
    let now = chrono::Utc::now().timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?)",
    )
    .bind(doing_block_id.to_string())
    .bind(page_id.to_string())
    .bind("A doing task")
    .bind("doing")
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .expect("failed to insert doing block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(in-progress)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 in-progress block");
}

#[tokio::test]
async fn test_e2e_scheduled_tomorrow() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert a block scheduled for tomorrow
    let scheduled_block_id = Uuid::new_v4();
    let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
    let tomorrow_start = tomorrow.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let tomorrow_ms = tomorrow_start.and_utc().timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, scheduled, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(scheduled_block_id.to_string())
    .bind(page_id.to_string())
    .bind("Tomorrow's task")
    .bind("todo")
    .bind(tomorrow_ms)
    .bind(tomorrow_ms)
    .bind(tomorrow_ms)
    .execute(&pool)
    .await
    .expect("failed to insert tomorrow's block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(scheduled tomorrow)", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 block scheduled for tomorrow");
}

#[tokio::test]
async fn test_e2e_predicates_compose_with_and() {
    let pool = setup_test_db().await;
    let (page_id, _todo_block_id, _done_block_id) = seed_test_data(&pool).await;

    // Insert a block scheduled for today AND doing
    let combined_block_id = Uuid::new_v4();
    let today = chrono::Utc::now();
    let today_start = today.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_ms = today_start.and_utc().timestamp_millis();

    sqlx::query(
        "INSERT INTO blocks (id, page_id, content, marker, format, level, scheduled, created_at, updated_at) VALUES (?, ?, ?, ?, 'markdown', 1, ?, ?, ?)",
    )
    .bind(combined_block_id.to_string())
    .bind(page_id.to_string())
    .bind("Combined task")
    .bind("doing")
    .bind(today_ms)
    .bind(today_ms)
    .bind(today_ms)
    .execute(&pool)
    .await
    .expect("failed to insert combined block");

    // Update FTS
    sqlx::query("INSERT INTO blocks_fts(rowid, content) SELECT rowid, content FROM blocks")
        .execute(&pool)
        .await
        .expect("failed to populate FTS");

    let use_cases = quilt_application::use_cases::SearchUseCasesImpl::new();
    let result = use_cases
        .query("(and (scheduled today) (in-progress))", 100)
        .await
        .expect("query failed");

    let result_count = execute_query(&pool, &result.sql, &result.params).await;
    assert_eq!(result_count, 1, "Should find exactly 1 block matching both predicates");
}
