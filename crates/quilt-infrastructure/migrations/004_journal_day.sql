-- Migration 004: Journal Day for Blocks
-- Journal-First Architecture: Track when blocks were created/updated in journal terms

BEGIN TRANSACTION;

-- 1. Add journal day columns to blocks (no-ops if columns already exist)
-- journal_day: Journal day when block was created (YYYYMMDD format)
-- updated_journal_day: Journal day when block was last updated (YYYYMMDD format)
ALTER TABLE blocks ADD COLUMN journal_day INTEGER;
ALTER TABLE blocks ADD COLUMN updated_journal_day INTEGER;

-- 2. Create indices for efficient day-based queries
CREATE INDEX IF NOT EXISTS idx_blocks_journal_day ON blocks(journal_day) WHERE journal_day IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_blocks_updated_journal_day ON blocks(updated_journal_day) WHERE updated_journal_day IS NOT NULL;

-- 3. Backfill journal_day from page.journal_day for existing blocks
-- This handles the case where blocks were created on journal pages before this feature
UPDATE blocks SET journal_day = (
    SELECT p.journal_day
    FROM pages p
    WHERE p.id = blocks.page_id
    AND p.journal_day IS NOT NULL
)
WHERE journal_day IS NULL AND deleted_at IS NULL;

-- 4. Set updated_journal_day = journal_day for existing blocks with journal_day
-- This ensures blocks that were migrated have consistent updated_journal_day
UPDATE blocks SET updated_journal_day = journal_day
WHERE updated_journal_day IS NULL AND journal_day IS NOT NULL;

-- 5. Partial index for orphan blocks (created before migration or on non-journal pages)
CREATE INDEX IF NOT EXISTS idx_blocks_orphan ON blocks(id) WHERE journal_day IS NULL AND deleted_at IS NULL;

COMMIT;