//! Search service using FTS5

use lru::LruCache;
use sqlx::SqlitePool;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tracing::instrument;

/// Circuit breaker states
const CB_CLOSED: u8 = 0;
const CB_OPEN: u8 = 1;
const CB_HALF_OPEN: u8 = 2;

/// Circuit breaker for search resilience
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    failure_count: Arc<AtomicU32>,
    last_failure: Arc<AtomicU64>,
    state: Arc<AtomicU8>,
    last_success: Arc<AtomicU64>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker in the closed (normal) state.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
            last_failure: Arc::new(AtomicU64::new(0)),
            state: Arc::new(AtomicU8::new(CB_CLOSED)),
            last_success: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns true if the circuit is open (rejecting requests without trying).
    #[allow(dead_code)]
    pub fn is_open(&self) -> bool {
        self.state.load(Ordering::Relaxed) == CB_OPEN
    }

    /// Returns true if the circuit is half-open (allowing a trial request).
    #[allow(dead_code)]
    pub fn is_half_open(&self) -> bool {
        self.state.load(Ordering::Relaxed) == CB_HALF_OPEN
    }

    /// Record a successful operation — resets failure count and closes the circuit.
    #[allow(dead_code)]
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state.store(CB_CLOSED, Ordering::Relaxed);
        self.last_success.store(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            Ordering::Relaxed,
        );
    }

    /// Record a failed operation — increments failure count and may open the circuit.
    #[allow(dead_code)]
    pub fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count >= 3 {
            self.state.store(CB_OPEN, Ordering::Relaxed);
            self.last_failure.store(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                Ordering::Relaxed,
            );
        }
    }

    /// Check if the circuit should transition from Open to HalfOpen after cooldown.
    /// Returns true if the transition to HalfOpen was made.
    #[allow(dead_code)]
    pub fn try_reset(&self) -> bool {
        if self.state.load(Ordering::Relaxed) == CB_OPEN {
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .saturating_sub(self.last_failure.load(Ordering::Relaxed));
            if elapsed >= 30 {
                self.state.store(CB_HALF_OPEN, Ordering::Relaxed);
                return true;
            }
        }
        false
    }

    /// Returns the number of seconds until the circuit can try to reset.
    #[allow(dead_code)]
    pub fn retry_after_secs(&self) -> u64 {
        if self.state.load(Ordering::Relaxed) == CB_OPEN {
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .saturating_sub(self.last_failure.load(Ordering::Relaxed));
            30u64.saturating_sub(elapsed)
        } else {
            0
        }
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Search errors including circuit breaker state
#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Search temporarily unavailable. Retry after {0} seconds.")]
    CircuitOpen(u64),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}

impl From<SearchError> for String {
    fn from(e: SearchError) -> String {
        e.to_string()
    }
}

/// Check if a sqlx error is transient and worth retrying.
fn is_transient_error(e: &sqlx::Error) -> bool {
    match e {
        sqlx::Error::Database(db) => db
            .code()
            .map(|c| c == "SQLITE_BUSY" || c == "SQLITE_LOCKED")
            .unwrap_or(false),
        sqlx::Error::Io(_) => true,
        sqlx::Error::PoolTimedOut => true,
        _ => false,
    }
}

/// Retry a database operation with exponential backoff.
/// Returns the result of the operation, or a SearchError on failure.
async fn retry_db_op<F, Fut, T>(mut op: F) -> Result<T, SearchError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, sqlx::Error>>,
{
    // Spec: 5s base delay, 60s max per retry, 300s max total elapsed
    // Delays: [5s, 10s, 20s, 40s, 60s] (5 retries, doubling each time)
    let delays = [5_000u64, 10_000, 20_000, 40_000, 60_000]; // ms
    let max_elapsed = 300_000; // 300s in ms
    let start = std::time::Instant::now();

    for (i, delay) in delays.iter().enumerate() {
        // Check if we've exceeded max elapsed time
        if start.elapsed().as_millis() as u64 > max_elapsed {
            return Err(SearchError::DatabaseError(
                "Retry operation exceeded max elapsed time of 300s".to_string(),
            ));
        }

        match op().await {
            Ok(result) => return Ok(result),
            Err(e) if i == delays.len() - 1 => {
                return Err(SearchError::DatabaseError(e.to_string()));
            }
            Err(e) if !is_transient_error(&e) => {
                return Err(SearchError::DatabaseError(e.to_string()));
            }
            Err(_e) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*delay)).await;
            }
        }
    }
    unreachable!("retry_db_op should always return in the loop")
}

/// Search result with ranking
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: String,
    pub score: f64,
}

/// FTS5 Search query result row
#[derive(Debug, sqlx::FromRow)]
pub struct FtsSearchRow {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub rank: f64,
}

/// Search service with caching and FTS5 integration
pub struct SearchService {
    pool: SqlitePool,
    cache: Arc<Mutex<LruCache<String, Vec<SearchResult>>>>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl SearchService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))),
            circuit_breaker: Arc::new(CircuitBreaker::new()),
        }
    }

    /// Create a new SearchService with a custom circuit breaker (for testing).
    #[allow(dead_code)]
    pub fn with_circuit_breaker(pool: SqlitePool, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            pool,
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1000).unwrap()))),
            circuit_breaker,
        }
    }

    /// Execute a full-text search using FTS5 with bm25 ranking.
    ///
    /// Sanitizes the query string to avoid FTS5 syntax errors, then executes
    /// against the `blocks_fts` virtual table, joining with `blocks` and `pages`.
    ///
    /// Uses a circuit breaker to prevent cascading failures and retries with
    /// exponential backoff on transient database errors.
    #[instrument(skip(self))]
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let sanitized = Self::sanitize_fts5_query(query);
        let cache_key = format!("search:{}:{}", sanitized, limit);

        // Check cache first — cache hits bypass the circuit breaker entirely
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Check circuit breaker before attempting DB operation
        if self.circuit_breaker.is_open() {
            let retry_after = self.circuit_breaker.retry_after_secs();
            return Err(SearchError::CircuitOpen(retry_after));
        }

        // Try reset if circuit is in open state (checking cooldown)
        let _ = self.circuit_breaker.try_reset();

        // Execute the FTS query with retry wrapper
        let pool = &self.pool;
        let cb = &self.circuit_breaker;

        let result = retry_db_op(|| {
            let sanitized = sanitized.clone();
            async move {
                let rows: Vec<FtsSearchRow> = sqlx::query_as::<_, FtsSearchRow>(
                    r#"
                    SELECT hex(b.id) as block_id, hex(b.page_id) as page_id, p.name as page_name, b.content, bm25(blocks_fts) as rank
                    FROM blocks_fts fts
                    JOIN blocks b ON fts.rowid = b.rowid
                    JOIN pages p ON b.page_id = p.id
                    WHERE blocks_fts MATCH ?
                    ORDER BY rank
                    LIMIT ?
                    "#,
                )
                .bind(&sanitized)
                .bind(limit as i64)
                .fetch_all(pool)
                .await?;

                Ok(rows)
            }
        })
        .await;

        match result {
            Ok(rows) => {
                cb.record_success();
                let results: Vec<SearchResult> = rows
                    .into_iter()
                    .map(|row| Self::row_to_result(row, &sanitized, 128))
                    .collect();

                // Cache the result
                {
                    let mut cache = self.cache.lock().unwrap();
                    cache.put(cache_key, results.clone());
                }
                Ok(results)
            }
            Err(e) => {
                cb.record_failure();
                Err(e)
            }
        }
    }

    /// Execute a low-level FTS5 query and return ranked rows.
    #[instrument(skip(self))]
    pub async fn search_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let rows: Vec<FtsSearchRow> = sqlx::query_as::<_, FtsSearchRow>(
                    r#"
                    SELECT hex(b.id) as block_id, hex(b.page_id) as page_id, p.name as page_name, b.content, bm25(blocks_fts) as rank
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
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SearchError::DatabaseError(format!("FTS5 query failed: {}", e)))?;

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
    ///
    /// Uses the circuit breaker to fail fast when the database is unhealthy.
    #[instrument(skip(self))]
    pub async fn fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let cache_key = format!("fuzzy:{}:{}", query, limit);

        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Check circuit breaker before attempting DB operations
        if self.circuit_breaker.is_open() {
            let retry_after = self.circuit_breaker.retry_after_secs();
            return Err(SearchError::CircuitOpen(retry_after));
        }

        let _ = self.circuit_breaker.try_reset();

        // Try FTS5 prefix search first with retry
        let fuzzy_query = Self::build_fuzzy_query(query);
        let pool = &self.pool;
        let cb = &self.circuit_breaker;

        let fts_result = retry_db_op(|| {
            let fuzzy_query = fuzzy_query.clone();
            async move {
                let rows: Vec<FtsSearchRow> = sqlx::query_as::<_, FtsSearchRow>(
                    r#"
                    SELECT hex(b.id) as block_id, hex(b.page_id) as page_id, p.name as page_name, b.content, bm25(blocks_fts) as rank
                    FROM blocks_fts fts
                    JOIN blocks b ON fts.rowid = b.rowid
                    JOIN pages p ON b.page_id = p.id
                    WHERE blocks_fts MATCH ?
                    ORDER BY rank
                    LIMIT ?
                    "#,
                )
                .bind(&fuzzy_query)
                .bind(limit as i64)
                .fetch_all(pool)
                .await?;
                Ok(rows)
            }
        })
        .await;

        let mut results: Vec<SearchResult> = match fts_result {
            Ok(rows) => {
                cb.record_success();
                rows.into_iter()
                    .map(|row| Self::row_to_result(row, &fuzzy_query, 128))
                    .collect()
            }
            Err(e) => {
                cb.record_failure();
                return Err(e);
            }
        };

        // Fallback to LIKE if FTS5 returned nothing (with retry)
        if results.is_empty() {
            let pool = &self.pool;
            let fallback_result = retry_db_op(|| {
                let query = query.to_string();
                async move {
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
                    .fetch_all(pool)
                    .await?;
                    Ok(rows)
                }
            })
            .await;

            match fallback_result {
                Ok(rows) => {
                    self.circuit_breaker.record_success();
                    results = rows
                        .into_iter()
                        .map(|row| Self::row_to_result(row, query, 128))
                        .collect();
                }
                Err(e) => {
                    self.circuit_breaker.record_failure();
                    return Err(e);
                }
            }
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
            let mut cache = self.cache.lock().unwrap();
            cache.put(cache_key, results.clone());
        }

        self.circuit_breaker.record_success();
        Ok(results)
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
            page_id: row.page_id,
            page_name: row.page_name,
            content: row.content.clone(),
            snippet: Self::generate_snippet(&row.content, query, snippet_len),
            score: row.rank,
        }
    }

    /// Clear the search cache
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }
}

/// Extension: synchronous search for CLI use
///
/// Uses `tokio::runtime::Handle::current().block_on()` to bridge sync/async.
impl SearchService {
    #[instrument(skip(self))]
    pub fn blocking_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|_| SearchError::DatabaseError("No Tokio runtime active".to_string()))?;
        handle.block_on(self.search(query, limit))
    }

    pub fn blocking_fuzzy_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let handle = tokio::runtime::Handle::try_current()
            .map_err(|_| SearchError::DatabaseError("No Tokio runtime active".to_string()))?;
        handle.block_on(self.fuzzy_search(query, limit))
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
            page_id: "def".to_string(),
            page_name: "Test Page".to_string(),
            content: "This is test content".to_string(),
            rank: -1.5,
        };
        let result = SearchService::row_to_result(row, "test", 50);
        assert_eq!(result.block_id, "abc");
        assert_eq!(result.page_id, "def");
        assert_eq!(result.page_name, "Test Page");
        assert_eq!(result.score, -1.5);
    }

    #[tokio::test]
    async fn test_search_finds_results() {
        let pool = setup_test_db().await;
        let service = SearchService::new(pool);

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
        let service = SearchService::new(pool);

        let results = service.search("nonexistent", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_fuzzy_search_prefix() {
        let pool = setup_test_db().await;
        let service = SearchService::new(pool);

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
        let service = SearchService::new(pool);

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
        let service = SearchService::new(pool);

        let first = service.search("rust", 10).await.unwrap();
        let second = service.search("rust", 10).await.unwrap();
        assert_eq!(first.len(), second.len());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let pool = setup_test_db().await;
        let service = SearchService::new(pool);

        let _ = service.search("rust", 10).await.unwrap();
        service.clear_cache();
        // Should still work after cache clear
        let results = service.search("rust", 10).await.unwrap();
        assert!(results.len() >= 2);
    }
}
