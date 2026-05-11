//! Query executor - generates SQL from AST
//!
//! This module converts [`QueryExpr`] AST nodes into SQL WHERE clauses
//! with properly parameterized values for safe database queries.

use crate::parser::{PropertyOp, QueryExpr, QueryValue, SortDirection};

/// SQL parameter type for safe query parameterization.
#[derive(Debug, Clone)]
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
/// let (sql, params) = executor.build_sql(&expr, 100);
/// ```
#[derive(Debug, Clone)]
pub struct QueryExecutor {
    /// Whether to include ORDER BY RANDOM() for SAMPLE
    pub sample_limit: Option<usize>,
}

impl QueryExecutor {
    /// Creates a new `QueryExecutor` with default settings.
    pub fn new() -> Self {
        Self { sample_limit: None }
    }

    /// Builds a SQL WHERE clause from a query expression.
    ///
    /// # Arguments
    ///
    /// * `expr` - The parsed query expression AST
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// - The SQL WHERE clause string with `?` placeholders
    /// - The parameter values to be bound to the query
    pub fn build_where(&self, expr: &QueryExpr) -> (String, Vec<SqlParam>) {
        match expr {
            QueryExpr::And(exprs) => {
                let clauses: Vec<_> = exprs.iter().map(|e| self.build_where(e)).collect();
                let sqls: Vec<_> = clauses.iter().map(|(s, _)| format!("({})", s)).collect();
                let params: Vec<_> = clauses.iter().flat_map(|(_, p)| p.clone()).collect();
                (sqls.join(" AND "), params)
            }

            QueryExpr::Or(exprs) => {
                let clauses: Vec<_> = exprs.iter().map(|e| self.build_where(e)).collect();
                let sqls: Vec<_> = clauses.iter().map(|(s, _)| format!("({})", s)).collect();
                let params: Vec<_> = clauses.iter().flat_map(|(_, p)| p.clone()).collect();
                (sqls.join(" OR "), params)
            }

            QueryExpr::Not(inner) => {
                let (sql, params) = self.build_where(inner);
                (format!("NOT ({})", sql), params)
            }

            QueryExpr::Task(markers) => {
                let placeholders: Vec<_> = markers.iter().map(|_| "?".to_string()).collect();
                let sql = format!("marker IN ({})", placeholders.join(", "));
                let params: Vec<_> = markers
                    .iter()
                    .map(|m| SqlParam::String(m.to_lowercase()))
                    .collect();
                (sql, params)
            }

            QueryExpr::Priority(priorities) => {
                let placeholders: Vec<_> = priorities.iter().map(|_| "?".to_string()).collect();
                let sql = format!("priority COLLATE NOCASE IN ({})", placeholders.join(", "));
                let params: Vec<_> = priorities
                    .iter()
                    .map(|p| SqlParam::String(p.to_lowercase()))
                    .collect();
                (sql, params)
            }

            QueryExpr::Page(name) => {
                // Use a correlated subquery with proper alias reference
                let sql = "EXISTS (SELECT 1 FROM pages p WHERE p.id = b.page_id AND p.name = ?)"
                    .to_string();
                (sql, vec![SqlParam::String(name.to_lowercase())])
            }

            QueryExpr::BlockContent(query) => {
                // Use IN pattern for FTS5 search without alias
                let sql =
                    "b.rowid IN (SELECT rowid FROM blocks_fts WHERE content MATCH ?)".to_string();
                (sql, vec![SqlParam::String(query.clone())])
            }

            QueryExpr::PageRef(name) => {
                let sql = "content LIKE ?".to_string();
                let param = format!("%[[{}]]%", name);
                (sql, vec![SqlParam::String(param)])
            }

            QueryExpr::SelfRef => ("1 = 1".to_string(), vec![]),

            QueryExpr::Sample(_n) => {
                // Mark that we want random ordering
                (String::new(), vec![])
            }

            QueryExpr::Between { field, start, end } => {
                // Resolve time offsets to timestamps for date fields
                let date_fields = [
                    "created_at",
                    "updated_at",
                    "scheduled",
                    "deadline",
                    "start_time",
                ];
                let is_date_field = date_fields.contains(&field.as_str());

                let (start_val, end_val) = if is_date_field {
                    (
                        self.value_to_timestamp_param(start),
                        self.value_to_timestamp_param(end),
                    )
                } else {
                    (self.value_to_param(start), self.value_to_param(end))
                };

                // Use b. prefix for date fields to avoid ambiguity with pages table
                let qualified_field = if date_fields.contains(&field.as_str()) {
                    format!("b.{}", field)
                } else {
                    field.clone()
                };
                let sql = format!("{} BETWEEN ? AND ?", qualified_field);
                (sql, vec![start_val, end_val])
            }

            QueryExpr::Property {
                key,
                op,
                value,
                value2,
            } => {
                let json_path = format!("$.{}", key);
                let val = self.value_to_param(value);

                match op {
                    PropertyOp::Equals => {
                        let sql = format!("json_extract(properties, '{}') = ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::NotEquals => {
                        let sql = format!("json_extract(properties, '{}') != ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::Contains => {
                        let sql = format!(
                            "json_extract(properties, '{}') LIKE '%' || ? || '%'",
                            json_path
                        );
                        (sql, vec![val])
                    }
                    PropertyOp::GreaterThan => {
                        let sql = format!("json_extract(properties, '{}') > ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::LessThan => {
                        let sql = format!("json_extract(properties, '{}') < ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::GreaterThanOrEqual => {
                        let sql = format!("json_extract(properties, '{}') >= ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::LessThanOrEqual => {
                        let sql = format!("json_extract(properties, '{}') <= ?", json_path);
                        (sql, vec![val])
                    }
                    PropertyOp::Between => {
                        let val2 = value2
                            .as_ref()
                            .map(|v| self.value_to_param(v))
                            .unwrap_or(val.clone());
                        let sql =
                            format!("json_extract(properties, '{}') BETWEEN ? AND ?", json_path);
                        (sql, vec![val, val2])
                    }
                }
            }

            QueryExpr::Tags(tag) => {
                let sql = "tags LIKE ?".to_string();
                let param = format!("%\"{} \"%", tag);
                (sql, vec![SqlParam::String(param)])
            }

            // Phase 2: SQL generation for new variants
            QueryExpr::Table(exprs) => {
                // Table combines inner expressions with AND
                if exprs.is_empty() {
                    ("1 = 1".to_string(), vec![])
                } else {
                    let clauses: Vec<_> = exprs.iter().map(|e| self.build_where(e)).collect();
                    let sqls: Vec<_> = clauses.iter().map(|(s, _)| format!("({})", s)).collect();
                    let params: Vec<_> = clauses.iter().flat_map(|(_, p)| p.clone()).collect();
                    (sqls.join(" AND "), params)
                }
            }
            QueryExpr::SortBy {
                field: _,
                direction: _,
                inner,
            } => {
                // SortBy: process inner expression for WHERE clause
                // ORDER BY is handled in build_sql
                self.build_where(inner)
            }
            QueryExpr::Exists(key) => {
                // Exists: json_extract(properties, '$.key') IS NOT NULL
                let sql = format!("json_extract(properties, '$.{}') IS NOT NULL", key);
                (sql, vec![])
            }
            QueryExpr::Missing(key) => {
                // Missing: json_extract(properties, '$.key') IS NULL
                let sql = format!("json_extract(properties, '$.{}') IS NULL", key);
                (sql, vec![])
            }
            QueryExpr::Namespace(ns) => {
                // Namespace: correlated subquery filtering by page namespace
                // Namespace is stored as a path on the page (e.g., "projects/work")
                let sql =
                    "EXISTS (SELECT 1 FROM pages p WHERE p.id = b.page_id AND p.namespace = ?)"
                        .to_string();
                (sql, vec![SqlParam::String(ns.clone())])
            }
        }
    }

    /// Builds a full SQL query from a query expression.
    ///
    /// Generates a complete SQL SELECT statement with proper JOINs,
    /// WHERE clause, ORDER BY (if SortBy), and LIMIT.
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
    /// let (sql, params) = executor.build_sql(&expr, 100);
    ///
    /// assert!(sql.contains("SELECT"));
    /// assert!(sql.contains("WHERE"));
    /// assert!(sql.contains("LIMIT 100"));
    /// ```
    pub fn build_sql(&self, expr: &QueryExpr, limit: usize) -> (String, Vec<SqlParam>) {
        let (where_clause, params) = self.build_where(expr);

        let mut sql = String::from(
            "SELECT b.*, p.name as page_name \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE ",
        );

        sql.push_str(&where_clause);

        // Handle SAMPLE - random ordering
        if matches!(expr, QueryExpr::Sample(_)) {
            sql.push_str(" ORDER BY RANDOM()");
        }

        // Handle SortBy - add ORDER BY clause
        if let QueryExpr::SortBy {
            field, direction, ..
        } = expr
        {
            let direction_sql = match direction {
                SortDirection::Asc => "ASC",
                SortDirection::Desc => "DESC",
            };
            // Check if field is an integer (column index) or string (column name)
            if field.parse::<i64>().is_ok() {
                // Integer column index - use directly
                sql.push_str(&format!(" ORDER BY {} {}", field, direction_sql));
            } else {
                // String column name - qualify if it's a known column
                let qualified_field = match field.as_str() {
                    "created_at" | "updated_at" | "scheduled" | "deadline" => {
                        format!("b.{}", field)
                    }
                    _ => field.clone(),
                };
                sql.push_str(&format!(" ORDER BY {} {}", qualified_field, direction_sql));
            }
        }

        sql.push_str(&format!(" LIMIT {}", limit));

        (sql, params)
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

    /// Convert a QueryValue to a timestamp SqlParam (i64 unix timestamp).
    ///
    /// Resolves TimeOffset strings (like "-7d") and Date strings (like "2024-01-15")
    /// to unix timestamps for comparison against INTEGER timestamp columns.
    fn value_to_timestamp_param(&self, value: &QueryValue) -> SqlParam {
        use crate::time_helpers::{parse_time_helper, TimeOffset};
        use chrono::NaiveDate;

        match value {
            QueryValue::TimeOffset(s) => {
                if let Some(offset) = TimeOffset::parse(s) {
                    let date = offset.to_date(chrono::Utc::now().date_naive());
                    let dt = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    SqlParam::Integer(dt.timestamp())
                } else if let Some(date) = parse_time_helper(s) {
                    let dt = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    SqlParam::Integer(dt.timestamp())
                } else {
                    // Fallback: treat as string
                    SqlParam::String(s.clone())
                }
            }
            QueryValue::Date(d) => {
                if let Ok(naive) = NaiveDate::parse_from_str(d, "%Y-%m-%d") {
                    let dt = naive.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    SqlParam::Integer(dt.timestamp())
                } else {
                    SqlParam::String(d.clone())
                }
            }
            QueryValue::Integer(n) => SqlParam::Integer(*n),
            QueryValue::String(s) => {
                // Try parsing as a date string
                if let Ok(naive) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                    let dt = naive.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    SqlParam::Integer(dt.timestamp())
                } else {
                    SqlParam::String(s.clone())
                }
            }
            QueryValue::Boolean(b) => SqlParam::Boolean(*b),
        }
    }
}

impl Default for QueryExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_task_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Task(vec!["todo".to_string()]);
        let (sql, params) = executor.build_sql(&expr, 100);

        assert!(sql.contains("marker IN"));
        assert!(sql.contains("LIMIT 100"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_page_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Page("Test Page".to_string());
        let (sql, params) = executor.build_where(&expr);

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
        let (sql, params) = executor.build_where(&expr);

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
        let (sql, params) = executor.build_where(&expr);

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
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains(" OR "));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_not_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Not(Box::new(QueryExpr::Task(vec!["done".to_string()])));
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("NOT"));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_page_ref_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::PageRef("TestPage".to_string());
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("content LIKE"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "%[[TestPage]]%");
    }

    #[test]
    fn test_block_content_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::BlockContent("hello world".to_string());
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("blocks_fts"));
        assert!(sql.contains("MATCH"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "hello world");
    }

    #[test]
    fn test_tags_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Tags("important".to_string());
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("tags LIKE"));
        assert_eq!(params.len(), 1);
        // The format is %"{tag} "% so it becomes %"important "%
        assert_eq!(params[0].as_string(), "%\"important \"%");
    }

    #[test]
    fn test_self_ref_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::SelfRef;
        let (sql, params) = executor.build_where(&expr);

        assert_eq!(sql, "1 = 1");
        assert!(params.is_empty());
    }

    #[test]
    fn test_sample_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Sample(5);
        let (where_sql, params) = executor.build_where(&expr);

        // Sample returns empty WHERE clause
        assert!(where_sql.is_empty());
        assert!(params.is_empty());
    }

    #[test]
    fn test_sample_query_full_sql() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Sample(10);
        let (sql, params) = executor.build_sql(&expr, 10);

        assert!(sql.contains("ORDER BY RANDOM()"));
        assert!(sql.contains("LIMIT 10"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_property_query() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "author".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::String("John".to_string()),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("$.author"));
        assert!(sql.contains("= ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_integer() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::Integer(42),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "42");
    }

    #[test]
    fn test_property_query_boolean() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "active".to_string(),
            op: PropertyOp::Equals,
            value: QueryValue::Boolean(true),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "true");
    }

    #[test]
    fn test_property_query_not_equals() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "status".to_string(),
            op: PropertyOp::NotEquals,
            value: QueryValue::String("done".to_string()),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("!= ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_contains() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "name".to_string(),
            op: PropertyOp::Contains,
            value: QueryValue::String("test".to_string()),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("LIKE '%' || ? || '%'"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_greater_than() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::GreaterThan,
            value: QueryValue::Integer(10),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("> ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_less_than() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::LessThan,
            value: QueryValue::Integer(100),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("< ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_greater_than_or_equal() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::GreaterThanOrEqual,
            value: QueryValue::Integer(10),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains(">= ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_less_than_or_equal() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::LessThanOrEqual,
            value: QueryValue::Integer(100),
            value2: None,
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("<= ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_property_query_between() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::Between,
            value: QueryValue::Integer(10),
            value2: Some(QueryValue::Integer(100)),
        };
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("json_extract"));
        assert!(sql.contains("BETWEEN ? AND ?"));
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].as_string(), "10");
        assert_eq!(params[1].as_string(), "100");
    }

    #[test]
    fn test_complex_and_or_not() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::And(vec![
            QueryExpr::Not(Box::new(QueryExpr::Task(vec!["done".to_string()]))),
            QueryExpr::Priority(vec!["a".to_string()]),
        ]);
        let (sql, params) = executor.build_where(&expr);

        assert!(sql.contains("NOT"));
        assert!(sql.contains("AND"));
        assert!(sql.contains("priority"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_build_sql_has_right_structure() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Task(vec!["todo".to_string()]);
        let (sql, params) = executor.build_sql(&expr, 100);

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
        let (_, params) = executor.build_where(&expr);

        // 1 param for task + 1 for priority + 1 for page
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_page_name_lowercased() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Page("MyPage".to_string());
        let (_, params) = executor.build_where(&expr);

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].as_string(), "mypage");
    }

    #[test]
    fn test_priority_lowercased() {
        let executor = QueryExecutor::new();
        let expr = QueryExpr::Priority(vec!["A".to_string(), "B".to_string()]);
        let (_, params) = executor.build_where(&expr);

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
        let (sql, params) = executor.build_sql(&expr, 50);

        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("WHERE b.created_at BETWEEN ? AND ?"));
        assert!(sql.contains("LIMIT 50"));
        assert_eq!(params.len(), 2);
    }
}
