-- 0001_init.sql — initial global app state schema (ADR-0030 §5)
--
-- The global state lives in `~/.local/share/quilt/global.db` (or
-- `XDG_DATA_HOME/quilt/global.db`) and stores:
--   - last_opened_graph       : path to the most recently opened graph
--   - recent_graphs_json      : JSON array of recent graph paths (capped at 10)
--   - right_sidebar_visible   : persisted visibility of the right sidebar
--                               (NULL = "no preference yet")
--
-- The `id = 1` CHECK enforces a single-row table (singleton pattern).
-- The `INSERT OR IGNORE` makes the bootstrap idempotent.

CREATE TABLE IF NOT EXISTS global_app_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_opened_graph TEXT NULL,
    recent_graphs_json TEXT NOT NULL DEFAULT '[]',
    right_sidebar_visible INTEGER NULL
);

INSERT OR IGNORE INTO global_app_state (id) VALUES (1);
