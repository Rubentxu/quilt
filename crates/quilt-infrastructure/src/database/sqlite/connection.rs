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
            tags BLOB NOT NULL DEFAULT '[]',
            journal_day INTEGER,
            updated_journal_day INTEGER,
            deleted_at INTEGER
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Add journal_day columns if they don't exist (for existing databases)
    sqlx::query("ALTER TABLE blocks ADD COLUMN IF NOT EXISTS journal_day INTEGER")
        .execute(pool)
        .await
        .ok(); // Ignore error if column exists
    sqlx::query("ALTER TABLE blocks ADD COLUMN IF NOT EXISTS updated_journal_day INTEGER")
        .execute(pool)
        .await
        .ok(); // Ignore error if column exists

    // Create user_settings table (singleton)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_settings (
            id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
            timezone TEXT NOT NULL DEFAULT 'UTC',
            journal_format TEXT NOT NULL DEFAULT '%Y-%m-%d',
            start_of_week INTEGER NOT NULL DEFAULT 1 CHECK (start_of_week BETWEEN 0 AND 6),
            preferred_format TEXT NOT NULL DEFAULT 'markdown' CHECK (preferred_format IN ('markdown', 'org')),
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Initialize default settings if not exists
    sqlx::query(
        r#"INSERT OR IGNORE INTO user_settings (id, timezone, journal_format, start_of_week, preferred_format, updated_at)
           VALUES (1, 'UTC', '%Y-%m-%d', 1, 'markdown', unixepoch('now'))"#,
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
            updated_at INTEGER NOT NULL,
            deleted_at INTEGER
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
            mime_type TEXT,
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

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS refs (
            source_id BLOB NOT NULL,
            target_id BLOB NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (source_id, target_id)
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
        CREATE TABLE IF NOT EXISTS aliases (
            page_id BLOB NOT NULL,
            alias TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (page_id, alias)
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

    // ── Property Definition Tables ───────────────────────────────────────

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS property_definitions (
            id BLOB PRIMARY KEY NOT NULL,
            db_ident TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            property_type TEXT NOT NULL,
            cardinality TEXT NOT NULL DEFAULT 'one',
            view_context TEXT NOT NULL DEFAULT 'block',
            public INTEGER NOT NULL DEFAULT 1,
            queryable INTEGER NOT NULL DEFAULT 1,
            hidden INTEGER NOT NULL DEFAULT 0,
            attribute TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS closed_values (
            id BLOB PRIMARY KEY NOT NULL,
            property_id BLOB NOT NULL,
            db_ident TEXT NOT NULL,
            value TEXT NOT NULL,
            icon TEXT,
            "order" REAL NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS block_properties (
            block_id BLOB NOT NULL,
            property_id BLOB NOT NULL,
            value_type TEXT NOT NULL,
            value_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (block_id, property_id),
            FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE,
            FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // ── Class Definition Tables ──────────────────────────────────────────

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS class_definitions (
            id BLOB PRIMARY KEY NOT NULL,
            db_ident TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            icon TEXT,
            builtin INTEGER NOT NULL DEFAULT 0,
            user_defined INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS class_inheritance (
            class_id BLOB NOT NULL,
            parent_id BLOB NOT NULL,
            PRIMARY KEY (class_id, parent_id),
            FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_id) REFERENCES class_definitions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS class_required_properties (
            class_id BLOB NOT NULL,
            property_id BLOB NOT NULL,
            PRIMARY KEY (class_id, property_id),
            FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
            FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS class_default_properties (
            class_id BLOB NOT NULL,
            property_id BLOB NOT NULL,
            default_value_json TEXT NOT NULL,
            PRIMARY KEY (class_id, property_id),
            FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
            FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
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
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_priority ON blocks(priority)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_deleted_at ON blocks(deleted_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_name ON pages(name)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_journal_day ON pages(journal_day)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_namespace ON pages(namespace_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_deleted_at ON pages(deleted_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_refs_target_id ON refs(target_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag)")
        .execute(pool)
        .await?;

    // Property and class indices
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_definitions_db_ident ON property_definitions(db_ident)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_closed_values_property_id ON closed_values(property_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_block_properties_block_id ON block_properties(block_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_block_properties_property_id ON block_properties(property_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_class_definitions_db_ident ON class_definitions(db_ident)",
    )
    .execute(pool)
    .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_class_inheritance_class_id ON class_inheritance(class_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_class_inheritance_parent_id ON class_inheritance(parent_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_class_required_properties_class_id ON class_required_properties(class_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_class_default_properties_class_id ON class_default_properties(class_id)")
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
