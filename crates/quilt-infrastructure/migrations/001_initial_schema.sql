-- Quilt Database Schema
-- Based on Logseq DB with DDD architecture

-- Blocks table (core entity)
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
    deleted_at INTEGER
);

-- Pages table
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
);

-- Files table
CREATE TABLE IF NOT EXISTS files (
    id BLOB PRIMARY KEY NOT NULL,
    path TEXT NOT NULL UNIQUE,
    content TEXT,
    hash BLOB NOT NULL,
    size_bytes INTEGER NOT NULL,
    mime_type TEXT,
    created_at INTEGER NOT NULL,
    last_modified_at INTEGER NOT NULL
);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (
    page_id BLOB NOT NULL,
    tag TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (page_id, tag)
);

-- Aliases table
CREATE TABLE IF NOT EXISTS aliases (
    page_id BLOB NOT NULL,
    alias TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (page_id, alias)
);

-- References table (block-to-block links)
CREATE TABLE IF NOT EXISTS refs (
    source_id BLOB NOT NULL,
    target_id BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (source_id, target_id)
);

-- Assets table
CREATE TABLE IF NOT EXISTS assets (
    block_id BLOB NOT NULL,
    file_id BLOB NOT NULL,
    asset_type TEXT NOT NULL,
    width INTEGER,
    height INTEGER,
    align TEXT DEFAULT 'center',
    external_url TEXT,
    PRIMARY KEY (block_id, file_id)
);

-- Key-Value store
CREATE TABLE IF NOT EXISTS kv_store (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Journals cache
CREATE TABLE IF NOT EXISTS journals (
    journal_day INTEGER PRIMARY KEY,
    page_id BLOB NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);

-- Config table
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Indices for blocks
CREATE INDEX IF NOT EXISTS idx_blocks_page_id ON blocks(page_id);
CREATE INDEX IF NOT EXISTS idx_blocks_parent_id ON blocks(parent_id);
CREATE INDEX IF NOT EXISTS idx_blocks_marker ON blocks(marker);
CREATE INDEX IF NOT EXISTS idx_blocks_priority ON blocks(priority);
CREATE INDEX IF NOT EXISTS idx_blocks_updated_at ON blocks(updated_at);

-- Indices for pages
CREATE INDEX IF NOT EXISTS idx_pages_name ON pages(name);
CREATE INDEX IF NOT EXISTS idx_pages_journal_day ON pages(journal_day);
CREATE INDEX IF NOT EXISTS idx_pages_namespace ON pages(namespace_id);

-- Indices for refs
CREATE INDEX IF NOT EXISTS idx_refs_target_id ON refs(target_id);

-- Indices for soft-delete
CREATE INDEX IF NOT EXISTS idx_blocks_deleted_at ON blocks(deleted_at);
CREATE INDEX IF NOT EXISTS idx_pages_deleted_at ON pages(deleted_at);

-- Indices for tags
CREATE INDEX IF NOT EXISTS idx_tags_tag ON tags(tag);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
    content,
    content=blocks,
    content_rowid=rowid
);

-- Trigger to keep FTS in sync on insert
CREATE TRIGGER IF NOT EXISTS blocks_ai AFTER INSERT ON blocks BEGIN
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Trigger to keep FTS in sync on delete
CREATE TRIGGER IF NOT EXISTS blocks_ad AFTER DELETE ON blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;

-- Trigger to keep FTS in sync on update
CREATE TRIGGER IF NOT EXISTS blocks_au AFTER UPDATE ON blocks BEGIN
    INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
END;

-- Property definitions (schema for typed properties)
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
);

-- Closed values (predefined options for closed-set properties)
CREATE TABLE IF NOT EXISTS closed_values (
    id BLOB PRIMARY KEY NOT NULL,
    property_id BLOB NOT NULL,
    db_ident TEXT NOT NULL,
    value TEXT NOT NULL,
    icon TEXT,
    "order" REAL NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_closed_values_property ON closed_values(property_id);

-- Class definitions (entity classes with inheritance)
CREATE TABLE IF NOT EXISTS class_definitions (
    id BLOB PRIMARY KEY NOT NULL,
    db_ident TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    extends BLOB,
    icon TEXT,
    builtin INTEGER NOT NULL DEFAULT 0,
    user_defined INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Class inheritance (many-to-many, supports multiple inheritance)
CREATE TABLE IF NOT EXISTS class_inheritance (
    class_id BLOB NOT NULL,
    parent_id BLOB NOT NULL,
    PRIMARY KEY (class_id, parent_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES class_definitions(id) ON DELETE CASCADE
);

-- Class required properties
CREATE TABLE IF NOT EXISTS class_required_properties (
    class_id BLOB NOT NULL,
    property_id BLOB NOT NULL,
    PRIMARY KEY (class_id, property_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

-- Class default properties
CREATE TABLE IF NOT EXISTS class_default_properties (
    class_id BLOB NOT NULL,
    property_id BLOB NOT NULL,
    default_value_json TEXT NOT NULL,
    PRIMARY KEY (class_id, property_id),
    FOREIGN KEY (class_id) REFERENCES class_definitions(id) ON DELETE CASCADE,
    FOREIGN KEY (property_id) REFERENCES property_definitions(id) ON DELETE CASCADE
);

-- Block summaries (LLM-generated, used by TreeRAG engine)
CREATE TABLE IF NOT EXISTS block_summaries (
    block_id BLOB PRIMARY KEY,
    summary TEXT NOT NULL,
    content_hash BLOB NOT NULL,
    generated_at INTEGER NOT NULL,
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_block_summaries_generated ON block_summaries(generated_at);

-- Scheduled tasks (for the integrated TaskScheduler)
CREATE TABLE IF NOT EXISTS scheduled_tasks (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    cron_expr TEXT NOT NULL,
    task_type TEXT NOT NULL,
    task_config_json TEXT NOT NULL DEFAULT '{}',
    enabled INTEGER NOT NULL DEFAULT 1,
    last_run INTEGER,
    next_run INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_next_run ON scheduled_tasks(next_run);
