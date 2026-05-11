-- Deep links table - supports internal and external links with metadata
CREATE TABLE IF NOT EXISTS deep_links (
    id BLOB PRIMARY KEY NOT NULL,
    source_id BLOB NOT NULL,
    source_type TEXT NOT NULL,
    target_id BLOB,
    target_page_name TEXT,
    link_type TEXT NOT NULL,
    external_url TEXT,
    link_text TEXT,
    context TEXT,
    created_at INTEGER NOT NULL
);

-- Index for querying links by source
CREATE INDEX IF NOT EXISTS idx_deep_links_source ON deep_links(source_id, source_type);

-- Index for querying links by target
CREATE INDEX IF NOT EXISTS idx_deep_links_target ON deep_links(target_id);

-- Index for querying links by type
CREATE INDEX IF NOT EXISTS idx_deep_links_type ON deep_links(link_type);

-- Index for text search on link_text
CREATE INDEX IF NOT EXISTS idx_deep_links_link_text ON deep_links(link_text);
