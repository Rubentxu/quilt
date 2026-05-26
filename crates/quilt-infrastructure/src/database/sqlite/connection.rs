//! SQLite connection pool management
//!
//! This module provides connection pooling and database migration functionality
//! for SQLite databases using the sqlx async driver.

use anyhow::Result;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::path::Path;

/// A type alias for the SQLite connection pool.
pub type DbPool = Pool<Sqlite>;

/// Creates an async SQLite connection pool for the given database path.
///
/// The pool is configured with `mode=rwc` which creates the database if it
/// doesn't exist and opens it for both reading and writing.
///
/// # Arguments
///
/// * `db_path` - Path to the SQLite database file
///
/// # Returns
///
/// Returns a [`DbPool`] connection pool on success.
///
/// # Example
///
/// ```
/// use quilt_infrastructure::database::sqlite::connection::create_pool;
///
/// async {
///     let pool = create_pool("/tmp/test.db").await.unwrap();
/// };
/// ```
pub async fn create_pool<P: AsRef<Path>>(db_path: P) -> Result<DbPool> {
    let database_url = format!("sqlite:{}?mode=rwc", db_path.as_ref().display());

    let pool = SqlitePoolOptions::new()
        .max_connections(1) // SQLite single writer
        .connect(&database_url)
        .await?;

    Ok(pool)
}

/// Creates a blocking SQLite connection pool for CLI use.
///
/// This is a convenience wrapper around [`create_pool`] that blocks on the
/// async pool creation. Use this when you need a pool outside of an async
/// context but still within a Tokio runtime.
///
/// # Arguments
///
/// * `db_path` - Path to the SQLite database file
///
/// # Errors
///
/// Returns an error if no Tokio runtime is currently active.
pub fn create_blocking_pool<P: AsRef<Path>>(db_path: P) -> Result<DbPool> {
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| anyhow::anyhow!("No Tokio runtime active"))?;
    handle.block_on(create_pool(db_path))
}

/// Runs database migrations (blocking version for CLI use).
///
/// This is a convenience wrapper around [`run_migrations`] that blocks.
/// Use this when you need to run migrations outside of an async context.
///
/// # Arguments
///
/// * `pool` - Reference to the database connection pool
///
/// # Errors
///
/// Returns an error if no Tokio runtime is currently active.
pub fn run_migrations_blocking(pool: &DbPool) -> Result<()> {
    let handle = tokio::runtime::Handle::try_current()
        .map_err(|_| anyhow::anyhow!("No Tokio runtime active"))?;
    handle.block_on(run_migrations(pool))
}

/// Runs all database migrations to set up the schema.
///
/// This function creates all necessary tables, indices, and triggers:
/// - `blocks`: Content blocks with properties, markers, and references
/// - `pages`: Named pages with namespace and journal support
/// - `files`: File metadata for attached files
/// - `tags`: Page tags with many-to-many relationship
/// - `refs`: Block-to-block references for backlinks
/// - `assets`: Block-to-file attachments
/// - `kv_store`: Key-value store for miscellaneous data
/// - `journals`: Journal day to page mapping
/// - `config`: Configuration key-value store
/// - `blocks_fts`: Full-text search virtual table
///
/// # Arguments
///
/// * `pool` - Reference to the database connection pool
///
/// # Example
///
/// ```
/// use quilt_infrastructure::database::sqlite::connection::{create_pool, run_migrations};
///
/// async {
///     let pool = create_pool("/tmp/test.db").await.unwrap();
///     run_migrations(&pool).await.unwrap();
/// };
/// ```
pub async fn run_migrations(pool: &DbPool) -> Result<()> {
    // Create tables
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blocks (
            id BLOB PRIMARY KEY NOT NULL,
            page_id BLOB NOT NULL,
            parent_id BLOB,
            order_index REAL NOT NULL DEFAULT 0,
            level INTEGER NOT NULL DEFAULT 1,
            format TEXT NOT NULL DEFAULT 'markdown',
            marker TEXT,
            priority TEXT,
            content TEXT NOT NULL DEFAULT '',
            properties BLOB NOT NULL DEFAULT '{}',
            scheduled INTEGER,
            deadline INTEGER,
            start_time INTEGER,
            repeated INTEGER,
            logbook INTEGER,
            collapsed INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            refs BLOB NOT NULL DEFAULT '[]',
            tags BLOB NOT NULL DEFAULT '[]'
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS pages (
            id BLOB PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            title TEXT,
            namespace_id BLOB,
            journal_day INTEGER,
            format TEXT NOT NULL DEFAULT 'markdown',
            file_id BLOB,
            original_name TEXT,
            journal INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            id BLOB PRIMARY KEY NOT NULL,
            path TEXT NOT NULL UNIQUE,
            content TEXT,
            hash BLOB NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            last_modified_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tags (
            page_id BLOB NOT NULL,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (page_id, tag)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Refs table — enhanced schema with ref_type column.
    // Drop the old table first (early dev — no production data) and recreate
    // with the full schema including ref_type as part of the primary key.
    sqlx::query("DROP TABLE IF EXISTS refs")
        .execute(pool)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS refs (
            source_id BLOB NOT NULL,
            target_id BLOB NOT NULL,
            ref_type TEXT NOT NULL CHECK(ref_type IN ('page_ref','block_ref','tag','alias')),
            created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
            PRIMARY KEY (source_id, target_id, ref_type)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS assets (
            block_id BLOB NOT NULL,
            file_id BLOB NOT NULL,
            asset_type TEXT NOT NULL,
            width INTEGER,
            height INTEGER,
            align TEXT DEFAULT 'center',
            external_url TEXT,
            PRIMARY KEY (block_id, file_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS kv_store (
            key TEXT PRIMARY KEY NOT NULL,
            value BLOB NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS journals (
            journal_day INTEGER PRIMARY KEY,
            page_id BLOB NOT NULL UNIQUE,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY NOT NULL,
            value BLOB NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indices
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_page_id ON blocks(page_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_parent_id ON blocks(parent_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_marker ON blocks(marker)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_updated_at ON blocks(updated_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_name ON pages(name)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_journal_day ON pages(journal_day)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_refs_source ON refs(source_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_refs_target ON refs(target_id, ref_type)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag)")
        .execute(pool)
        .await?;

    // Create FTS virtual table
    sqlx::query(
        r#"
        CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
            content,
            content=blocks,
            content_rowid=rowid
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create FTS triggers
    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS blocks_ai AFTER INSERT ON blocks BEGIN
            INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS blocks_ad AFTER DELETE ON blocks BEGIN
            INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
        END
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TRIGGER IF NOT EXISTS blocks_au AFTER UPDATE ON blocks BEGIN
            INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
        END
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
