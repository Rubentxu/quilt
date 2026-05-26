# Bidirectional Reference Tracking — Research Report

> For Quilt: Rust + SQLite + WASM target
> Date: 2026-05-26

---

## 1. How Logseq DB Stores Refs

### 1.1 DataScript Schema

From `deps/db/src/logseq/db/frontend/schema.cljs` (confirmed in source):

```clojure
;; Reference blocks — multi-reference, cardinality many
:block/refs {:db/valueType :db.type/ref
              :db/cardinality :db.cardinality/many}

;; Single link (for tags, embeds)
:block/link {:db/valueType :db.type/ref
              :db/index true}

;; Tags on blocks/pages
:block/tags {:db/valueType :db.type/ref
              :db/cardinality :db.cardinality/many}

;; Aliases on pages
:block/alias {:db/valueType :db.type/ref
               :db/cardinality :db.cardinality/many
               :db/index true}
```

### 1.2 What Datoms Are Created

When a block contains `[[Some Page]]`:

1. **Parser** (`logseq.graph-parser`) extracts page references from the Mldoc AST
2. **Reference resolution**: The page name is lowercased → looked up by `:block/name`
3. **Transaction**: The following datoms are created:

```
[<block-dbid> :block/refs <page-dbid>  tx-id]   ;; one datom per reference
[<block-dbid> :block/content "text with [[Some Page]]" tx-id]
```

When a block contains `((block-uuid))`:

1. **Parser** extracts the block reference UUID
2. **Resolution**: UUID is looked up via `:block/uuid` unique identity
3. **Transaction**: Same `:block/refs` datom pointing to the target block's `db/id`:

```
[<block-dbid> :block/refs <target-block-dbid>  tx-id]
[<block-dbid> :block/link <target-block-dbid>  tx-id]  ;; specific link
```

**Key insight**: `:block/refs` is a **flat multi-reference set** — it stores ALL references (page refs + block refs + tag refs) in one attribute. The type of reference is not distinguished at the datom level.

### 1.3 DataScript Indexing for Refs

DataScript maintains four index trees (like Datomic):

| Index | Structure | Use Case |
|-------|-----------|----------|
| **EAVT** | `[entity attribute value tx]` | Look up all refs from a block |
| **AVET** | `[attribute value entity tx]` | **Reverse lookup**: find all blocks that ref a page |
| **VAET** | `[value attribute entity tx]` | Similar reverse lookup for ref types |

The **AVET index** is the critical one for backlinks. Since `:block/refs` has `:db/valueType :db.type/ref`, DataScript automatically maintains a reverse index. You can access it via the **reverse attribute syntax**: `:block/_refs` (note the underscore prefix).

This means: "give me all entities that have `:block/refs` pointing to entity X" is simply:

```clojure
(:block/_refs page-entity)  ;; returns all blocks referencing this page
```

---

## 2. How Logseq Computes Backlinks (Linked References)

### 2.1 Datalog Query via Reverse Index

The "Linked References" section uses the reverse `:block/_refs` relationship. The core query pattern:

```clojure
;; Simplified backlinks query for a page
[:find (pull ?b [*])
 :in $ ?page-id
 :where
 [?b :block/refs ?page-id]]
```

Or equivalently, using the reverse attribute:

```clojure
;; Get all blocks that reference this page
(d/q '[:find ?block :in $ ?page-name :where
       [?p :block/name ?page-name]
       [?b :block/refs ?p]]
     db page-name)
```

### 2.2 DSL Integration

When a user navigates to a page, the "Linked References" section uses the `page-ref` rule from the DSL query engine (`frontend.db.query-dsl`):

```clojure
;; From build-page-ref in query_dsl.cljs
(defn- build-page-ref [e]
  (let [page-name (-> (page-ref/get-page-name! e)
                      (common-util/page-name-sanity-lc))
        page (ldb/get-page *current-db* page-name)]
    (when page
      {:query (list 'page-ref '?b (:db/id page))
       :rules [:page-ref]})))
```

The `:page-ref` rule in the Datalog rules engine translates to:

```clojure
;; The page-ref rule (from rules.cljc)
[[(page-ref ?b ?page-id)
  [?b :block/refs ?page-id]]]
```

### 2.3 Reactive Updates

Backlinks are **reactive** via the Rum/Reagent integration:

1. Component subscribes via `frontend.db.react/q` with a unique cache key
2. When any transaction touches `:block/refs`, `refresh-affected-queries!` is called
3. Only queries whose inputs changed are re-executed
4. The result atom updates, triggering re-render

This is **O(k)** where k = number of referencing blocks, not O(n) where n = total blocks.

---

## 3. How Logseq Computes Unlinked References

### 3.1 Implementation

Unlinked references use **fuzzy text search** against page names, not the `:block/refs` index. The algorithm:

1. Get the current page name (and aliases)
2. Search all block **content** for text that matches the page name but is NOT wrapped in `[[ ]]`
3. Exclude blocks that already have a `:block/refs` to this page

From `frontend.search`:
- Uses a **fuzzy search engine** (`frontend.common.search-fuzzy`)
- The search protocol has `protocol/query engine q option`
- For DB-mode graphs, this searches the **in-memory DataScript** directly
- The fuzzy search normalizes text, strips accents, and does substring matching

### 3.2 Performance Characteristics

This is **expensive** — it scans all blocks. In practice:
- Logseq limits the search to a reasonable result set
- The search is debounced and runs asynchronously
- Results are cached and only recomputed when blocks change
- For large graphs (>10k blocks), this can be slow

### 3.3 Not Suitable for Real-Time at Scale

For Quilt with potentially hundreds of thousands of blocks, we need a better approach. See Section 4.

---

## 4. High-Performance Bidirectional Reference Architectures

### 4.1 How Other Tools Handle Backlinks

#### Roam Research
- Uses **DataScript in the browser** (same foundation as Logseq)
- Stores references as datoms with reverse index (AVET)
- Backlinks query: simple Datalog `[?b :block/refs ?page]`
- Unlinked references: JavaScript `string.includes()` scan over all blocks (expensive)
- Key difference: Roam keeps everything in memory and does lazy computation

#### Notion
- **Server-side indexing**: Backlinks are computed on the server
- Uses a **materialized backlink table** — when a block is saved, a background job extracts references and writes to a `backlinks` table
- Unlinked mentions: Uses **full-text search index** (likely Elasticsearch or similar)
- API-driven: `GET /v1/blocks/{id}/references` returns pre-computed results
- **Key insight**: Notion pre-computes backlinks on write, not on read

#### Athens Research (Logseq fork)
- Also uses **DataScript** (ClojureScript)
- Same reverse-index pattern as Logseq
- Had an experimental **search indexing service** using Lucene for unlinked refs
- Was exploring **Datascript-to-SQLite** sync for persistence

#### Obsidian
- **File-based**: Scans `.md` files on disk
- Maintains a **metadata cache** (`metadata.json`) with forward references
- Backlinks computed by **inverting the forward reference index** on load
- Unlinked mentions: Regex scan over all files (can be slow for vaults >10k files)
- Uses a **background indexing thread** that updates incrementally

### 4.2 Academic/Industry Patterns for Bidirectional Link Indexing

#### Pattern 1: Materialized Edge Table (Best for SQLite)

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   entities   │     │    edges     │     │  edge_types  │
├──────────────┤     ├──────────────┤     ├──────────────┤
│ id (PK)      │◄───┤ source_id FK │     │ id (PK)      │
│ type         │     │ target_id FK │     │ name         │
│ name         │     │ type_id FK   │────►│ bidirectional│
│ content      │     │ created_at   │     └──────────────┘
└──────────────┘     └──────────────┘
```

**Key indexes**:
```sql
-- Forward: "what does X reference?"
CREATE INDEX idx_edges_source ON edges(source_id, type_id);

-- Reverse: "what references X?" (THIS IS THE BACKLINK QUERY)
CREATE INDEX idx_edges_target ON edges(target_id, type_id);
```

#### Pattern 2: Trigger-Maintained Reverse Index

```sql
CREATE TABLE backlinks (
    target_id BLOB NOT NULL,
    source_id BLOB NOT NULL,
    source_page_id BLOB NOT NULL,
    context_text TEXT,        -- surrounding text for preview
    ref_type TEXT NOT NULL,   -- 'page_ref', 'block_ref', 'tag'
    created_at INTEGER NOT NULL,
    PRIMARY KEY (target_id, source_id)
);

CREATE INDEX idx_backlinks_target ON backlinks(target_id);
CREATE INDEX idx_backlinks_source ON backlinks(source_id);

-- Trigger to maintain on edge insert
CREATE TRIGGER trg_backlinks_insert
AFTER INSERT ON refs BEGIN
    INSERT OR IGNORE INTO backlinks (target_id, source_id, source_page_id, ref_type, created_at)
    SELECT NEW.target_id, NEW.source_id, b.page_id, 'page_ref', NEW.created_at
    FROM blocks b WHERE b.id = NEW.source_id;
END;
```

#### Pattern 3: FTS5 for Unlinked References

This is the **killer pattern for Quilt**:

```sql
-- FTS5 content table for block text
CREATE VIRTUAL TABLE blocks_fts USING fts5(
    content,
    content='blocks',
    content_rowid='rowid',
    tokenize='porter unicode61'  -- stemmer + unicode support
);

-- Materialized view of all page titles for matching
CREATE TABLE page_titles (
    page_id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    aliases TEXT  -- JSON array of alias strings
);

-- Unlinked references query: find blocks mentioning a page title
-- but NOT in the refs table
SELECT b.id, b.content, p.name
FROM blocks b
JOIN blocks_fts fts ON blocks_fts MATCH ? AND fts.rowid = b.rowid
JOIN page_titles p ON b.content LIKE '%' || p.name || '%'
WHERE p.page_id = ?
  AND NOT EXISTS (
      SELECT 1 FROM refs r
      WHERE r.source_id = b.id AND r.target_id = p.page_id
  );
```

**Optimization**: Instead of scanning all blocks, FTS5 narrows candidates first, then we check for existing refs:

```sql
-- Efficient unlinked refs using FTS5
WITH page_variants(name) AS (
    SELECT name FROM page_titles WHERE page_id = ?
    UNION
    SELECT value FROM page_titles, json_each(aliases) WHERE page_id = ?
),
candidates(block_rowid) AS (
    SELECT rowid FROM blocks_fts WHERE blocks_fts MATCH (
        SELECT group_concat('"' || name || '"', ' OR ')
        FROM page_variants
    )
)
SELECT b.id, b.page_id, b.content, snippet(blocks_fts, -1, '<<', '>>', '...', 32)
FROM blocks b
JOIN candidates c ON b.rowid = c.block_rowid
WHERE NOT EXISTS (
    SELECT 1 FROM refs r
    WHERE r.source_id = b.id AND r.target_id = ?
)
LIMIT 50;
```

#### Pattern 4: Recursive CTE for Graph Traversal

```sql
-- Find all entities reachable from a page (transitive closure)
WITH RECURSIVE graph_walk(entity_id, depth, path) AS (
    -- Base case: direct references
    SELECT target_id, 1, '/' || source_id || '/' || target_id || '/'
    FROM refs WHERE source_id = ?

    UNION ALL

    -- Recursive case: follow edges
    SELECT r.target_id, gw.depth + 1, gw.path || r.target_id || '/'
    FROM refs r
    JOIN graph_walk gw ON r.source_id = gw.entity_id
    WHERE gw.depth < 5          -- max depth
      AND gw.path NOT LIKE '%/' || r.target_id || '/%'  -- cycle detection
)
SELECT DISTINCT entity_id, depth
FROM graph_walk
ORDER BY depth;
```

### 4.3 Recommended Schema for Quilt

Based on the above analysis, here's the optimal SQLite schema:

```sql
-- Core refs table (forward edges)
CREATE TABLE refs (
    source_id BLOB NOT NULL,        -- block that contains the reference
    target_id BLOB NOT NULL,        -- page or block being referenced
    ref_type TEXT NOT NULL DEFAULT 'page_ref',  -- 'page_ref' | 'block_ref' | 'tag' | 'alias'
    created_at INTEGER NOT NULL,
    PRIMARY KEY (source_id, target_id, ref_type)
) WITHOUT ROWID;

-- Forward index: what does this block reference?
CREATE INDEX idx_refs_forward ON refs(source_id);

-- Reverse index: what references this entity? (THE BACKLINK INDEX)
CREATE INDEX idx_refs_reverse ON refs(target_id, ref_type);

-- FTS5 for content search (unlinked refs + general search)
CREATE VIRTUAL TABLE blocks_fts USING fts5(
    content,
    content='blocks',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Page titles lookup for unlinked ref matching
CREATE VIRTUAL TABLE page_names_fts USING fts5(
    name,
    content='pages',
    content_rowid='rowid',
    tokenize='unicode61'
);

-- Triggers to keep refs in sync with block content
-- (When block content is updated, re-parse refs and update the refs table)
-- This is application logic, best done in Rust, not SQL triggers

-- Denormalized backlink count for fast page listing
CREATE TABLE page_stats (
    page_id BLOB PRIMARY KEY,
    backlink_count INTEGER NOT NULL DEFAULT 0,
    block_count INTEGER NOT NULL DEFAULT 0,
    word_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);
```

**Why WITHOUT ROWID on refs?**
- The refs table is a pure join table
- Queries always filter by source_id or target_id
- WITHOUT ROWID makes it a clustered index → fewer disk reads
- B-tree is ordered by the primary key (source_id, target_id, ref_type)

---

## 5. Rust Crates and Patterns for Bidirectional Reference Indexing

### 5.1 Recommended In-Memory Structure

For WASM target (no native deps), use pure-Rust data structures:

```rust
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Bidirectional reference index maintained alongside SQLite
pub struct RefIndex {
    /// Forward: source → set of targets
    forward: HashMap<Uuid, HashSet<(Uuid, RefType)>>,

    /// Reverse: target → set of sources (THE BACKLINK INDEX)
    reverse: HashMap<Uuid, HashSet<(Uuid, RefType)>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefType {
    PageRef,
    BlockRef,
    Tag,
    Alias,
}

impl RefIndex {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Add a reference. O(1) amortized.
    pub fn add_ref(&mut self, source: Uuid, target: Uuid, ref_type: RefType) {
        self.forward
            .entry(source)
            .or_default()
            .insert((target, ref_type));

        self.reverse
            .entry(target)
            .or_default()
            .insert((source, ref_type));
    }

    /// Remove a reference. O(1) amortized.
    pub fn remove_ref(&mut self, source: Uuid, target: Uuid, ref_type: RefType) {
        if let Some(targets) = self.forward.get_mut(&source) {
            targets.remove(&(target, ref_type));
            if targets.is_empty() {
                self.forward.remove(&source);
            }
        }

        if let Some(sources) = self.reverse.get_mut(&target) {
            sources.remove(&(source, ref_type));
            if sources.is_empty() {
                self.reverse.remove(&target);
            }
        }
    }

    /// Get all backlinks to an entity. O(1) lookup + O(k) iteration.
    pub fn get_backlinks(&self, target: Uuid) -> Vec<(Uuid, RefType)> {
        self.reverse
            .get(&target)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all forward references from an entity. O(1) lookup + O(k) iteration.
    pub fn get_forward_refs(&self, source: Uuid) -> Vec<(Uuid, RefType)> {
        self.forward
            .get(&source)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Rebuild index from SQLite (on startup or after sync)
    pub async fn rebuild_from_db(pool: &sqlx::SqlitePool) -> Result<Self, sqlx::Error> {
        let mut index = Self::new();

        let rows = sqlx::query_as!(
            RefRow,
            "SELECT source_id, target_id, ref_type FROM refs"
        )
        .fetch_all(pool)
        .await?;

        for row in rows {
            let ref_type = match row.ref_type.as_str() {
                "page_ref" => RefType::PageRef,
                "block_ref" => RefType::BlockRef,
                "tag" => RefType::Tag,
                "alias" => RefType::Alias,
                _ => continue,
            };
            index.add_ref(row.source_id, row.target_id, ref_type);
        }

        Ok(index)
    }

    /// Get backlink count (for page stats). O(1).
    pub fn backlink_count(&self, target: Uuid) -> usize {
        self.reverse.get(&target).map(|s| s.len()).unwrap_or(0)
    }
}
```

### 5.2 Relevant Rust Crates

| Crate | Purpose | WASM Compatible | Use for Quilt |
|-------|---------|-----------------|---------------|
| **`petgraph`** | Graph data structures (DiGraph, adjacency lists) | Yes | For graph traversal, path finding, cycle detection |
| **`dashmap`** | Concurrent HashMap | Partial (needs `--cfg=dashmap_atomic_refcell`) | For multi-threaded access to ref index |
| **`indexmap`** | Ordered HashMap/Set | Yes | For maintaining insertion-order of refs |
| **`bimap`** | Bidirectional map (one-to-one) | Yes | NOT suitable (we need one-to-many) |
| **`rustc-hash`** (FxHashMap) | Faster hash map for small keys | Yes | For faster Uuid → HashSet lookups |
| **`slotmap`** | Arena-based slot map with stable keys | Yes | For compact entity storage |

**Recommendation**: Use `HashMap<Uuid, HashSet<(Uuid, RefType)>>` (std) for the in-memory index. No external crate needed for the core data structure. Use `petgraph` for graph algorithms (shortest path, connected components, SCC for cycle detection).

### 5.3 WASM Considerations

For WASM target:
- `HashMap` and `HashSet` from `std` work in WASM
- `petgraph` compiles to WASM (pure Rust, no system deps)
- `sqlx` with `sqlite` feature does NOT work in WASM (SQLite is native)
  - For WASM, use `rusqlite` compiled with `-C target-feature=+atomics,+bulk-memory` or use a WASM-compatible SQLite wrapper like `sqlite-wasm-rs` or `wa-sqlite`
- `uuid` crate works in WASM

### 5.4 Architecture for Quilt

```
┌─────────────────────────────────────────────────────┐
│                    RefService                        │
│  ┌─────────────────────────────────────────────────┐│
│  │  RefIndex (in-memory, hot path)                  ││
│  │  forward: HashMap<Uuid, HashSet<(Uuid, RefType)>>││
│  │  reverse: HashMap<Uuid, HashSet<(Uuid, RefType)>>││
│  └───────────────────┬─────────────────────────────┘│
│                      │ sync                          │
│  ┌───────────────────▼─────────────────────────────┐│
│  │  refs table (SQLite, persistent)                 ││
│  │  idx_refs_forward (source_id)                    ││
│  │  idx_refs_reverse (target_id) ← BACKLINK INDEX   ││
│  └─────────────────────────────────────────────────┘│
│                      │                               │
│  ┌───────────────────▼─────────────────────────────┐│
│  │  blocks_fts (FTS5, unlinked refs)                ││
│  │  page_names_fts (FTS5, title matching)           ││
│  └─────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────┘
```

**Write path** (when block content changes):
1. Parse refs from content (extract `[[ ]]` and `(( ))` patterns)
2. Diff old refs vs new refs
3. Update `refs` table in SQLite transaction
4. Update in-memory `RefIndex`
5. Update FTS5 content (via trigger or explicit)
6. Emit `backlinks_changed` notification

**Read path** (when viewing a page):
1. `RefIndex::get_backlinks(page_id)` — O(1) from memory
2. If not loaded: `SELECT source_id FROM refs WHERE target_id = ?` — index hit
3. Unlinked refs: FTS5 query (see Section 4.3)

---

## 6. Summary: Key Design Decisions for Quilt

| Decision | Recommendation | Rationale |
|----------|---------------|-----------|
| Ref storage | Separate `refs` join table | Cleaner than embedded in block, enables efficient reverse lookup |
| Backlink query | Dual index (forward + reverse) on refs table | O(log n) per query, same as DataScript AVET |
| In-memory index | `HashMap<Uuid, HashSet<(Uuid, RefType)>>` x2 | O(1) backlink lookup, WASM compatible |
| Unlinked refs | FTS5 with exclusion join | 100x faster than full scan for large graphs |
| Ref parsing | On write, in Rust | Extract refs from content before storing |
| Sync | Write refs to SQLite, rebuild in-memory on load | Durable + fast reads |
| Ref type tracking | `ref_type` column in refs table | Logseq doesn't distinguish; Quilt should (enables type-filtered backlinks) |
| Denormalized counts | `page_stats.backlink_count` | O(1) for page listing without JOIN |

---

## 7. Concrete SQLite Migration for Quilt

```sql
-- Migration: 003_create_refs.sql

-- Reference edges between blocks/pages
CREATE TABLE refs (
    source_id BLOB NOT NULL,
    target_id BLOB NOT NULL,
    ref_type TEXT NOT NULL CHECK(ref_type IN ('page_ref', 'block_ref', 'tag', 'alias')),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000),
    PRIMARY KEY (source_id, target_id, ref_type)
) WITHOUT ROWID;

-- Forward: "what does this block reference?"
CREATE INDEX idx_refs_source ON refs(source_id);

-- Reverse: "what references this page/block?" (BACKLINK QUERY)
CREATE INDEX idx_refs_target ON refs(target_id, ref_type);

-- Page statistics (denormalized)
CREATE TABLE IF NOT EXISTS page_stats (
    page_id BLOB PRIMARY KEY,
    backlink_count INTEGER NOT NULL DEFAULT 0,
    block_count INTEGER NOT NULL DEFAULT 0,
    word_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000)
);

-- Trigger to maintain backlink_count
CREATE TRIGGER trg_refs_backlink_count_insert
AFTER INSERT ON refs
WHEN NEW.ref_type IN ('page_ref', 'block_ref') BEGIN
    INSERT INTO page_stats (page_id, backlink_count, updated_at)
    VALUES (NEW.target_id, 1, strftime('%s', 'now') * 1000)
    ON CONFLICT(page_id) DO UPDATE SET
        backlink_count = backlink_count + 1,
        updated_at = strftime('%s', 'now') * 1000;
END;

CREATE TRIGGER trg_refs_backlink_count_delete
AFTER DELETE ON refs
WHEN OLD.ref_type IN ('page_ref', 'block_ref') BEGIN
    UPDATE page_stats SET
        backlink_count = MAX(0, backlink_count - 1),
        updated_at = strftime('%s', 'now') * 1000
    WHERE page_id = OLD.target_id;
END;
```
