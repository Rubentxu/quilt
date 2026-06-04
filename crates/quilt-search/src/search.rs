//! Search service using FTS5

use async_trait::async_trait;
use lru::LruCache;
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use crate::sanitize::build_fts5_match_query;

/// Search result with ranking
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub block_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: String,
    pub score: f64,
}

/// FTS5 Search query result row
#[derive(Debug, sqlx::FromRow)]
pub struct FtsSearchRow {
    pub block_id: String,
    pub page_name: String,
    pub content: String,
    pub rank: f64,
}

/// Errors that can occur during search operations.
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("FTS5 query failed: {0}")]
    Fts5Query(#[from] sqlx::Error),

    #[error("Query sanitization failed: {0}")]
    Sanitization(String),

    #[error("Empty query: input produced no FTS5 tokens")]
    EmptyQuery,

    #[error("Cache error: {0}")]
    Cache(String),
}

/// Main search service trait — object-safe, async-only.
#[async_trait]
pub trait SearchServiceTrait: Send + Sync {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError>;
    async fn fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError>;
}

/// Search service with caching and FTS5 integration
pub struct SearchService {
    pool: Arc<SqlitePool>,
    cache: Arc<Mutex<LruCache<String, Vec<SearchResult>>>>,
}

impl SearchService {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self {
            pool,
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))),
        }
    }

    /// Execute a full-text search using FTS5 with bm25 ranking.
    ///
    /// Sanitizes the query string to avoid FTS5 syntax errors, then executes
    /// against the `blocks_fts` virtual table, joining with `blocks` and `pages`.
    ///
    /// Returns [`SearchError::EmptyQuery`] if the input produces no FTS5
    /// tokens (empty input, all-whitespace, or only FTS5 operators).
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let sanitized = build_fts5_match_query(query).ok_or(SearchError::EmptyQuery)?;
        let cache_key = format!("search:{}:{}", sanitized, limit);

        // Check cache first
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| SearchError::Cache(e.to_string()))?;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let results = self.search_fts(&sanitized, limit).await?;

        // Cache the result
        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| SearchError::Cache(e.to_string()))?;
            cache.put(cache_key, results.clone());
        }
        Ok(results)
    }

    /// Execute a low-level FTS5 query and return ranked rows.
    pub async fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let rows: Vec<FtsSearchRow> = sqlx::query_as::<_, FtsSearchRow>(
            r#"
            SELECT hex(b.id) as block_id, p.name as page_name, b.content, bm25(blocks_fts) as rank
            FROM blocks_fts fts
            JOIN blocks b ON fts.rowid = b.rowid
            JOIN pages p ON b.page_id = p.id
            WHERE blocks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(self.pool.as_ref())
        .await?;

        let results: Vec<SearchResult> = rows
            .into_iter()
            .map(|row| Self::row_to_result(row, query, 128))
            .collect();
        Ok(results)
    }

    /// Execute a fuzzy search with prefix matching and LIKE fallback.
    ///
    /// First tries FTS5 with `*` prefix matching on each term.
    /// Falls back to `LIKE '%term%'` pattern matching on both
    /// block contents and page names if FTS5 returns no results.
    pub async fn fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let cache_key = format!("fuzzy:{}:{}", query, limit);

        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| SearchError::Cache(e.to_string()))?;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Try FTS5 prefix search first
        let fuzzy_query = Self::build_fuzzy_query(query);
        let mut results = self.search_fts(&fuzzy_query, limit).await?;

        // Fallback to LIKE if FTS5 returned nothing
        if results.is_empty() {
            results = self.like_fallback_search(query, limit).await?;
        }

        // Apply custom scoring: shorter page names and exact matches rank higher
        let lower_query = query.to_lowercase();
        for r in &mut results {
            let content_lower = r.content.to_lowercase();
            if content_lower == lower_query {
                r.score = 1.0;
            } else if content_lower.starts_with(&lower_query) {
                r.score = 0.8;
            } else {
                r.score = 0.5;
            }
        }
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        {
            let mut cache = self
                .cache
                .lock()
                .map_err(|e| SearchError::Cache(e.to_string()))?;
            cache.put(cache_key, results.clone());
        }
        Ok(results)
    }

    /// LIKE-based fallback search on block content and page names.
    async fn like_fallback_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let like_pattern = format!("%{}%", query);
        let rows = sqlx::query_as::<_, FtsSearchRow>(
            r#"
            SELECT DISTINCT hex(b.id) as block_id, p.name as page_name, b.content, 0.0 as rank
            FROM blocks b
            JOIN pages p ON b.page_id = p.id
            WHERE b.content LIKE ? OR p.name LIKE ?
            ORDER BY p.name
            LIMIT ?
            "#,
        )
        .bind(&like_pattern)
        .bind(&like_pattern)
        .bind(limit as i64)
        .fetch_all(self.pool.as_ref())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| Self::row_to_result(row, query, 128))
            .collect())
    }

    /// Build an FTS5 prefix-match query by appending `*` to each term.
    ///
    /// Used by [`Self::fuzzy_search`]. This is a separate concern from
    /// the safety sanitization in [`crate::sanitize`]: fuzzy search wants
    /// prefix matching, not literal phrases.
    pub fn build_fuzzy_query(query: &str) -> String {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return "\"*\"".to_string();
        }
        trimmed
            .split_whitespace()
            .map(|term| {
                let clean: String = term.chars().filter(|c| c.is_alphanumeric()).collect();
                if clean.is_empty() {
                    "*".to_string()
                } else {
                    format!("{}*", clean)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Generate a snippet with highlighted matches
    pub fn generate_snippet(content: &str, _query: &str, max_len: usize) -> String {
        if content.len() <= max_len {
            return content.to_string();
        }
        // Simple truncation with ellipsis
        format!("{}...", &content[..max_len.saturating_sub(3)])
    }

    /// Convert FTS row to SearchResult
    pub fn row_to_result(row: FtsSearchRow, query: &str, snippet_len: usize) -> SearchResult {
        SearchResult {
            block_id: row.block_id,
            page_name: row.page_name,
            content: row.content.clone(),
            snippet: Self::generate_snippet(&row.content, query, snippet_len),
            score: row.rank,
        }
    }

    /// Clear the search cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }
}

#[async_trait]
impl SearchServiceTrait for SearchService {
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        SearchService::search(self, query, limit).await
    }
    async fn fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        SearchService::fuzzy_search(self, query, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Create schema, triggers, and a test page (no blocks).
    /// Used by edge case tests that need precise data control.
    async fn setup_schema() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        sqlx::query(
            r#"
            CREATE TABLE pages (
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
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TABLE blocks (
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
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE VIRTUAL TABLE blocks_fts USING fts5(
                content,
                content=blocks,
                content_rowid=rowid
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Triggers for keeping FTS in sync
        sqlx::query(
            r#"
            CREATE TRIGGER blocks_ai AFTER INSERT ON blocks BEGIN
                INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
            END
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TRIGGER blocks_ad AFTER DELETE ON blocks BEGIN
                INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
            END
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            CREATE TRIGGER blocks_au AFTER UPDATE ON blocks BEGIN
                INSERT INTO blocks_fts(blocks_fts, rowid, content) VALUES('delete', old.rowid, old.content);
                INSERT INTO blocks_fts(rowid, content) VALUES (new.rowid, new.content);
            END
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert a test page (no blocks)
        let page_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO pages (id, name, created_at, updated_at) VALUES (?, ?, 0, 0)")
            .bind(&page_id)
            .bind("Test Page")
            .execute(&pool)
            .await
            .unwrap();

        pool
    }

    /// Create full test DB with schema + 3 default blocks.
    async fn setup_test_db() -> SqlitePool {
        let pool = setup_schema().await;

        let page_id: Vec<u8> = sqlx::query_scalar("SELECT id FROM pages LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();

        let block_ids: Vec<Vec<u8>> = (0..3)
            .map(|_| uuid::Uuid::new_v4().as_bytes().to_vec())
            .collect();

        let contents = [
            "Hello world from Rust",
            "Full-text search is powerful",
            "Rust type system prevents errors",
        ];

        for (i, content) in contents.iter().enumerate() {
            sqlx::query(
                "INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)"
            )
            .bind(&block_ids[i])
            .bind(&page_id)
            .bind(content)
            .execute(&pool)
            .await
            .unwrap();
        }

        pool
    }

    #[test]
    fn test_generate_snippet_short() {
        let content = "Short content";
        let snippet = SearchService::generate_snippet(content, "test", 50);
        assert_eq!(snippet, "Short content");
    }

    #[test]
    fn test_generate_snippet_long() {
        let content = "This is a very long content that should be truncated";
        let snippet = SearchService::generate_snippet(content, "test", 20);
        assert!(snippet.ends_with("..."));
        assert!(snippet.len() <= 20);
    }

    #[test]
    fn test_build_fuzzy_query() {
        let result = SearchService::build_fuzzy_query("hello world");
        assert_eq!(result, "hello* world*");
    }

    #[test]
    fn test_build_fuzzy_query_special_chars() {
        let result = SearchService::build_fuzzy_query("foo* (bar)");
        assert_eq!(result, "foo* bar*");
    }

    #[test]
    fn test_build_fuzzy_query_empty() {
        let result = SearchService::build_fuzzy_query("");
        assert_eq!(result, r#""*""#);
    }

    #[test]
    fn test_row_to_result() {
        let row = FtsSearchRow {
            block_id: "abc".to_string(),
            page_name: "Test Page".to_string(),
            content: "This is test content".to_string(),
            rank: -1.5,
        };
        let result = SearchService::row_to_result(row, "test", 50);
        assert_eq!(result.block_id, "abc");
        assert_eq!(result.page_name, "Test Page");
        assert_eq!(result.score, -1.5);
    }

    #[tokio::test]
    async fn test_search_finds_results() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        let results = service.search("rust", 10).await.unwrap();
        assert!(
            results.len() >= 2,
            "Expected at least 2 results for 'rust', got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn test_search_no_results() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        let results = service.search("nonexistent", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_fuzzy_search_prefix() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        // "Hel" should match "Hello" via prefix matching
        let results = service.fuzzy_search("Hel", 10).await.unwrap();
        assert!(
            !results.is_empty(),
            "Fuzzy search should find 'Hello world from Rust'"
        );
    }

    #[tokio::test]
    async fn test_fuzzy_search_like_fallback() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        // "power" should match "powerful" via LIKE fallback
        let results = service.fuzzy_search("power", 10).await.unwrap();
        assert!(
            !results.is_empty(),
            "LIKE fallback should find 'Full-text search is powerful'"
        );
    }

    #[tokio::test]
    async fn test_search_cache_hit() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        let first = service.search("rust", 10).await.unwrap();
        let second = service.search("rust", 10).await.unwrap();
        assert_eq!(first.len(), second.len());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let pool = setup_test_db().await;
        let service = SearchService::new(Arc::new(pool));

        let _ = service.search("rust", 10).await.unwrap();
        service.clear_cache();
        // Should still work after cache clear
        let results = service.search("rust", 10).await.unwrap();
        assert!(results.len() >= 2);
    }

    // ── Edge case test helpers ──

    /// Test helper with both a pool (for raw SQL) and a SearchService.
    /// Uses schema-only setup (no default blocks) for precise data control.
    struct TestEnv {
        pool: SqlitePool,
        service: SearchService,
    }

    impl TestEnv {
        async fn new() -> Self {
            let pool = setup_schema().await;
            let pool_clone = pool.clone();
            let service = SearchService::new(Arc::new(pool));
            Self {
                pool: pool_clone,
                service,
            }
        }

        async fn insert_block(&self, content: &str) {
            let id = uuid::Uuid::new_v4().as_bytes().to_vec();
            let page_id: Vec<u8> = sqlx::query_scalar("SELECT id FROM pages LIMIT 1")
                .fetch_one(&self.pool)
                .await
                .unwrap();
            sqlx::query(
                "INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)",
            )
            .bind(&id)
            .bind(&page_id)
            .bind(content)
            .execute(&self.pool)
            .await
            .unwrap();
        }

        async fn insert_blocks(&self, contents: &[&str]) {
            let page_id: Vec<u8> = sqlx::query_scalar("SELECT id FROM pages LIMIT 1")
                .fetch_one(&self.pool)
                .await
                .unwrap();
            for &content in contents {
                let id = uuid::Uuid::new_v4().as_bytes().to_vec();
                sqlx::query(
                    "INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)",
                )
                .bind(&id)
                .bind(&page_id)
                .bind(content)
                .execute(&self.pool)
                .await
                .unwrap();
            }
        }

        #[allow(dead_code)]
        async fn count_blocks(&self) -> i64 {
            sqlx::query_scalar("SELECT COUNT(*) FROM blocks")
                .fetch_one(&self.pool)
                .await
                .unwrap()
        }
    }

    // ── Edge case tests ──

    /// Test 1: Exact word match  |  Test 11: Case insensitivity  |  Test 13: Multi-word AND
    #[tokio::test]
    async fn test_fts5_exact_match() {
        let env = TestEnv::new().await;
        env.insert_block("hello world").await;
        env.insert_block("red apple").await;
        env.insert_block("blue apple").await;
        env.service.clear_cache();

        // Single word
        let results = env.service.search("hello", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find 'hello' in 'hello world'");

        let results = env.service.search("world", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find 'world' in 'hello world'");

        // Multi-word AND (both terms required)
        let results = env.service.search("hello world", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Both terms 'hello' and 'world' must match"
        );

        // Multi-word AND — "red apple" only
        let results = env.service.search("red apple", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Only one block contains both 'red' and 'apple'"
        );

        // Single word across multiple blocks
        let results = env.service.search("apple", 10).await.unwrap();
        assert_eq!(results.len(), 2, "Two blocks contain 'apple'");
    }

    /// Test 3: No match
    #[tokio::test]
    async fn test_fts5_no_match() {
        let env = TestEnv::new().await;
        env.insert_block("hello world").await;
        env.service.clear_cache();

        let results = env.service.search("nonexistent_term", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "Non-matching term should return 0 results"
        );
    }

    /// Test 4: Empty query
    #[tokio::test]
    async fn test_fts5_empty_query() {
        let env = TestEnv::new().await;
        env.insert_block("hello world").await;
        env.service.clear_cache();

        // Empty input produces no FTS5 tokens → EmptyQuery error.
        // Callers (HTTP handler) map this to 400 BadRequest.
        let result = env.service.search("", 10).await;
        assert!(matches!(result, Err(SearchError::EmptyQuery)));
    }

    /// Test 11: Case insensitivity (FTS5 is case-insensitive by default)
    #[tokio::test]
    async fn test_fts5_case_insensitivity() {
        let env = TestEnv::new().await;
        env.insert_block("Hello World").await;
        env.service.clear_cache();

        let results = env.service.search("hello", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Lowercase query should match 'Hello World'"
        );

        let results = env.service.search("HELLO", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Uppercase query should match 'Hello World'"
        );

        let results = env.service.search("Hello", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Title-case query should match 'Hello World'"
        );
    }

    /// Test 2: Partial match via fuzzy_search (prefix matching)
    #[tokio::test]
    async fn test_fts5_partial_match() {
        let env = TestEnv::new().await;
        env.insert_block("testing").await;
        env.insert_block("developer").await;
        env.service.clear_cache();

        // fuzzy_search appends * to each term for FTS5 prefix matching
        let results = env.service.fuzzy_search("test", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Prefix 'test' should match 'testing'");

        let results = env.service.fuzzy_search("develop", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Prefix 'develop' should match 'developer'"
        );

        // search() wraps in quotes — no prefix matching, so no match for partial term
        let results = env.service.search("test", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "search() wraps terms in quotes — partial 'test' should not match 'testing'"
        );
    }

    /// Test 5: Unicode and emoji support
    #[tokio::test]
    async fn test_fts5_unicode() {
        let env = TestEnv::new().await;
        env.insert_block("café").await;
        env.insert_block("日本語").await;
        env.insert_block("🎉 party").await;
        env.insert_block("über cool").await;
        env.service.clear_cache();

        // Basic unicode
        let results = env.service.search("café", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find 'café'");

        // CJK characters
        let results = env.service.search("日本語", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find '日本語'");

        // Emoji followed by text — search for the text part
        let results = env.service.search("party", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find 'party' after emoji");

        // Accented characters
        let results = env.service.search("über", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find 'über'");
    }

    /// Test 6: Special characters that FTS5 interprets as operators.
    /// `build_fts5_match_query` quotes each term AND strips FTS5 boolean
    /// operator words (AND, OR, NOT, NEAR), so user-typed operators
    /// never reach the FTS5 parser.
    #[tokio::test]
    async fn test_fts5_special_characters() {
        let env = TestEnv::new().await;
        env.insert_block("foo bar baz").await;
        env.insert_block("foo or bar").await;
        env.insert_block("required excluded").await;
        env.service.clear_cache();

        // All of these should NOT crash (sanitization prevents FTS5 syntax errors)
        let queries = [
            "foo bar",             // space-separated → AND (sanitized to "foo" AND "bar")
            "foo*",                // asterisk → prefix (sanitized to "foo"*)
            "foo OR bar",          // OR → stripped, leaves "foo" AND "bar"
            "foo NEAR bar",        // NEAR → stripped
            "foo NOT bar",         // NOT → stripped
            "+required -excluded", // + and - are edge-trimmed; tokens are "required" and "excluded"
            "\"foo bar\"",         // quotes stripped from edges; leaves "foo" AND "bar"
        ];

        for query in queries {
            let results = env.service.search(query, 10).await;
            assert!(
                results.is_ok(),
                "Query '{query}' should not crash FTS5, got: {:?}",
                results
            );
        }
    }

    /// Test 7: SQL injection / FTS5 injection attempts
    #[tokio::test]
    async fn test_fts5_injection_safety() {
        let env = TestEnv::new().await;
        env.insert_block("safe content").await;
        env.service.clear_cache();

        // SQL injection attempts — should be treated as literal queries, not executed
        let injection_queries = [
            "'; DROP TABLE blocks; --",
            "1; SELECT * FROM blocks;",
            "' OR '1'='1",
            "\"; DROP TABLE pages; --",
            "../etc/passwd",
            "<script>alert('xss')</script>",
        ];

        for query in injection_queries {
            let results = env.service.search(query, 10).await;
            assert!(
                results.is_ok(),
                "Injection query '{query}' should not crash: {:?}",
                results
            );
        }
    }

    /// Test 8: Very long query string (10KB)
    #[tokio::test]
    async fn test_fts5_long_query() {
        let env = TestEnv::new().await;
        env.insert_block("needle in a haystack").await;
        env.service.clear_cache();

        // Generate 10KB query
        let large_query = "a".repeat(10_000);

        let results = env.service.search(&large_query, 10).await;
        assert!(
            results.is_ok(),
            "10KB query should not crash: {:?}",
            results
        );
        assert!(
            results.unwrap().is_empty(),
            "Random 10KB string should not match"
        );
    }

    /// Test 8b: Very long query with a real word at the end
    #[tokio::test]
    async fn test_fts5_long_query_with_target() {
        let env = TestEnv::new().await;
        env.insert_block("needle in a haystack").await;
        env.service.clear_cache();

        // Prepend 10KB of padding before a real search term
        let padding = "a".repeat(10_000);
        let large_query = format!("{} needle", padding);

        let results = env.service.search(&large_query, 10).await.unwrap();
        // Sanitized: "aaaa...a" "needle" — the padding term doesn't match anything
        assert_eq!(
            results.len(),
            0,
            "Sanitization splits on whitespace, so the padding is a separate term that doesn't match"
        );
    }

    /// Test 9: Very long content (1MB block)
    #[tokio::test]
    async fn test_fts5_very_long_content() {
        let env = TestEnv::new().await;

        // Build 1MB content with "target_word" in the middle
        let prefix = "x".repeat(500_000);
        let suffix = "y".repeat(524_280); // remaining to approx 1MB
        let content = format!("{} target_word {}", prefix, suffix);
        assert!(content.len() > 1_000_000, "Content should be >1MB");

        env.insert_block(&content).await;
        env.service.clear_cache();

        let results = env.service.search("target_word", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Should find term in 1MB content");
    }

    /// Test 10: Many blocks (pagination via limit)
    #[tokio::test]
    async fn test_fts5_many_blocks() {
        let env = TestEnv::new().await;

        // Insert 100 blocks with varied content
        let mut contents = Vec::new();
        for i in 0..100 {
            contents.push(format!("unique content block number {}", i));
        }
        let str_refs: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();
        env.insert_blocks(&str_refs).await;
        env.service.clear_cache();

        // Search with different limits
        let results_all = env.service.search("content", 200).await.unwrap();
        assert_eq!(results_all.len(), 100, "Should find all 100 blocks");

        let results_limited = env.service.search("content", 10).await.unwrap();
        assert_eq!(
            results_limited.len(),
            10,
            "Limit=10 should return 10 results"
        );

        let results_specific = env.service.search("block number 42", 10).await.unwrap();
        assert_eq!(results_specific.len(), 1, "Should find the specific block");
    }

    /// Test 12: Block creation / update / deletion stays in sync with FTS5 index via triggers
    #[tokio::test]
    async fn test_fts5_trigger_sync() {
        let env = TestEnv::new().await;

        // Create block — triggers blocks_ai → should be findable
        let id = uuid::Uuid::new_v4().as_bytes().to_vec();
        let page_id: Vec<u8> = sqlx::query_scalar("SELECT id FROM pages LIMIT 1")
            .fetch_one(&env.pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO blocks (id, page_id, content, created_at, updated_at) VALUES (?, ?, ?, 0, 0)",
        )
        .bind(&id)
        .bind(&page_id)
        .bind("fresh content")
        .execute(&env.pool)
        .await
        .unwrap();
        env.service.clear_cache();

        let results = env.service.search("fresh", 10).await.unwrap();
        assert_eq!(results.len(), 1, "Trigger should auto-index inserted block");

        // Update block content — triggers blocks_au → old removed, new added
        sqlx::query("UPDATE blocks SET content = ? WHERE id = ?")
            .bind("updated content")
            .bind(&id)
            .execute(&env.pool)
            .await
            .unwrap();
        env.service.clear_cache();

        let results = env.service.search("fresh", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "Old term 'fresh' should NOT match after update"
        );

        let results = env.service.search("updated", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "New term 'updated' should match after update"
        );

        // Delete block — triggers blocks_ad → removed from FTS
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(&id)
            .execute(&env.pool)
            .await
            .unwrap();
        env.service.clear_cache();

        let results = env.service.search("updated", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "Term should NOT match after block deletion"
        );
    }

    /// Test 14: Phrase search (via raw search_fts, bypassing sanitization)
    #[tokio::test]
    async fn test_fts5_phrase_search() {
        let env = TestEnv::new().await;
        env.insert_block("the quick brown fox jumps").await;
        env.insert_block("the slow brown bear sleeps").await;
        env.service.clear_cache();

        // Phrase match via search_fts (bypasses sanitize)
        // In FTS5, "quick brown" (quoted) is a phrase match — words must appear consecutively
        let results = env.service.search_fts("\"quick brown\"", 10).await.unwrap();
        assert_eq!(
            results.len(),
            1,
            "Phrase 'quick brown' should match exactly"
        );

        // Wrong order should NOT match
        let results = env.service.search_fts("\"brown quick\"", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "Phrase 'brown quick' should not match (wrong order)"
        );

        // Verify that AND search (via search_fts without phrase) matches both
        let results = env.service.search_fts("brown", 10).await.unwrap();
        assert_eq!(
            results.len(),
            2,
            "Single word 'brown' should match both blocks"
        );
    }

    /// Test 15: fuzzy_search LIKE fallback covers page titles
    #[tokio::test]
    async fn test_fts5_fuzzy_search_titles() {
        let env = TestEnv::new().await;
        // Only insert a block with unrelated content
        env.insert_block("some random content here").await;
        env.service.clear_cache();

        // search() only finds indexed content, not page names
        let results = env.service.search("Test Page", 10).await.unwrap();
        assert!(
            results.is_empty(),
            "search() only uses FTS5 on content, page name is not indexed"
        );

        // fuzzy_search first tries FTS5 prefix, then falls back to LIKE
        // The LIKE fallback searches both content and page name
        let results = env.service.fuzzy_search("Test", 10).await.unwrap();
        assert!(
            results.len() >= 1,
            "fuzzy_search LIKE fallback should find page named 'Test Page'"
        );
    }

    /// Test 16: Performance benchmarks
    #[tokio::test]
    async fn test_fts5_performance_small() {
        let env = TestEnv::new().await;

        // Insert 100 blocks
        let mut contents = Vec::new();
        for i in 0..100 {
            contents.push(format!("benchmark content line {}", i));
        }
        let str_refs: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();
        env.insert_blocks(&str_refs).await;
        env.service.clear_cache();

        let start = std::time::Instant::now();
        let results = env.service.search("benchmark", 50).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 50, "Should return up to 50 results");
        assert!(
            elapsed.as_millis() < 5000,
            "Search over 100 blocks should be fast (<5s), took {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_fts5_performance_larger() {
        let env = TestEnv::new().await;

        // Insert 500 blocks
        let mut contents = Vec::new();
        for i in 0..500 {
            contents.push(format!("performance test entry {}", i));
        }
        let str_refs: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();
        env.insert_blocks(&str_refs).await;
        env.service.clear_cache();

        let start = std::time::Instant::now();
        let results = env.service.search("performance", 100).await.unwrap();
        let elapsed = start.elapsed();

        assert_eq!(results.len(), 100);
        assert!(
            elapsed.as_millis() < 10_000,
            "Search over 500 blocks should complete (<10s), took {:?}",
            elapsed
        );
    }

    /// Test fuzzy_search LIKE fallback with partial text match
    #[tokio::test]
    async fn test_fts5_like_fallback_partial() {
        let env = TestEnv::new().await;
        env.insert_block("the word is powerful").await;
        env.service.clear_cache();

        // FTS5 prefix for "power" — should match "powerful" via FTS5 prefix directly
        let results = env.service.fuzzy_search("power", 10).await.unwrap();
        assert!(
            !results.is_empty(),
            "fuzzy_search should find 'powerful' via prefix match"
        );
    }
}
