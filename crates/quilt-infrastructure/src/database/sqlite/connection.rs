//! SQLite connection pool management
//!
//! This module provides connection pooling and database migration functionality
//! for SQLite databases using the sqlx async driver.

use anyhow::Result;
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};
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
            block_type TEXT NOT NULL DEFAULT 'paragraph',
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

    // Migration 007: add `block_type` column to `blocks` (P0 of
    // `quilt-blocktype-persistence`). The column is `NOT NULL DEFAULT
    // 'paragraph'` so existing rows backfill cleanly with the same
    // default value the entity used to assume implicitly. The
    // migration is idempotent: if the column already exists, this
    // errors with "duplicate column" but we catch and ignore that case.
    // Mirrors the pattern of migration 006 (F5: pages.properties).
    match sqlx::query("ALTER TABLE blocks ADD COLUMN block_type TEXT NOT NULL DEFAULT 'paragraph'")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            // Ignore "duplicate column" error so the migration is idempotent.
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(e.into());
            }
        }
    }

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

    // Migration 006: add `properties` column to `pages` (F5).
    // Additive: no data backfill, no row rewriting. Pre-existing pages get
    // `'{}'` (empty JSON object) which parses to an empty HashMap on read.
    // The migration is idempotent: if the column already exists, this errors
    // with "duplicate column" but we catch and ignore that case.
    match sqlx::query("ALTER TABLE pages ADD COLUMN properties TEXT NOT NULL DEFAULT '{}'")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            // Ignore "duplicate column" error so the migration is idempotent.
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(e.into());
            }
        }
    }

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
            custom_context TEXT,
            PRIMARY KEY (source_id, target_id, ref_type)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Migration 007: add `custom_context` column to `refs` (Q028
    // Editable Backlinks). Additive: no data backfill, no row
    // rewriting. Pre-existing refs get `NULL` which the Rust
    // deserializer treats as "no override". Idempotent: if the
    // column already exists, this errors with "duplicate column"
    // but we catch and ignore that case.
    match sqlx::query("ALTER TABLE refs ADD COLUMN custom_context TEXT")
        .execute(pool)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(e.into());
            }
        }
    }

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

    // Initialize with defaults if empty
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO user_settings (id, timezone, journal_format, start_of_week, preferred_format, updated_at)
        VALUES (1, 'UTC', '%Y-%m-%d', 1, 'markdown', unixepoch('now'))
        "#,
    )
    .execute(pool)
    .await?;

    // Create tour_dismissals table (B of quilt-fase4-cross-device-tour).
    // Keyed by the opaque `user_id` (V1: the api key from the
    // Authorization header) and a short tour-name slug. The composite
    // primary key is idempotent — dismissing the same tour twice
    // updates the timestamp rather than creating a duplicate row.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tour_dismissals (
            user_id TEXT NOT NULL,
            tour_name TEXT NOT NULL,
            dismissed_at INTEGER NOT NULL DEFAULT (unixepoch('now')),
            PRIMARY KEY (user_id, tour_name)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Index by user_id for the common "list my dismissed tours" query.
    // The PK already covers the lookup (it starts with user_id), so
    // the index is implicit; we still add it explicitly for the
    // post-dismissal `WHERE user_id = ?` reads in case the optimizer
    // ever changes its mind.
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tour_dismissals_user ON tour_dismissals(user_id)")
        .execute(pool)
        .await?;

    // ── Property Intelligence v3 (PI-3) — typed property definitions ──
    //
    // Persists the schema side of the typed property system. BUILTIN
    // properties (status, priority, deadline, ...) are seeded by the
    // application layer from `BUILTIN_PROPERTIES` because their
    // closed_values live in Rust; the table here is for USER-DEFINED
    // definitions and the closed values that the user has registered
    // for them. Builtin definitions still resolve via the static
    // `BUILTIN_PROPERTIES` map in `quilt-domain::properties::builtin`,
    // so this table is the "custom" namespace — builtin definitions
    // are NOT auto-seeded here (they're read from the in-memory map).
    //
    // UUIDs are stored as BLOB (16 bytes) — consistent with all other
    // tables. Booleans are stored as INTEGER (0/1). DateTime<Utc> as
    // i64 milliseconds since epoch (matching the rest of the schema).
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS property_definitions (
            id BLOB PRIMARY KEY NOT NULL,
            db_ident TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            property_type TEXT NOT NULL DEFAULT 'Text',
            cardinality TEXT NOT NULL DEFAULT 'One',
            view_context TEXT NOT NULL DEFAULT 'Block',
            public INTEGER NOT NULL DEFAULT 1,
            queryable INTEGER NOT NULL DEFAULT 1,
            hidden INTEGER NOT NULL DEFAULT 0,
            attribute TEXT,
            read_only INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            alias_of TEXT,
            block_count INTEGER NOT NULL DEFAULT 0,
            page_count INTEGER NOT NULL DEFAULT 0,
            first_seen_at INTEGER,
            last_seen_at INTEGER
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS property_closed_values (
            id BLOB PRIMARY KEY NOT NULL,
            property_id BLOB NOT NULL REFERENCES property_definitions(id) ON DELETE CASCADE,
            db_ident TEXT NOT NULL,
            value TEXT NOT NULL,
            icon TEXT,
            sort_order REAL NOT NULL DEFAULT 0.0
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Indices that back the three new PropertyRepository methods
    // (get_by_db_ident, get_by_db_idents, list_by_usage).
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_defs_db_ident ON property_definitions(db_ident)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_defs_status ON property_definitions(status)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_defs_usage ON property_definitions(block_count DESC)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_closed_values_property ON property_closed_values(property_id)")
        .execute(pool)
        .await?;

    // ── Property Intelligence v7 (PI-7) — property schema templates ──
    //
    // Stores reusable property clusters (schemas). Each schema has a name,
    // description, and a JSON array of property keys. Auto-detected schemas
    // are flagged for user review. UUIDs as BLOB, timestamps as i64 ms.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS property_schemas (
            id BLOB PRIMARY KEY NOT NULL,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            property_keys TEXT NOT NULL DEFAULT '[]',
            auto_detected INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_schemas_name ON property_schemas(name)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_property_schemas_auto ON property_schemas(auto_detected)")
        .execute(pool)
        .await?;

    Ok(())
}
