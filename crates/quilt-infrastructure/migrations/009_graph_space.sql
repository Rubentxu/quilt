-- Migration 009: Graph Space Metadata
-- Singleton table for graph-level metadata including display name

CREATE TABLE IF NOT EXISTS graph_space (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- Ensures singleton
    name TEXT NOT NULL DEFAULT 'My Graph',
    description TEXT NOT NULL DEFAULT '',
    version TEXT NOT NULL DEFAULT '1.0',
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Initialize with defaults if not exists
INSERT OR IGNORE INTO graph_space (id, name, description, version, created_at, updated_at)
VALUES (1, 'My Graph', '', '1.0', unixepoch('now'), unixepoch('now'));

CREATE INDEX IF NOT EXISTS idx_graph_space_id ON graph_space(id);
