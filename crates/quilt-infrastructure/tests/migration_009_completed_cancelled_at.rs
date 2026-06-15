//! Integration tests for the `completed_at` and `cancelled_at` columns migration (009).
//!
//! Verifies the schema changes:
//! - The `completed_at` and `cancelled_at` columns exist on the `blocks` table
//! - Backfill populates `completed_at` from `logbook` for existing Done blocks
//! - Backfill populates `cancelled_at` from `logbook` for existing Cancelled blocks
//! - The migration is idempotent (running it twice doesn't fail)

use sqlx::Row;

use quilt_infrastructure::database::sqlite::connection;

/// Helper: connect to an in-memory SQLite, run migrations, return the pool.
async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory DB");
    connection::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

/// Returns true if the column exists in the given table.
async fn column_exists(pool: &sqlx::SqlitePool, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .expect("PRAGMA failed");
    rows.iter().any(|r| {
        let name: String = r.get("name");
        name == column
    })
}

#[tokio::test]
async fn blocks_table_has_completed_at_column() {
    let pool = setup_test_db().await;
    assert!(
        column_exists(&pool, "blocks", "completed_at").await,
        "blocks table should have completed_at column"
    );
}

#[tokio::test]
async fn blocks_table_has_cancelled_at_column() {
    let pool = setup_test_db().await;
    assert!(
        column_exists(&pool, "blocks", "cancelled_at").await,
        "blocks table should have cancelled_at column"
    );
}

#[tokio::test]
async fn migration_is_idempotent() {
    // Running migrations twice on the same pool should not error.
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    connection::run_migrations(&pool).await.expect("first run");
    connection::run_migrations(&pool)
        .await
        .expect("second run must be a no-op (idempotent)");
}

#[tokio::test]
async fn backfill_completed_at_from_logbook_for_done_blocks() {
    let pool = setup_test_db().await;

    // Insert a page first
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now_ts = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
         VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(&page_id)
    .bind("test page")
    .bind(now_ts)
    .bind(now_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Insert a block with marker='done' and logbook set
    let logbook_ts = 1718000000i64; // Some fixed timestamp
    sqlx::query(
        "INSERT INTO blocks (id, page_id, format, block_type, marker, content, properties, \
         collapsed, created_at, updated_at, refs, tags, logbook) \
         VALUES (?, ?, 'markdown', 'todo', 'done', 'Done task', '{}', 0, ?, ?, '[]', '[]', ?)",
    )
    .bind(&block_id)
    .bind(&page_id)
    .bind(now_ts)
    .bind(now_ts)
    .bind(logbook_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Re-run migrations to trigger backfill
    connection::run_migrations(&pool).await.expect("backfill run");

    // Verify completed_at was backfilled from logbook
    let completed_at: Option<i64> = sqlx::query_scalar(
        "SELECT completed_at FROM blocks WHERE id = ?",
    )
    .bind(&block_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        completed_at.is_some(),
        "completed_at should be populated for done blocks"
    );
    assert_eq!(
        completed_at.unwrap(),
        logbook_ts,
        "completed_at should equal logbook value"
    );
}

#[tokio::test]
async fn backfill_cancelled_at_from_logbook_for_cancelled_blocks() {
    let pool = setup_test_db().await;

    // Insert a page first
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now_ts = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
         VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(&page_id)
    .bind("test page")
    .bind(now_ts)
    .bind(now_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Insert a block with marker='cancelled' and logbook set
    let logbook_ts = 1718000000i64; // Some fixed timestamp
    sqlx::query(
        "INSERT INTO blocks (id, page_id, format, block_type, marker, content, properties, \
         collapsed, created_at, updated_at, refs, tags, logbook) \
         VALUES (?, ?, 'markdown', 'todo', 'cancelled', 'Cancelled task', '{}', 0, ?, ?, '[]', '[]', ?)",
    )
    .bind(&block_id)
    .bind(&page_id)
    .bind(now_ts)
    .bind(now_ts)
    .bind(logbook_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Re-run migrations to trigger backfill
    connection::run_migrations(&pool).await.expect("backfill run");

    // Verify cancelled_at was backfilled from logbook
    let cancelled_at: Option<i64> = sqlx::query_scalar(
        "SELECT cancelled_at FROM blocks WHERE id = ?",
    )
    .bind(&block_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        cancelled_at.is_some(),
        "cancelled_at should be populated for cancelled blocks"
    );
    assert_eq!(
        cancelled_at.unwrap(),
        logbook_ts,
        "cancelled_at should equal logbook value"
    );
}

#[tokio::test]
async fn new_done_block_columns_start_null() {
    // Verifies that newly inserted blocks have NULL completed_at and cancelled_at.
    // The application-level Block.update() method sets these on marker transitions.
    let pool = setup_test_db().await;

    // Insert a page first
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now_ts = chrono::Utc::now().timestamp();

    sqlx::query(
        "INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
         VALUES (?, ?, 'markdown', 0, ?, ?)",
    )
    .bind(&page_id)
    .bind("test page")
    .bind(now_ts)
    .bind(now_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Insert a block with marker='todo'
    sqlx::query(
        "INSERT INTO blocks (id, page_id, format, block_type, marker, content, properties, \
         collapsed, created_at, updated_at, refs, tags) \
         VALUES (?, ?, 'markdown', 'todo', 'todo', 'A task', '{}', 0, ?, ?, '[]', '[]')",
    )
    .bind(&block_id)
    .bind(&page_id)
    .bind(now_ts)
    .bind(now_ts)
    .execute(&pool)
    .await
    .unwrap();

    // Both timestamp columns should be NULL initially
    let completed_at: Option<i64> = sqlx::query_scalar(
        "SELECT completed_at FROM blocks WHERE id = ?",
    )
    .bind(&block_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let cancelled_at: Option<i64> = sqlx::query_scalar(
        "SELECT cancelled_at FROM blocks WHERE id = ?",
    )
    .bind(&block_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        completed_at.is_none(),
        "completed_at should be NULL for a fresh block"
    );
    assert!(
        cancelled_at.is_none(),
        "cancelled_at should be NULL for a fresh block"
    );
}
