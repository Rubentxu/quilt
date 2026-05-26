//! Search service using FTS5

use async_trait::async_trait;
use lru::LruCache;
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use thiserror::Error;

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
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let sanitized = Self::sanitize_fts5_query(query);
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

    /// Sanitize user input for FTS5.
    ///
    /// Wraps each whitespace-separated term in double quotes so FTS5 treats
    /// it as a literal phrase, avoiding syntax errors from special characters
    /// like `*`, `"`, `(`, `)`, `+`, `-`, `NEAR`, `AND`, `OR`, `NOT`.
    pub fn sanitize_fts5_query(query: &str) -> String {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return "\"\"".to_string();
        }
        trimmed
            .split_whitespace()
            .map(|term| {
                let clean = term.trim_matches('"');
                format!("\"{}\"", clean)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Build an FTS5 prefix-match query by appending `*` to each term.
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

    async fn setup_test_db() -> SqlitePool {
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

        // Insert a test page and blocks
        let page_id = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query("INSERT INTO pages (id, name, created_at, updated_at) VALUES (?, ?, 0, 0)")
            .bind(&page_id)
            .bind("Test Page")
            .execute(&pool)
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
    fn test_sanitize_fts5_query_normal() {
        let result = SearchService::sanitize_fts5_query("hello world");
        assert_eq!(result, r#""hello" "world""#);
    }

    #[test]
    fn test_sanitize_fts5_query_special_chars() {
        let result = SearchService::sanitize_fts5_query("foo* (bar)");
        assert_eq!(result, r#""foo*" "(bar)""#);
    }

    #[test]
    fn test_sanitize_fts5_query_empty() {
        let result = SearchService::sanitize_fts5_query("");
        assert_eq!(result, r#""""#);
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
}
