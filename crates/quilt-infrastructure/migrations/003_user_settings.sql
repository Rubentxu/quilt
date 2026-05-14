-- Migration 003: User Settings
-- Journal-First Architecture: Persist user preferences including timezone

-- User settings (singleton table - only one row with id=1)
CREATE TABLE IF NOT EXISTS user_settings (
    id INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),  -- Ensures singleton
    timezone TEXT NOT NULL DEFAULT 'UTC',
    journal_format TEXT NOT NULL DEFAULT '%Y-%m-%d',
    start_of_week INTEGER NOT NULL DEFAULT 1 CHECK (start_of_week BETWEEN 0 AND 6),
    preferred_format TEXT NOT NULL DEFAULT 'markdown' CHECK (preferred_format IN ('markdown', 'org')),
    updated_at INTEGER NOT NULL
);

-- Initialize with defaults if not exists
INSERT OR IGNORE INTO user_settings (id, timezone, journal_format, start_of_week, preferred_format, updated_at)
VALUES (1, 'UTC', '%Y-%m-%d', 1, 'markdown', unixepoch('now'));

-- Index for settings lookup (redundant but explicit)
CREATE INDEX IF NOT EXISTS idx_user_settings_id ON user_settings(id);