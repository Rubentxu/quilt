-- Enhanced refs table with ref_type support
-- Replaces the old refs table (source_id, target_id) with the new schema
-- that includes ref_type as part of the primary key.
--
-- Changes from 001_initial_schema:
--   - Added ref_type column (page_ref, block_ref, tag, alias)
--   - Changed PRIMARY KEY to (source_id, target_id, ref_type)
--   - Added DEFAULT unixepoch() * 1000 for created_at

CREATE TABLE IF NOT EXISTS refs (
    source_id BLOB NOT NULL,
    target_id BLOB NOT NULL,
    ref_type TEXT NOT NULL CHECK(ref_type IN ('page_ref','block_ref','tag','alias')),
    created_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    PRIMARY KEY (source_id, target_id, ref_type)
);

CREATE INDEX IF NOT EXISTS idx_refs_source ON refs(source_id);
CREATE INDEX IF NOT EXISTS idx_refs_target ON refs(target_id, ref_type);
