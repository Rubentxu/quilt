//! Integration tests for the `annotations` table migration (008).
//!
//! Verifies the schema is created correctly:
//! - The `annotations` table exists with all expected columns
//! - The CHECK constraints are active
//! - Indices are created
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

/// Returns the SQL the SQLite engine would use to describe a column, or
/// `None` if the column doesn't exist. We probe the pragma table info so
/// we can assert schema shape without parsing the entire `sqlite_master`.
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
async fn annotations_table_exists() {
    let pool = setup_test_db().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='annotations'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 1, "annotations table should exist");
}

#[tokio::test]
async fn annotations_table_has_all_columns() {
    let pool = setup_test_db().await;
    let required_columns = [
        "id",
        "block_id",
        "scope",
        "author_type",
        "author_name",
        "content",
        "status",
        "highlight_start",
        "highlight_end",
        "parent_annotation_id",
        "created_at",
        "resolved_at",
        "resolved_by",
    ];
    for col in required_columns {
        assert!(
            column_exists(&pool, "annotations", col).await,
            "annotations table missing column: {col}"
        );
    }
}

#[tokio::test]
async fn annotations_indices_created() {
    let pool = setup_test_db().await;
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='annotations' \
         AND name LIKE 'idx_annotations_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(count >= 4, "expected >= 4 indices, got {count}");
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
async fn annotations_table_rejects_empty_content() {
    // CHECK (length(content) > 0) must be enforced by SQLite.
    let pool = setup_test_db().await;
    // First insert a parent block so the FK is satisfied.
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now: i64 = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                 VALUES (?, ?, 'markdown', 0, ?, ?)")
        .bind(&page_id)
        .bind("p")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO blocks (id, page_id, format, block_type, content, properties, \
                 collapsed, created_at, updated_at, refs, tags) \
                 VALUES (?, ?, 'markdown', 'paragraph', '', '{}', 0, ?, ?, '[]', '[]')")
        .bind(&block_id)
        .bind(&page_id)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

    // Empty content → CHECK violation
    let res = sqlx::query("INSERT INTO annotations \
        (id, block_id, scope, author_type, author_name, content, status, created_at) \
        VALUES (?, ?, 'block', 'human', 'u', '', 'pending', ?)")
        .bind(vec![2u8; 16])
        .bind(&block_id)
        .bind(now)
        .execute(&pool)
        .await;
    assert!(res.is_err(), "empty content must be rejected by CHECK");
}

#[tokio::test]
async fn annotations_table_rejects_invalid_scope() {
    let pool = setup_test_db().await;
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now: i64 = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                 VALUES (?, ?, 'markdown', 0, ?, ?)")
        .bind(&page_id)
        .bind("p")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO blocks (id, page_id, format, block_type, content, properties, \
                 collapsed, created_at, updated_at, refs, tags) \
                 VALUES (?, ?, 'markdown', 'paragraph', '', '{}', 0, ?, ?, '[]', '[]')")
        .bind(&block_id)
        .bind(&page_id)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

    let res = sqlx::query("INSERT INTO annotations \
        (id, block_id, scope, author_type, author_name, content, status, created_at) \
        VALUES (?, ?, 'bogus', 'human', 'u', 'x', 'pending', ?)")
        .bind(vec![2u8; 16])
        .bind(&block_id)
        .bind(now)
        .execute(&pool)
        .await;
    assert!(res.is_err(), "scope='bogus' must be rejected by CHECK");
}

#[tokio::test]
async fn annotations_table_cascade_deletes_on_block_delete() {
    let pool = setup_test_db().await;
    let page_id = vec![0u8; 16];
    let block_id = vec![1u8; 16];
    let now: i64 = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO pages (id, name, format, journal, created_at, updated_at) \
                 VALUES (?, ?, 'markdown', 0, ?, ?)")
        .bind(&page_id)
        .bind("p")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO blocks (id, page_id, format, block_type, content, properties, \
                 collapsed, created_at, updated_at, refs, tags) \
                 VALUES (?, ?, 'markdown', 'paragraph', '', '{}', 0, ?, ?, '[]', '[]')")
        .bind(&block_id)
        .bind(&page_id)
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();

    // Insert annotation, then delete block, then verify annotation is gone.
    sqlx::query("INSERT INTO annotations \
        (id, block_id, scope, author_type, author_name, content, status, created_at) \
        VALUES (?, ?, 'block', 'human', 'u', 'hi', 'pending', ?)")
        .bind(vec![2u8; 16])
        .bind(&block_id)
        .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM annotations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);

    sqlx::query("DELETE FROM blocks WHERE id = ?")
        .bind(&block_id)
        .execute(&pool)
        .await
        .unwrap();

    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM annotations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count_after, 0, "annotation must be cascade-deleted");
}
