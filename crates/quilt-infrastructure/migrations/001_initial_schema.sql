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
    tags BLOB NOT NULL DEFAULT '[]'
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
    updated_at INTEGER NOT NULL
);

-- Files table
CREATE TABLE IF NOT EXISTS files (
    id BLOB PRIMARY KEY NOT NULL,
    path TEXT NOT NULL UNIQUE,
    content TEXT,
    hash BLOB NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    last_modified_at INTEGER NOT NULL
);

-- Tags table
CREATE TABLE IF NOT EXISTS tags (
    page_id BLOB PRIMARY KEY NOT NULL,
    tag TEXT NOT NULL,
    created_at INTEGER NOT NULL
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
