//! Query executor - generates SQL from AST
//!
//! This module converts [`QueryExpr`] AST nodes into SQL WHERE clauses
//! with properly parameterized values for safe database queries.

use crate::compiler::CompilerError;
use crate::dialect::{SqlDialect, SqliteDialect};
use crate::parser::{PropertyOp, QueryExpr, QueryValue};

/// SQL parameter type for safe query parameterization.
#[derive(Debug, Clone, PartialEq)]
pub enum SqlParam {
    /// String parameter
    String(String),
    /// Integer parameter
    Integer(i64),
    /// Float parameter
    Float(f64),
    /// Boolean parameter
    Boolean(bool),
}

impl SqlParam {
    /// Converts the parameter to its string representation.
    pub fn as_string(&self) -> String {
        match self {
            SqlParam::String(s) => s.clone(),
            SqlParam::Integer(n) => n.to_string(),
            SqlParam::Float(f) => f.to_string(),
            SqlParam::Boolean(b) => b.to_string(),
        }
    }
}

/// Result of an analyze operation.
/// Uses JSON values since analysis types are owned by quilt-analysis.
#[derive(Debug, Clone)]
pub enum AnalyzeResult {
    /// Structural mirror analysis result (JSON serialized StructureMap)
    StructureMap(serde_json::Value),
    /// Serendipity connections result (JSON serialized connections)
    SerendipityConnections(serde_json::Value),
}

/// Errors that can occur during analyze execution.
#[derive(Debug, thiserror::Error)]
pub enum AnalyzeError {
    #[error("Analysis engine not configured: {0}")]
    EngineNotConfigured(String),
    #[error("Analysis execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Block repository error: {0}")]
    Repository(String),
}

/// Query executor that converts AST to SQL.
///
/// This executor transforms [`QueryExpr`] AST nodes into SQL WHERE clauses
/// with parameterized values for safe database queries.
///
/// # Example
///
/// ```
/// use quilt_query::{QueryParser, QueryExecutor};
///
/// let parser = QueryParser;
/// let executor = QueryExecutor::new();
///
/// let expr = parser.parse("(task todo)").unwrap();
/// let (sql, params) = executor.build_sql(&expr, 100).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct QueryExecutor<D = SqliteDialect>
where
    D: SqlDialect,
{
    /// Whether to include ORDER BY RANDOM() for SAMPLE
    pub sample_limit: Option<usize>,
    dialect: D,
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryExecutor {
    /// Creates a new `QueryExecutor` with default settings.
    pub fn new() -> Self {
        Self {
            sample_limit: None,
            dialect: SqliteDialect,
        }
    }
}

impl<D> QueryExecutor<D>
where
    D: SqlDialect,
{
    /// Creates a new `QueryExecutor` with a custom dialect.
    pub fn with_dialect(dialect: D) -> Self {
        Self {
            sample_limit: None,
            dialect,
        }
    }

    /// Builds a SQL WHERE clause from a query expression.
    ///
    /// Returns `Err(CompilerError::UnsupportedOperator { op })` for `Stats`
    /// and `Analyze` variants — these are not WHERE-able operators and
    /// must be compiled via `build_sql` / `build_analyze_sql` instead.
    /// This replaces the two historical `panic!` calls (F1).
    ///
    /// # Arguments
    ///
    /// * `expr` - The parsed query expression AST
    ///
    /// # Returns
    ///
    /// `Ok((sql, params))` on success, `Err(CompilerError)` if the
    /// expression contains a top-level operator that `build_where`
    /// cannot compile (e.g., `Stats`, `Analyze`).
    pub fn build_where(&self, expr: &QueryExpr) -> Result<(String, Vec<SqlParam>), CompilerError> {
        match expr {
            QueryExpr::And(exprs) => {
                let mut sqls = Vec::with_capacity(exprs.len());
                let mut params = Vec::new();
                for e in exprs {
                    let (s, mut p) = self.build_where(e)?;
                    sqls.push(format!("({})", s));
                    params.append(&mut p);
                }
                Ok((sqls.join(" AND "), params))
            }

            QueryExpr::Or(exprs) => {
                let mut sqls = Vec::with_capacity(exprs.len());
                let mut params = Vec::new();
                for e in exprs {
                    let (s, mut p) = self.build_where(e)?;
                    sqls.push(format!("({})", s));
                    params.append(&mut p);
                }
                Ok((sqls.join(" OR "), params))
            }

            QueryExpr::Not(inner) => {
                let (sql, params) = self.build_where(inner)?;
                Ok((format!("NOT ({})", sql), params))
            }

            QueryExpr::Task(markers) => {
                let placeholders: Vec<_> = markers.iter().map(|_| "?".to_string()).collect();
                let sql = format!("marker IN ({})", placeholders.join(", "));
                let params: Vec<_> = markers
                    .iter()
                    .map(|m| SqlParam::String(m.to_lowercase()))
                    .collect();
                Ok((sql, params))
            }

            QueryExpr::Priority(priorities) => {
                let placeholders: Vec<_> = priorities.iter().map(|_| "?".to_string()).collect();
                let sql = format!("priority IN ({})", placeholders.join(", "));
                let params: Vec<_> = priorities
                    .iter()
                    .map(|p| SqlParam::String(p.to_lowercase()))
                    .collect();
                Ok((sql, params))
            }

            QueryExpr::Page(name) => {
                // Use a correlated subquery with proper alias reference
                let sql = "EXISTS (SELECT 1 FROM pages p WHERE p.id = b.page_id AND p.name = ?)"
                    .to_string();
                Ok((sql, vec![SqlParam::String(name.to_lowercase())]))
            }

            QueryExpr::BlockContent(query) => {
                // Use IN pattern for FTS5 search without alias
                let sql =
                    "b.rowid IN (SELECT rowid FROM blocks_fts WHERE content MATCH ?)".to_string();
                Ok((sql, vec![SqlParam::String(query.clone())]))
            }

            QueryExpr::PageRef(name) => {
                let sql = "content LIKE ?".to_string();
                let param = format!("%[[{}]]%", name);
                Ok((sql, vec![SqlParam::String(param)]))
            }

            QueryExpr::SelfRef => Ok(("1 = 1".to_string(), vec![])),

            QueryExpr::Sample(_n) => {
                // Mark that we want random ordering
                Ok((String::new(), vec![]))
            }

            QueryExpr::Between { field, start, end } => {
                let start_val = self.value_to_param(start);
                let end_val = self.value_to_param(end);
                // Use b. prefix for fields that might conflict with pages table
                let qualified_field = if field == "created_at" {
                    "b.created_at".to_string()
                } else {
                    field.clone()
                };
                let sql = format!("{} BETWEEN ? AND ?", qualified_field);
                Ok((sql, vec![start_val, end_val]))
            }

            QueryExpr::Property {
                key,
                op,
                value,
                value2,
            } => {
                // F3 — use the dialect's `property_op_sql` for the
                // operator-specific fragment. `Contains` is bound with
                // `LIKE`; we wrap the value as `%v%` here.
                let prop_path = self.dialect.property_path(key);
                let sql_fragment = self.dialect.property_op_sql(*op, &prop_path);
                let val = self.value_to_param(value);
                let bound_value = match op {
                    crate::parser::PropertyOp::Contains => {
                        SqlParam::String(format!("%{}%", val.as_string()))
                    }
                    _ => val,
                };
                let mut params = vec![bound_value];
                if matches!(op, crate::parser::PropertyOp::Between) {
                    let v2 = value2.as_ref().ok_or_else(|| {
                        CompilerError::Invalid("PropertyOp::Between requires value2".to_string())
                    })?;
                    params.push(self.value_to_param(v2));
                }
                Ok((sql_fragment, params))
            }

            QueryExpr::Tags(tag) => {
                let sql = "tags LIKE ?".to_string();
                let param = format!("%\"{} \"%", tag);
                Ok((sql, vec![SqlParam::String(param)]))
            }

            QueryExpr::Aggregate {
                inner, group_by, ..
            } => {
                let (inner_where, params) = self.build_where(inner)?;
                let prop_path = self.dialect.property_path(group_by);
                let null_check = format!("{} IS NOT NULL", prop_path);
                let where_clause = if inner_where.is_empty() {
                    null_check.clone()
                } else {
                    format!("{} AND {}", inner_where, null_check)
                };
                Ok((
                    format!("{} AND {} GROUP BY {}", where_clause, prop_path, prop_path),
                    params,
                ))
            }

            // F1 — Stats is handled in `build_sql`, not `build_where`.
            // Returning `Err` instead of `panic!` makes the executor
            // panic-free in the runtime paths.
            QueryExpr::Stats { .. } => Err(CompilerError::UnsupportedOperator { op: "Stats" }),

            QueryExpr::GroupBy { inner, property } => {
                let (inner_where, params) = self.build_where(inner)?;
                let prop_path = self.dialect.property_path(property);
                let null_check = format!("{} IS NOT NULL", prop_path);
                let where_clause = if inner_where.is_empty() {
                    null_check.clone()
                } else {
                    format!("{} AND {}", inner_where, null_check)
                };
                Ok((
                    format!("{} AND {} GROUP BY {}", where_clause, prop_path, prop_path),
                    params,
                ))
            }

            // F1 — Analyze is handled in `build_analyze_sql`, not
            // `build_where`. Returning `Err` instead of `panic!` makes
            // the executor panic-free in the runtime paths.
            QueryExpr::Analyze { .. } => Err(CompilerError::UnsupportedOperator { op: "Analyze" }),

            // F2 — `Table` is a passthrough (the table layout is a
            // presentation concern, not a SQL filter).
            QueryExpr::Table(_) => Ok((String::new(), vec![])),

            // F2 — `SortBy` is a passthrough at the WHERE level; the
            // sort direction is applied by the caller in `build_sql`.
            QueryExpr::SortBy { inner, .. } => self.build_where(inner),

            // F2 — `Exists(key)` — property is present.
            QueryExpr::Exists(key) => {
                let sql = format!("{} IS NOT NULL", self.dialect.property_path(&key));
                Ok((sql, vec![]))
            }

            // F2 — `Missing(key)` — property is absent.
            QueryExpr::Missing(key) => {
                let sql = format!("{} IS NULL", self.dialect.property_path(&key));
                Ok((sql, vec![]))
            }

            // F2 — `Namespace(ns)` — filter on the page namespace.
            QueryExpr::Namespace(ns) => {
                let sql =
                    "EXISTS (SELECT 1 FROM pages p WHERE p.id = b.page_id AND p.namespace_id = ?)"
                        .to_string();
                Ok((sql, vec![SqlParam::String(ns.clone())]))
            }

            // G5 — PageFuzzy is handled in compile_page_fuzzy hook, not build_where.
            QueryExpr::PageFuzzy { .. } => {
                Err(CompilerError::UnsupportedOperator { op: "PageFuzzy" })
            }

            // G3 — Temporal is handled in compile_temporal hook, not build_where.
            QueryExpr::Temporal { .. } => {
                Err(CompilerError::UnsupportedOperator { op: "Temporal" })
            }

            // F12 — VirtualSelect is handled in compile_virtual_select hook, not build_where.
            QueryExpr::VirtualSelect { .. } => Err(CompilerError::UnsupportedOperator {
                op: "VirtualSelect",
            }),
        }
    }

    /// Builds a full SQL query from a query expression.
    ///
    /// Generates a complete SQL SELECT statement with proper JOINs,
    /// WHERE clause, and LIMIT.
    ///
    /// # Arguments
    ///
    /// * `expr` - The parsed query expression AST
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - The complete SQL SELECT statement
    /// - The parameter values to be bound to the query
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_query::{QueryParser, QueryExecutor};
    ///
    /// let parser = QueryParser;
    /// let executor = QueryExecutor::new();
    ///
    /// let expr = parser.parse("(task todo)").unwrap();
    /// let (sql, params) = executor.build_sql(&expr, 100).unwrap();
    ///
    /// assert!(sql.contains("SELECT"));
    /// assert!(sql.contains("WHERE"));
    /// assert!(sql.contains("LIMIT 100"));
    /// ```
    pub fn build_sql(
        &self,
        expr: &QueryExpr,
        limit: usize,
    ) -> Result<(String, Vec<SqlParam>), CompilerError> {
        // Handle aggregate variants with special SQL generation
        match expr {
            QueryExpr::Aggregate {
                inner,
                group_by,
                aggregate_fn,
            } => {
                let (inner_where, params) = self.build_where(inner)?;
                let prop_path = self.dialect.property_path(group_by);
                let fn_sql = self.dialect.aggregate_fn(aggregate_fn.clone(), &prop_path);
                let null_check = format!("{} IS NOT NULL", prop_path);
                let where_clause = if inner_where.is_empty() {
                    null_check.clone()
                } else {
                    format!("{} AND {}", inner_where, null_check)
                };
                let sql = format!(
                    "SELECT {}, {} \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE {} \
                     GROUP BY {}",
                    prop_path, fn_sql, where_clause, prop_path
                );
                Ok((sql, params))
            }

            QueryExpr::Stats { property, compute } => {
                let prop_path = self.dialect.property_path(property);
                let fn_sql = self.dialect.stats_fn(compute.clone(), &prop_path);
                let sql = format!(
                    "SELECT {} \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE {} IS NOT NULL",
                    fn_sql, prop_path
                );
                Ok((sql, vec![]))
            }

            QueryExpr::GroupBy { inner, property } => {
                let (inner_where, params) = self.build_where(inner)?;
                let prop_path = self.dialect.property_path(property);
                let null_check = format!("{} IS NOT NULL", prop_path);
                let where_clause = if inner_where.is_empty() {
                    null_check.clone()
                } else {
                    format!("{} AND {}", inner_where, null_check)
                };
                let sql = format!(
                    "SELECT DISTINCT {} \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE {}",
                    prop_path, where_clause
                );
                Ok((sql, params))
            }

            _ => {
                let (where_clause, params) = self.build_where(expr)?;

                let mut sql = String::from(
                    "SELECT b.*, p.name as page_name \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE ",
                );

                sql.push_str(&where_clause);

                // Handle SAMPLE
                if matches!(expr, QueryExpr::Sample(_)) {
                    sql.push_str(" ORDER BY RANDOM()");
                }

                sql.push_str(&format!(" LIMIT {}", limit));

                Ok((sql, params))
            }
        }
    }

    /// Convert QueryValue to SqlParam
    fn value_to_param(&self, value: &QueryValue) -> SqlParam {
        match value {
            QueryValue::String(s) => SqlParam::String(s.clone()),
            QueryValue::Integer(n) => SqlParam::Integer(*n),
            QueryValue::Date(d) => SqlParam::String(d.clone()),
            QueryValue::TimeOffset(t) => SqlParam::String(t.clone()),
            QueryValue::Boolean(b) => SqlParam::Boolean(*b),
        }
    }

    /// Builds a SQL query for analyze operations.
    ///
    /// Extracts the inner expression, builds the WHERE clause from it,
    /// and returns a SQL query that selects blocks for analysis.
    pub fn build_analyze_sql(
        &self,
        expr: &QueryExpr,
    ) -> Result<(String, Vec<SqlParam>), CompilerError> {
        match expr {
            QueryExpr::Analyze { inner, .. } => {
                let (where_clause, params) = self.build_where(inner)?;
                let sql = if where_clause.is_empty() {
                    format!(
                        "SELECT b.* FROM blocks b JOIN pages p ON b.page_id = p.id LIMIT {}",
                        1000 // hard cap for analyze
                    )
                } else {
                    format!(
                        "SELECT b.*, p.name as page_name \
                         FROM blocks b \
                         JOIN pages p ON b.page_id = p.id \
                         WHERE {} \
                         LIMIT {}",
                        where_clause, 1000
                    )
                };
                Ok((sql, params))
            }
            _ => Err(CompilerError::Invalid(
                "Expected Analyze expression".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::QueryAst;

    #[test]
    fn test_simple_task_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Task(vec!["todo".to_string()]);
        let (sql, params) = executor.build_sql(&expr, 100).unwrap();

        assert!(sql.contains("marker IN"));
        assert!(sql.contains("LIMIT 100"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_page_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Page("Test Page".to_string());
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("EXISTS"));
        assert!(sql.contains("pages"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "test page"); // lowercase
    }

    #[test]
    fn test_and_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::And(vec![
            QueryExpr::Task(vec!["todo".to_string()]),
            QueryExpr::Priority(vec!["a".to_string()]),
        ]);
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_between_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Between {
            field: "created_at".to_string(),
            start: QueryValue::Integer(1000),
            end: QueryValue::Integer(2000),
        };
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("BETWEEN ? AND ?"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_or_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Or(vec![
            QueryExpr::Task(vec!["todo".to_string()]),
            QueryExpr::Task(vec!["done".to_string()]),
        ]);
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains(" OR "));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_not_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Not(Box::new(QueryExpr::Task(vec!["done".to_string()])));
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("NOT"));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_page_ref_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::PageRef("TestPage".to_string());
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("content LIKE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "%[[TestPage]]%");
    }

    #[test]
    fn test_block_content_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::BlockContent("hello world".to_string());
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("blocks_fts"));
        assert!(sql.contains("MATCH"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "hello world");
    }

    #[test]
    fn test_tags_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Tags("important".to_string());
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("tags LIKE"));
        assert_eq!(params.len(), 1);
        // The format is %"{tag} "% so it becomes %"important "%
        assert_eq!(params[0].as_string(), "%\"important \"%");
    }

    #[test]
    fn test_self_ref_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::SelfRef;
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert_eq!(sql, "1 = 1");
        assert!(params.is_empty());
    }

    #[test]
    fn test_sample_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Sample(5);
        let (where_sql, params) = executor.build_where(&expr).unwrap();

        // Sample returns empty WHERE clause
        assert!(where_sql.is_empty());
        assert!(params.is_empty());
    }

    #[test]
    fn test_sample_query_full_sql() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Sample(10);
        let (sql, params) = executor.build_sql(&expr, 10).unwrap();

        assert!(sql.contains("ORDER BY RANDOM()"));
        assert!(sql.contains("LIMIT 10"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_property_query() {
        let executor = QueryExecutor::new();
        let expr = QueryAst::Property {
            key: "author".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::String("John".to_string()),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("$.author"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_integer() {
        let executor = QueryExecutor::new();
        let expr = QueryAst::Property {
            key: "count".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::Integer(42),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("json_extract"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "42");
    }

    #[test]
    fn test_property_query_boolean() {
        let executor = QueryExecutor::new();
        let expr = QueryAst::Property {
            key: "active".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::Boolean(true),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("json_extract"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "true");
    }

    #[test]
    fn test_complex_and_or_not() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::And(vec![
            QueryExpr::Not(Box::new(QueryExpr::Task(vec!["done".to_string()]))),
            QueryExpr::Priority(vec!["a".to_string()]),
        ]);
        let (sql, params) = executor.build_where(&expr).unwrap();

        assert!(sql.contains("NOT"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("priority IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_build_sql_has_right_structure() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Task(vec!["todo".to_string()]);
        let (sql, params) = executor.build_sql(&expr, 100).unwrap();

        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("JOIN pages p"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("LIMIT 100"));
        assert!(!params.is_empty());
    }

    #[test]
    fn test_params_are_ordered() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::And(vec![
            QueryExpr::Task(vec!["todo".to_string()]),
            QueryExpr::Priority(vec!["a".to_string()]),
            QueryExpr::Page("Test".to_string()),
        ]);
        let (_, params) = executor.build_where(&expr).unwrap();

        // 1 param for task + 1 for priority + 1 for page
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_page_name_lowercased() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Page("MyPage".to_string());
        let (_, params) = executor.build_where(&expr).unwrap();

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "mypage");
    }

    #[test]
    fn test_priority_lowercased() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Priority(vec!["A".to_string(), "B".to_string()]);
        let (_, params) = executor.build_where(&expr).unwrap();

        assert_eq!(params.len(), 2);
        assert_eq!(params[0].as_string(), "a");
        assert_eq!(params[1].as_string(), "b");
    }

    #[test]
    fn test_between_query_full_sql() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Between {
            field: "created_at".to_string(),
            start: QueryValue::Integer(1000),
            end: QueryValue::Integer(2000),
        };
        let (sql, params) = executor.build_sql(&expr, 50).unwrap();

        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("WHERE b.created_at BETWEEN ? AND ?"));
        assert!(sql.contains("LIMIT 50"));
        assert_eq!(params.len(), 2);
    }

    // Analyze tests

    #[test]
    fn test_build_analyze_sql_simple() {
        use crate::parser::AnalyzeKind;
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Analyze {
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
            kind: AnalyzeKind::StructuralMirror,
        };
        let (sql, _params) = executor.build_analyze_sql(&expr).unwrap();
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("JOIN pages p"));
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("LIMIT 1000"));
    }

    #[test]
    fn test_build_analyze_sql_page_filter() {
        use crate::parser::AnalyzeKind;
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Analyze {
            inner: Box::new(QueryExpr::Page("Test".to_string())),
            kind: AnalyzeKind::Serendipity {
                limit: None,
                min_confidence: None,
                temporal_window_days: None,
            },
        };
        let (sql, _params) = executor.build_analyze_sql(&expr).unwrap();
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("EXISTS"));
    }

    #[test]
    fn test_build_analyze_sql_non_analyze_error() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Task(vec!["todo".to_string()]);
        let result = executor.build_analyze_sql(&expr);
        assert!(result.is_err());
    }

    // F1 — panic→Result conversion tests.
    //
    // Before F1, `build_where` panicked for `Stats` and `Analyze` variants.
    // After F1, it returns `Err(CompilerError::UnsupportedOperator { op })`.

    #[test]
    fn test_build_where_stats_returns_unsupported_operator_error() {
        use crate::compiler::CompilerError;
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Stats {
            property: "count".to_string(),
            compute: crate::parser::StatsFn::Stddev,
        };
        let result = executor.build_where(&expr);
        match result {
            Err(CompilerError::UnsupportedOperator { op }) => {
                assert_eq!(op, "Stats");
            }
            other => panic!("expected UnsupportedOperator(Stats), got {:?}", other),
        }
    }

    #[test]
    fn test_build_where_analyze_returns_unsupported_operator_error() {
        use crate::compiler::CompilerError;
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Analyze {
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
            kind: crate::parser::AnalyzeKind::StructuralMirror,
        };
        let result = executor.build_where(&expr);
        match result {
            Err(CompilerError::UnsupportedOperator { op }) => {
                assert_eq!(op, "Analyze");
            }
            other => panic!("expected UnsupportedOperator(Analyze), got {:?}", other),
        }
    }
}
