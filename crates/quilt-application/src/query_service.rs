//! Query Service - High-level query preparation and execution
//!
//! Orchestrates the query parser, SQL generation, and execution against SQLite.
//!
//! This module provides a high-level interface for preparing and executing queries
//! using a DSL (Domain Specific Language) that gets translated to SQL.

use quilt_domain::entities::Block;
use quilt_domain::content::BlockContent;
use quilt_domain::value_objects::{BlockFormat, Priority, PropertyValue, TaskMarker, Uuid};
use quilt_query::executor::SqlParam;
use quilt_query::{QueryExecutor, QueryParser};
use sqlx::Row;
use std::collections::HashMap;

/// The result of preparing a query (plan only, no execution).
///
/// Contains the generated SQL statement, parameter values for safe
/// substitution, and the parsed AST for debugging purposes.
#[derive(Debug)]
pub struct QueryResult {
    /// The generated SQL statement with `?` placeholders
    pub sql: String,
    /// Parameter values to be bound to the SQL statement
    pub params: Vec<String>,
    /// The parsed Abstract Syntax Tree representation
    pub ast: String,
}

/// The result of executing a query against the database.
#[derive(Debug, Clone)]
pub struct ExecutedQueryResult {
    /// The matching blocks
    pub blocks: Vec<Block>,
    /// Number of results returned
    pub count: usize,
    /// The generated SQL (for debugging)
    pub sql: String,
}

/// Query service that combines parsing, SQL generation, and execution.
///
/// This service takes a query string in the Quilt DSL format,
/// parses it into an AST, generates safe SQL, and optionally
/// executes it against a SQLite database.
///
/// # Example
///
/// ```ignore
/// use quilt_application::query_service::QueryService;
///
/// let service = QueryService::new();
///
/// // Plan only (no DB needed)
/// let result = service.prepare("(task todo)", 100).unwrap();
///
/// // Execute against DB
/// let result = service.execute("(task todo)", 100, &pool).await.unwrap();
/// ```
///
/// # Supported Query Syntax
///
/// - `(task todo done)` - Filter by task markers
/// - `(priority a b c)` - Filter by priority levels
/// - `(page "Name")` - Filter by page reference
/// - `(property "key" "value")` - Filter by JSON property (eq)
/// - `(property "key" != "value")` - Not equals
/// - `(property "key" > "10")` - Greater than
/// - `(property "key" < "100")` - Less than
/// - `(property "key" contains "text")` - Contains
/// - `(property "key" between "1" "10")` - Range on property
/// - `(and ...)` / `(or ...)` / `(not ...)` - Boolean logic
/// - `[[Page Name]]` - Page reference
/// - `(full-text-search "keyword")` - FTS search
/// - `(tags "tag")` - Filter by tags
/// - `(between "1000" "2000")` - Numeric/date range filter
pub struct QueryService {
    parser: QueryParser,
    executor: QueryExecutor,
}

impl QueryService {
    /// Creates a new `QueryService` with default parser and executor.
    pub fn new() -> Self {
        Self {
            parser: QueryParser,
            executor: QueryExecutor::new(),
        }
    }

    /// Parses a query DSL string and generates SQL with parameters (plan only).
    pub fn prepare(&self, dsl: &str, limit: usize) -> Result<QueryResult, String> {
        let ast = self
            .parser
            .parse(dsl)
            .map_err(|e| format!("Parse error: {}", e))?;

        let (sql, params) = self.executor.build_sql(&ast, limit);

        Ok(QueryResult {
            sql,
            params: params.iter().map(|p| p.as_string()).collect(),
            ast: format!("{:?}", ast),
        })
    }

    /// Parses, generates SQL, and executes the query against a SQLite database.
    ///
    /// Returns the matching blocks as domain entities.
    ///
    /// # Arguments
    ///
    /// * `dsl` - The query string in Quilt DSL format
    /// * `limit` - Maximum number of results to return
    /// * `pool` - The SQLite connection pool
    ///
    /// # Returns
    ///
    /// Returns [`ExecutedQueryResult`] with the matching blocks.
    pub async fn execute(
        &self,
        dsl: &str,
        limit: usize,
        pool: &sqlx::SqlitePool,
    ) -> Result<ExecutedQueryResult, String> {
        let ast = self
            .parser
            .parse(dsl)
            .map_err(|e| format!("Parse error: {}", e))?;

        let (sql, params) = self.executor.build_sql(&ast, limit);

        // Build the query with bound parameters
        let mut query = sqlx::query(&sql);
        for param in &params {
            query = match param {
                SqlParam::String(s) => query.bind(s),
                SqlParam::Integer(n) => query.bind(n),
                SqlParam::Float(f) => query.bind(f),
                SqlParam::Boolean(b) => query.bind(b),
            };
        }

        let rows = query
            .fetch_all(pool)
            .await
            .map_err(|e| format!("Query execution failed: {}", e))?;

        let blocks: Vec<Block> = rows
            .iter()
            .filter_map(|row| row_to_block(row).ok())
            .collect();

        let count = blocks.len();

        Ok(ExecutedQueryResult { blocks, count, sql })
    }
}

impl Default for QueryService {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a SQLite row to a Block entity.
fn row_to_block(row: &sqlx::sqlite::SqliteRow) -> Result<Block, String> {
    use chrono::{DateTime, TimeZone, Utc};

    let id_blob: Vec<u8> = row.get("id");
    let page_id_blob: Vec<u8> = row.get("page_id");
    let parent_id_blob: Option<Vec<u8>> = row.get("parent_id");

    let id = blob_to_uuid(&id_blob)?;
    let page_id = blob_to_uuid(&page_id_blob)?;
    let parent_id = parent_id_blob
        .as_ref()
        .filter(|b| !b.is_empty())
        .map(|b| blob_to_uuid(b))
        .transpose()?;

    let order: f64 = row.try_get("order_index").unwrap_or(0.0);
    let level: i64 = row.try_get("level").unwrap_or(1);
    let format_str: String = row
        .try_get("format")
        .unwrap_or_else(|_| "markdown".to_string());
    let marker_str: Option<String> = row.try_get("marker").ok();
    let priority_str: Option<String> = row.try_get("priority").ok();
    let content: String = row.try_get("content").unwrap_or_default();
    let properties_blob: Vec<u8> = row.try_get("properties").unwrap_or_default();
    let collapsed: i64 = row.try_get("collapsed").unwrap_or(0);
    let created_at_ts: i64 = row.try_get("created_at").unwrap_or(0);
    let updated_at_ts: i64 = row.try_get("updated_at").unwrap_or(0);

    let scheduled: Option<i64> = row.try_get("scheduled").ok().flatten();
    let deadline: Option<i64> = row.try_get("deadline").ok().flatten();
    let start_time: Option<i64> = row.try_get("start_time").ok().flatten();
    let repeated: Option<i64> = row.try_get("repeated").ok().flatten();
    let logbook: Option<i64> = row.try_get("logbook").ok().flatten();

    let refs_blob: Vec<u8> = row.try_get("refs").unwrap_or_default();
    let tags_blob: Vec<u8> = row.try_get("tags").unwrap_or_default();

    let ts_to_dt =
        |ts: i64| -> DateTime<Utc> { Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now) };
    let opt_ts_to_dt = |ts: Option<i64>| -> Option<DateTime<Utc>> { ts.map(ts_to_dt) };

    let format = match format_str.as_str() {
        "org" => BlockFormat::Org,
        _ => BlockFormat::Markdown,
    };

    let marker = marker_str.and_then(|s| match s.as_str() {
        "now" => Some(TaskMarker::Now),
        "later" => Some(TaskMarker::Later),
        "todo" => Some(TaskMarker::Todo),
        "done" => Some(TaskMarker::Done),
        "cancelled" => Some(TaskMarker::Cancelled),
        _ => None,
    });

    let priority = priority_str.and_then(|s| match s.to_lowercase().as_str() {
        "a" => Some(Priority::A),
        "b" => Some(Priority::B),
        "c" => Some(Priority::C),
        _ => None,
    });

    let properties: HashMap<String, quilt_domain::value_objects::PropertyValue> =
        if properties_blob.is_empty() || properties_blob == b"{}" {
            HashMap::new()
        } else {
            serde_json::from_slice::<HashMap<String, serde_json::Value>>(&properties_blob)
                .ok()
                .map(|map| {
                    map.into_iter()
                        .filter_map(|(k, v)| PropertyValue::from_json(&v).map(|pv| (k, pv)))
                        .collect()
                })
                .unwrap_or_default()
        };

    let refs: Vec<Uuid> = if refs_blob.is_empty() || refs_blob == b"[]" {
        Vec::new()
    } else {
        serde_json::from_slice::<Vec<String>>(&refs_blob)
            .ok()
            .map(|v| v.iter().filter_map(|s| Uuid::parse_str(s)).collect())
            .unwrap_or_default()
    };

    let tags: Vec<String> = if tags_blob.is_empty() || tags_blob == b"[]" {
        Vec::new()
    } else {
        serde_json::from_slice::<Vec<String>>(&tags_blob).unwrap_or_default()
    };

    Ok(Block {
        id,
        page_id,
        parent_id,
        order,
        level: level as u8,
        format,
        marker,
        priority,
        // Parse content - try JSON (new format) first, then fall back to plain text (legacy)
        content: if let Ok(c) = serde_json::from_str::<BlockContent>(&content) {
            c
        } else {
            BlockContent::from_text(content)
        },
        properties,
        refs,
        tags,
        scheduled: opt_ts_to_dt(scheduled),
        deadline: opt_ts_to_dt(deadline),
        start_time: opt_ts_to_dt(start_time),
        repeated: opt_ts_to_dt(repeated),
        logbook: opt_ts_to_dt(logbook),
        collapsed: collapsed != 0,
        created_at: ts_to_dt(created_at_ts),
        updated_at: ts_to_dt(updated_at_ts),
        journal_day: None,
        updated_journal_day: None,
    })
}

fn blob_to_uuid(blob: &[u8]) -> Result<Uuid, String> {
    let bytes: [u8; 16] = blob
        .try_into()
        .map_err(|_| format!("Invalid UUID blob length: {}", blob.len()))?;
    Ok(Uuid::from_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::entities::{BlockCreate, PageCreate};
    use quilt_domain::repositories::{BlockRepository, PageRepository};
    use quilt_domain::TimezoneService;
    use quilt_infrastructure::database::sqlite::connection;
    use quilt_infrastructure::database::sqlite::repositories::{
        SqliteBlockRepository, SqlitePageRepository,
    };
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        connection::run_migrations(&pool).await.unwrap();
        pool
    }

    fn make_page(name: &str) -> quilt_domain::entities::Page {
        quilt_domain::entities::Page::new(PageCreate {
            name: name.to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        })
        .unwrap()
    }

    // ── Plan tests (no DB) ────────────────────────────────────────

    #[test]
    fn test_prepare_simple_query() {
        let service = QueryService::new();
        let result = service.prepare("(task todo)", 100).unwrap();
        assert!(result.sql.contains("marker IN"));
        assert!(result.ast.contains("Task"));
    }

    #[test]
    fn test_prepare_complex_query() {
        let service = QueryService::new();
        let result = service
            .prepare("(and (task todo) (priority a))", 50)
            .unwrap();
        assert!(result.sql.contains("AND"));
        assert_eq!(result.params.len(), 2);
    }

    #[test]
    fn test_prepare_property_query() {
        let service = QueryService::new();
        let result = service
            .prepare("(property \"author\" \"John\")", 100)
            .unwrap();
        assert!(result.sql.contains("json_extract"));
    }

    #[test]
    fn test_prepare_invalid_query() {
        let service = QueryService::new();
        assert!(service.prepare("", 100).is_err());
    }

    // ── Execution tests (with DB) ─────────────────────────────────

    #[tokio::test]
    async fn test_execute_task_query() {
        let pool = setup_test_db().await;
        let service = QueryService::new();
        let tz = TimezoneService::from_tz_string("UTC").unwrap();

        // Create a page and blocks with markers
        let page = make_page("test-tasks");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let mut b1 = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Task one"),
                marker: Some(TaskMarker::Todo),
                ..Default::default()
            },
            &tz,
        )
        .unwrap();
        let b2 = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Task two"),
                marker: Some(TaskMarker::Done),
                ..Default::default()
            },
            &tz,
        )
        .unwrap();
        let b3 = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Not a task"),
                marker: None,
                ..Default::default()
            },
            &tz,
        )
        .unwrap();

        let block_repo = SqliteBlockRepository::new(pool.clone());
        block_repo.insert(&b1).await.unwrap();
        block_repo.insert(&b2).await.unwrap();
        block_repo.insert(&b3).await.unwrap();

        // Execute task query
        let result = service.execute("(task todo)", 100, &pool).await.unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(result.blocks[0].content.as_plain_text(), "Task one");
    }

    #[tokio::test]
    async fn test_execute_page_query() {
        let pool = setup_test_db().await;
        let service = QueryService::new();
        let tz = TimezoneService::from_tz_string("UTC").unwrap();

        let page = make_page("target-page");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        let block = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Block on target page"),
                ..Default::default()
            },
            &tz,
        )
        .unwrap();
        SqliteBlockRepository::new(pool.clone())
            .insert(&block)
            .await
            .unwrap();

        let result = service
            .execute("(page \"target-page\")", 100, &pool)
            .await
            .unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(result.blocks[0].content.as_plain_text(), "Block on target page");
    }

    #[tokio::test]
    async fn test_execute_empty_result() {
        let pool = setup_test_db().await;
        let service = QueryService::new();

        let result = service.execute("(task todo)", 100, &pool).await.unwrap();
        assert_eq!(result.count, 0);
        assert!(result.blocks.is_empty());
    }

    #[tokio::test]
    async fn test_execute_and_query() {
        let pool = setup_test_db().await;
        let service = QueryService::new();
        let tz = TimezoneService::from_tz_string("UTC").unwrap();

        let page = make_page("and-test");
        SqlitePageRepository::new(pool.clone())
            .insert(&page)
            .await
            .unwrap();

        // Create block with both marker and priority
        let mut b1 = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Priority A task"),
                marker: Some(TaskMarker::Todo),
                ..Default::default()
            },
            &tz,
        )
        .unwrap();
        b1.priority = Some(Priority::A);

        let mut b2 = Block::new(
            BlockCreate {
                page_id: page.id,
                content: BlockContent::from_text("Priority B task"),
                marker: Some(TaskMarker::Todo),
                ..Default::default()
            },
            &tz,
        )
        .unwrap();
        b2.priority = Some(Priority::B);

        let repo = SqliteBlockRepository::new(pool.clone());
        repo.insert(&b1).await.unwrap();
        repo.update(&b1).await.unwrap();
        repo.insert(&b2).await.unwrap();
        repo.update(&b2).await.unwrap();

        // First verify task query works
        let task_result = service.execute("(task todo)", 100, &pool).await.unwrap();
        assert_eq!(task_result.count, 2, "Should find 2 todo tasks");

        // Then verify priority query works
        let prio_result = service.execute("(priority a)", 100, &pool).await.unwrap();
        assert_eq!(prio_result.count, 1, "Should find 1 priority A block");

        // Now AND query
        let result = service
            .execute("(and (task todo) (priority a))", 100, &pool)
            .await
            .unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(result.blocks[0].content.as_plain_text(), "Priority A task");
    }
}
