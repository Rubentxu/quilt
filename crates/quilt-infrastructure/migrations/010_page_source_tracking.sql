-- Migration 010: Page Source Tracking for Manual Ingestion & Reindex (GS-9)
--
-- ADR-0030 §4: Track the source file path and modification time for pages
-- that were ingested from the graph directory. This enables the reindex
-- feature to detect which files have changed since ingestion.
--
-- Additive migration: no data backfill, no row rewriting.
-- Pre-existing pages get NULL for both columns (no source = manually created).
-- The migration is idempotent: if the columns already exist, the ALTER TABLE
-- errors with "duplicate column" but we catch and ignore that case.

-- Add source_path column: relative POSIX path to the source .md file
-- (never absolute, relative to graph root). NULL means the page was not
-- ingested from a file.
ALTER TABLE pages ADD COLUMN source_path TEXT;

-- Add source_mtime column: modification time of the source file at ingestion
-- (and at subsequent reindex). Stored as i64 (Unix timestamp ms) for SQLite
-- integer affinity. NULL if source_path is NULL.
ALTER TABLE pages ADD COLUMN source_mtime INTEGER;

-- Partial index for fast reindex lookups: only index rows with a non-null
-- source_path. This speeds up the scan_directory candidate classification
-- (get_by_source_path) without bloating the index for manually-created pages.
CREATE INDEX IF NOT EXISTS idx_pages_source_path ON pages(source_path)
    WHERE source_path IS NOT NULL;
