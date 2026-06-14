-- 008_annotations.sql
--
-- First-class annotations table (the new "comments 2.0" system).
-- Replaces the previous ad-hoc `type:: comment` block property pattern
-- with a typed table. UUIDs are stored as BLOB (16 bytes), timestamps
-- as i64 epoch SECONDS (consistent with the `blocks` table and
-- `datetime_to_ts()` in the SQLite repos).
--
-- # Scope and inline highlights
--
-- `scope` is `block` (annotation targets the whole block) or `inline`
-- (annotation targets a byte range). For `inline`, BOTH
-- `highlight_start` and `highlight_end` MUST be non-NULL; the
-- application layer validates `start < end` and that the range fits
-- within the block's content. We do NOT enforce that with a CHECK
-- constraint because the DB cannot compute `end <= content.len()`
-- (the block's content is a JSON blob, not a TEXT column). All
-- validation lives in `Annotation::new()` in `quilt-domain`.
--
-- # Defaults
--
-- `scope` defaults to `'block'` so the common case (whole-block
-- annotation) doesn't require the caller to specify the column.
-- `status` does NOT default — the application layer always sets it
-- (the entity default is `pending`, but a future migration may
-- introduce a different default, so we make the SQL explicit).
--
-- # Foreign keys
--
-- `block_id` REFERENCES blocks(id) ON DELETE CASCADE — when the
-- target block is deleted, its annotations go with it.
-- `parent_annotation_id` REFERENCES annotations(id) ON DELETE
-- CASCADE — same logic for threaded replies.
--
-- # Rollback
--
--   DROP INDEX IF EXISTS idx_annotations_parent;
--   DROP INDEX IF EXISTS idx_annotations_status;
--   DROP INDEX IF EXISTS idx_annotations_block;
--   DROP TABLE IF EXISTS annotations;

CREATE TABLE IF NOT EXISTS annotations (
    id BLOB PRIMARY KEY NOT NULL,
    block_id BLOB NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
    scope TEXT NOT NULL DEFAULT 'block' CHECK(scope IN ('block','inline')),
    author_type TEXT NOT NULL CHECK(author_type IN ('human','agent')),
    author_name TEXT NOT NULL,
    content TEXT NOT NULL CHECK(length(content) > 0),
    status TEXT NOT NULL CHECK(status IN ('pending','in_progress','resolved','dismissed')),
    highlight_start INTEGER,
    highlight_end INTEGER,
    parent_annotation_id BLOB REFERENCES annotations(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    resolved_at INTEGER,
    resolved_by TEXT
);

-- Indices that back the common lookup paths:
-- - `get_by_block(block_id)`     → ordered by created_at
-- - `get_by_status(status)`      → ordered by created_at DESC
-- - `get_thread_replies(parent)` → ordered by created_at ASC
-- - `get_by_filters` with mixed predicates can use any of the above
CREATE INDEX IF NOT EXISTS idx_annotations_block ON annotations(block_id, created_at);
CREATE INDEX IF NOT EXISTS idx_annotations_status ON annotations(status, created_at);
CREATE INDEX IF NOT EXISTS idx_annotations_parent ON annotations(parent_annotation_id, created_at);
CREATE INDEX IF NOT EXISTS idx_annotations_author ON annotations(author_name);
