//! Query compiler — errors, result types, and the `QueryCompiler` trait.
//!
//! F11 (Work Unit C) — the single SQL-generation entry point.
//! PI-1 — Aggregate, Stats, GroupBy, SortBy implementations.

use thiserror::Error;

use quilt_query_core::ast::{AggregateFn, AnalyzeKind, QueryAst, SortDirection, StatsFn, SqlParam};
use quilt_query_core::dialect::{SqlDialect, SqliteDialect};

/// Errors that can occur while compiling a `QueryAst` to SQL.
#[derive(Debug, Error, PartialEq)]
pub enum CompilerError {
    /// The requested operator / AST variant is not supported by the
    /// compiler.
    #[error("Unsupported operator: {op}")]
    UnsupportedOperator {
        /// Name of the operator (e.g., `"Analyze"`)
        op: &'static str,
    },
    /// Generic compilation failure.
    #[error("Invalid query compilation: {0}")]
    Invalid(String),
}

/// A compiled SQL query ready for execution against a SQLx connection.
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledQuery {
    /// The SQL string with `?` placeholders for parameters.
    pub sql: String,
    /// The bound parameters in the same order as the `?` placeholders.
    pub params: Vec<SqlParam>,
}

/// Trait for compiling a [`QueryAst`] into a [`CompiledQuery`].
///
/// F11 (Work Unit C) — replaces the three legacy methods on
/// [`crate::executor::QueryExecutor`] (`build_where`, `build_sql`,
/// `build_analyze_sql`) with a single `compile` entry point. The
/// `compile_*` methods are extension hooks — they default to
/// `Err(UnsupportedOperator)` and can be overridden by downstream
/// crates (`dsl-analyze`).
pub trait QueryCompiler: Send + Sync + std::fmt::Debug {
    /// Compile a [`QueryAst`] into a [`CompiledQuery`] with the given
    /// row limit. The default impl dispatches to the appropriate
    /// `compile_*` method, and falls back to wrapping `compile_where`
    /// in `SELECT ... FROM blocks b JOIN pages p ON ... WHERE ... LIMIT ...`
    /// for ordinary expressions.
    fn compile(&self, ast: &QueryAst, limit: usize) -> Result<CompiledQuery, CompilerError>;

    /// Compile the WHERE clause for an inner expression.
    fn compile_where(&self, ast: &QueryAst) -> Result<(String, Vec<SqlParam>), CompilerError>;

    /// Extension hook for `QueryAst::Analyze`. Default returns
    /// `Err(UnsupportedOperator { op: "Analyze" })`.
    fn compile_analyze(
        &self,
        _inner: &QueryAst,
        _kind: &AnalyzeKind,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "Analyze" })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy — fuzzy page name matching
    // ─────────────────────────────────────────────────────────────────────────

    /// Extension hook for `QueryAst::PageFuzzy`. Default returns
    /// `Err(UnsupportedOperator { op: "PageFuzzy" })`.
    fn compile_page_fuzzy(
        &self,
        _term: &str,
        _limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "PageFuzzy" })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal — temporal classification
    // ─────────────────────────────────────────────────────────────────────────

    /// Extension hook for `QueryAst::Temporal`. Default returns
    /// `Err(UnsupportedOperator { op: "Temporal" })`.
    fn compile_temporal(
        &self,
        _range: &quilt_query_core::ast::TemporalRange,
        _inner: &QueryAst,
        _limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "Temporal" })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect — virtual column selection
    // ─────────────────────────────────────────────────────────────────────────

    /// Extension hook for `QueryAst::VirtualSelect`. Default returns
    /// `Err(UnsupportedOperator { op: "VirtualSelect" })`.
    fn compile_virtual_select(
        &self,
        _columns: &[quilt_query_core::ast::VirtualColumn],
        _inner: &QueryAst,
        _limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator {
            op: "VirtualSelect",
        })
    }
}

/// Default SQLite compiler — implements [`QueryCompiler`] with a
/// SQLite-compatible `compile_where` and concrete implementations for
/// Aggregate, Stats, GroupBy, SortBy, PageFuzzy, Temporal, VirtualSelect.
#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteCompiler;

impl SqliteCompiler {
    /// Creates a new `SqliteCompiler` with default settings.
    pub fn new() -> Self {
        Self
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PI-1: Aggregate implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `Aggregate` using GROUP BY + aggregate function.
    pub fn compile_aggregate(
        &self,
        inner: &QueryAst,
        group_by: &str,
        aggregate_fn: &AggregateFn,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let dialect = SqliteDialect;
        let prop_path = dialect.property_path(group_by);

        let agg_expr = match aggregate_fn {
            AggregateFn::Count => "COUNT(*)".to_string(),
            _ => dialect.aggregate_fn(aggregate_fn.clone(), &prop_path),
        };

        let (inner_where, params) = self.compile_where(inner)?;

        let where_clause = if inner_where.is_empty() {
            "1 = 1".to_string()
        } else {
            inner_where
        };

        let sql = format!(
            "SELECT {} as group_val, {} as agg_val \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE {} \
             GROUP BY group_val \
             LIMIT {}",
            prop_path, agg_expr, where_clause, limit
        );

        Ok(CompiledQuery { sql, params })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PI-1: Stats implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `Stats` — computes a statistical function over a property.
    pub fn compile_stats(
        &self,
        property: &str,
        compute: &StatsFn,
    ) -> Result<CompiledQuery, CompilerError> {
        let dialect = SqliteDialect;
        let prop_path = dialect.property_path(property);

        let stats_expr = dialect.stats_fn(compute.clone(), &prop_path);

        let sql = format!(
            "SELECT {} as stat_val \
             FROM blocks b \
             WHERE {} IS NOT NULL \
             LIMIT 1",
            stats_expr, prop_path
        );

        Ok(CompiledQuery {
            sql,
            params: vec![],
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PI-1: GroupBy implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `GroupBy` — groups results by property value, returns blocks.
    pub fn compile_group_by(
        &self,
        inner: &QueryAst,
        property: &str,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let dialect = SqliteDialect;
        let prop_path = dialect.property_path(property);

        let (inner_where, params) = self.compile_where(inner)?;

        let where_clause = if inner_where.is_empty() {
            "1 = 1".to_string()
        } else {
            inner_where
        };

        let sql = format!(
            "SELECT b.*, p.name as page_name, {} as group_val \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE {} \
             GROUP BY group_val \
             LIMIT {}",
            prop_path, where_clause, limit
        );

        Ok(CompiledQuery { sql, params })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // PI-1: SortBy implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `SortBy` — wraps inner query with ORDER BY.
    pub fn compile_sort_by(
        &self,
        field: &str,
        direction: SortDirection,
        inner: &QueryAst,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let (inner_where, params) = self.compile_where(inner)?;

        let where_clause = if inner_where.is_empty() {
            "1 = 1".to_string()
        } else {
            inner_where
        };

        let order_expr = if matches!(
            field,
            "created_at" | "updated_at" | "content" | "order" | "level" | "id"
        ) {
            format!("b.{}", field)
        } else {
            format!("json_extract(b.properties, '$.{}')", field)
        };

        let dir_str = match direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        let sql = format!(
            "SELECT b.*, p.name as page_name \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE {} \
             ORDER BY {} {} \
             LIMIT {}",
            where_clause, order_expr, dir_str, limit
        );

        Ok(CompiledQuery { sql, params })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `PageFuzzy` using FTS5 prefix-first matching with LIKE fallback.
    pub fn compile_page_fuzzy(
        &self,
        term: &str,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let fts_sql = format!(
            "WITH fts_results AS ( \
                SELECT p.id, p.name FROM pages p \
                JOIN pages_fts f ON p.id = f.rowid \
                WHERE pages_fts MATCH '{{term}}*' \
                LIMIT {} \
            )",
            limit
        );

        let like_pattern = format!("%{}%", term.to_lowercase());

        let sql = format!(
            "{} \
             SELECT b.*, p.name as page_name \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE p.id IN (SELECT id FROM fts_results) \
                OR p.id IN ( \
                    SELECT id FROM fts_results UNION ALL \
                    SELECT p2.id FROM pages p2 \
                    WHERE LOWER(p2.name) LIKE ? AND p2.id NOT IN (SELECT id FROM fts_results) \
                    LIMIT ? \
                )",
            fts_sql
        );

        let params = vec![
            SqlParam::String(like_pattern),
            SqlParam::Integer(limit as i64),
        ];

        Ok(CompiledQuery { sql, params })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `Temporal` by combining `temporal_range_sql` with inner WHERE.
    pub fn compile_temporal(
        &self,
        range: &quilt_query_core::ast::TemporalRange,
        inner: &QueryAst,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let (temporal_sql, temporal_params) = SqliteDialect.temporal_range_sql(range);
        let (inner_sql, inner_params) = self.compile_where(inner)?;

        let where_clause = if inner_sql.is_empty() {
            temporal_sql
        } else {
            format!("{} AND ({})", temporal_sql, inner_sql)
        };

        let mut params = temporal_params;
        params.extend(inner_params);

        let sql = format!(
            "SELECT b.*, p.name as page_name \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE {} \
             LIMIT {}",
            where_clause, limit
        );

        Ok(CompiledQuery { sql, params })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `VirtualSelect` with computed columns.
    pub fn compile_virtual_select(
        &self,
        columns: &[quilt_query_core::ast::VirtualColumn],
        inner: &QueryAst,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        let mut select_cols = vec!["b.*".to_string(), "p.name as page_name".to_string()];

        for col in columns {
            let expr = match col {
                quilt_query_core::ast::VirtualColumn::WordCount => {
                    "LENGTH(b.content) - LENGTH(REPLACE(b.content, ' ', '')) + 1 AS word_count"
                        .to_string()
                }
                quilt_query_core::ast::VirtualColumn::CharCount => {
                    "LENGTH(b.content) AS char_count".to_string()
                }
                quilt_query_core::ast::VirtualColumn::RefCount => {
                    "(SELECT COUNT(*) FROM refs r WHERE r.block_id = b.id) AS ref_count"
                        .to_string()
                }
                quilt_query_core::ast::VirtualColumn::BlockAgeDays => {
                    "CAST(julianday('now') - julianday(b.created_at/1000, 'unixepoch') AS INTEGER) AS block_age_days".to_string()
                }
            };
            select_cols.push(expr);
        }

        let (inner_sql, inner_params) = self.compile_where(inner)?;

        let where_clause = if inner_sql.is_empty() {
            "1 = 1".to_string()
        } else {
            inner_sql
        };

        let sql = format!(
            "SELECT {} \
             FROM blocks b \
             JOIN pages p ON b.page_id = p.id \
             WHERE {} \
             LIMIT {}",
            select_cols.join(", "),
            where_clause,
            limit
        );

        Ok(CompiledQuery {
            sql,
            params: inner_params,
        })
    }
}

impl QueryCompiler for SqliteCompiler {
    fn compile(&self, ast: &QueryAst, limit: usize) -> Result<CompiledQuery, CompilerError> {
        match ast {
            // ── PI-1: Aggregate, Stats, GroupBy ──
            QueryAst::Aggregate {
                inner,
                group_by,
                aggregate_fn,
            } => self.compile_aggregate(inner, group_by, aggregate_fn, limit),

            QueryAst::Stats { property, compute } => self.compile_stats(property, compute),

            QueryAst::GroupBy { inner, property } => self.compile_group_by(inner, property, limit),

            // ── PI-1: SortBy ──
            QueryAst::SortBy {
                field,
                direction,
                inner,
            } => self.compile_sort_by(field, *direction, inner, limit),

            // ── Existing extensions ──
            QueryAst::Analyze { inner, kind } => self.compile_analyze(inner, kind),
            QueryAst::PageFuzzy { term, limit: _ } => self.compile_page_fuzzy(term, limit),
            QueryAst::Temporal { range, inner } => self.compile_temporal(range, inner, limit),
            QueryAst::VirtualSelect { columns, inner } => {
                self.compile_virtual_select(columns, inner, limit)
            }

            // ── Default: simple WHERE → SELECT ──
            _ => {
                let (where_clause, params) = self.compile_where(ast)?;

                let mut sql = String::from(
                    "SELECT b.*, p.name as page_name \
                     FROM blocks b \
                     JOIN pages p ON b.page_id = p.id \
                     WHERE ",
                );

                if !where_clause.is_empty() {
                    sql.push_str(&where_clause);
                } else {
                    sql.push_str("1 = 1");
                }

                if matches!(ast, QueryAst::Sample(_)) {
                    sql.push_str(" ORDER BY RANDOM()");
                }

                sql.push_str(&format!(" LIMIT {}", limit));

                Ok(CompiledQuery { sql, params })
            }
        }
    }

    fn compile_where(&self, ast: &QueryAst) -> Result<(String, Vec<SqlParam>), CompilerError> {
        // Delegate to the executor's `build_where` (single source of truth).
        let executor = crate::executor::QueryExecutor::with_dialect(SqliteDialect);
        executor.build_where(ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_query_core::ast::{AnalyzeKind, PropertyOp};
    use crate::parser::QueryParser;

    // ── PropertyOp SQL fragments ──

    fn fragment(op: PropertyOp) -> String {
        use quilt_query_core::dialect::SqlDialect;
        SqliteDialect.property_op_sql(op, "json_extract(properties, '$.count')")
    }

    #[test]
    fn test_property_op_sql_equals() {
        assert_eq!(
            fragment(PropertyOp::Equals),
            "json_extract(properties, '$.count') = ?"
        );
    }

    #[test]
    fn test_property_op_sql_not_equals() {
        assert_eq!(
            fragment(PropertyOp::NotEquals),
            "json_extract(properties, '$.count') != ?"
        );
    }

    #[test]
    fn test_property_op_sql_contains_is_like() {
        assert_eq!(
            fragment(PropertyOp::Contains),
            "json_extract(properties, '$.count') LIKE ?"
        );
    }

    #[test]
    fn test_property_op_sql_greater_than() {
        assert_eq!(
            fragment(PropertyOp::GreaterThan),
            "json_extract(properties, '$.count') > ?"
        );
    }

    #[test]
    fn test_property_op_sql_less_than() {
        assert_eq!(
            fragment(PropertyOp::LessThan),
            "json_extract(properties, '$.count') < ?"
        );
    }

    #[test]
    fn test_property_op_sql_greater_than_or_equal() {
        assert_eq!(
            fragment(PropertyOp::GreaterThanOrEqual),
            "json_extract(properties, '$.count') >= ?"
        );
    }

    #[test]
    fn test_property_op_sql_less_than_or_equal() {
        assert_eq!(
            fragment(PropertyOp::LessThanOrEqual),
            "json_extract(properties, '$.count') <= ?"
        );
    }

    #[test]
    fn test_property_op_sql_between() {
        assert_eq!(
            fragment(PropertyOp::Between),
            "json_extract(properties, '$.count') BETWEEN ? AND ?"
        );
    }

    // ── Simple compile tests ──

    #[test]
    fn test_compile_simple_task_has_limit_and_param_count() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Task(vec!["todo".to_string()]);
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("LIMIT 100"));
        assert_eq!(result.sql.matches('?').count(), result.params.len());
    }

    #[test]
    fn test_compile_property_greater_than_uses_json_extract() {
        let parser = QueryParser;
        let ast = parser.parse("(property \"count\" > 5)").unwrap();
        let compiler = SqliteCompiler::new();
        let result = compiler.compile(&ast, 50).unwrap();
        assert!(
            result
                .sql
                .contains("json_extract(properties, '$.count') > ?")
        );
        assert!(result.sql.contains("LIMIT 50"));
        assert_eq!(result.params[0].as_string(), "5");
    }

    #[test]
    fn test_compile_contains_binds_with_like() {
        let parser = QueryParser;
        let ast = parser.parse("(property \"name\" contains \"ru\")").unwrap();
        let compiler = SqliteCompiler::new();
        let result = compiler.compile(&ast, 10).unwrap();
        assert!(result.sql.contains("LIKE"));
        assert_eq!(result.params[0].as_string(), "%ru%");
    }

    // ── PI-1: Aggregate tests ──

    #[test]
    fn test_compile_aggregate_count_generates_group_by() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Aggregate {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            group_by: "status".to_string(),
            aggregate_fn: AggregateFn::Count,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(
            result.sql.contains("GROUP BY"),
            "expected GROUP BY, got: {}",
            result.sql
        );
        assert!(
            result.sql.contains("COUNT(*)"),
            "expected COUNT(*), got: {}",
            result.sql
        );
        assert!(
            result.sql.contains("group_val"),
            "expected group_val alias, got: {}",
            result.sql
        );
        assert!(result.sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_compile_aggregate_avg_uses_json_extract() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Aggregate {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            group_by: "priority".to_string(),
            aggregate_fn: AggregateFn::Avg,
        };
        let result = compiler.compile(&ast, 50).unwrap();
        assert!(result.sql.contains("AVG("));
        assert!(result.sql.contains("json_extract"));
        assert!(result.sql.contains("LIMIT 50"));
    }

    #[test]
    fn test_compile_aggregate_sum() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Aggregate {
            inner: Box::new(QueryAst::Task(vec!["done".to_string()])),
            group_by: "project".to_string(),
            aggregate_fn: AggregateFn::Sum,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("SUM("));
    }

    #[test]
    fn test_compile_aggregate_min_max() {
        for fn_type in [AggregateFn::Min, AggregateFn::Max] {
            let compiler = SqliteCompiler::new();
            let expected = format!("{:?}", fn_type).to_uppercase();
            let ast = QueryAst::Aggregate {
                inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
                group_by: "score".to_string(),
                aggregate_fn: fn_type,
            };
            let result = compiler.compile(&ast, 100).unwrap();
            assert!(result.sql.contains(&expected));
        }
    }

    #[test]
    fn test_compile_aggregate_combines_inner_where() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Aggregate {
            inner: Box::new(QueryAst::And(vec![
                QueryAst::Task(vec!["todo".to_string()]),
                QueryAst::Page("Project".to_string()),
            ])),
            group_by: "status".to_string(),
            aggregate_fn: AggregateFn::Count,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("EXISTS"));
        assert!(result.sql.contains("GROUP BY"));
    }

    // ── PI-1: Stats tests ──

    #[test]
    fn test_compile_stats_stddev() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Stats {
            property: "priority".to_string(),
            compute: StatsFn::Stddev,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(
            result.sql.contains("STDDEV_POP"),
            "expected STDDEV_POP, got: {}",
            result.sql
        );
        assert!(result.sql.contains("IS NOT NULL"));
    }

    #[test]
    fn test_compile_stats_variance() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Stats {
            property: "score".to_string(),
            compute: StatsFn::Variance,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("VAR_POP"));
    }

    #[test]
    fn test_compile_stats_median() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Stats {
            property: "count".to_string(),
            compute: StatsFn::Median,
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(
            result.sql.contains("ROW_NUMBER"),
            "expected ROW_NUMBER for median, got: {}",
            result.sql
        );
    }

    #[test]
    fn test_compile_stats_percentile() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Stats {
            property: "score".to_string(),
            compute: StatsFn::Percentile(90),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("ROW_NUMBER"));
        assert!(result.sql.contains("0.9")); // 90/100
    }

    // ── PI-1: GroupBy tests ──

    #[test]
    fn test_compile_group_by_generates_group_by() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::GroupBy {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            property: "status".to_string(),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("GROUP BY"));
        assert!(result.sql.contains("group_val"));
        assert!(result.sql.contains("b.*")); // returns block data
        assert!(result.sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_compile_group_by_uses_json_extract() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::GroupBy {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            property: "priority".to_string(),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("json_extract"));
    }

    // ── PI-1: SortBy tests ──

    #[test]
    fn test_compile_sort_by_asc() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::SortBy {
            field: "priority".to_string(),
            direction: SortDirection::Asc,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(
            result.sql.contains("ORDER BY"),
            "expected ORDER BY, got: {}",
            result.sql
        );
        assert!(result.sql.contains("ASC"));
        assert!(result.sql.contains("json_extract"));
    }

    #[test]
    fn test_compile_sort_by_desc() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::SortBy {
            field: "priority".to_string(),
            direction: SortDirection::Desc,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("ORDER BY"));
        assert!(result.sql.contains("DESC"));
    }

    #[test]
    fn test_compile_sort_by_block_column_uses_direct_ref() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::SortBy {
            field: "created_at".to_string(),
            direction: SortDirection::Desc,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(
            result.sql.contains("b.created_at"),
            "expected b.created_at for known column, got: {}",
            result.sql
        );
        assert!(!result.sql.contains("json_extract(properties"));
    }

    #[test]
    fn test_compile_sort_by_combines_inner_where() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::SortBy {
            field: "priority".to_string(),
            direction: SortDirection::Asc,
            inner: Box::new(QueryAst::And(vec![
                QueryAst::Task(vec!["todo".to_string()]),
                QueryAst::Page("Project".to_string()),
            ])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("ORDER BY"));
        assert!(result.sql.contains("EXISTS")); // inner AND
        assert!(result.sql.contains("LIMIT 100"));
    }

    // ── Analyze stays unsupported ──

    #[test]
    fn test_compile_analyze_still_returns_unsupported() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Analyze {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            kind: AnalyzeKind::StructuralMirror,
        };
        assert_eq!(
            compiler.compile(&ast, 100),
            Err(CompilerError::UnsupportedOperator { op: "Analyze" })
        );
    }

    // ── G5: PageFuzzy ──

    #[test]
    fn test_compile_page_fuzzy_produces_fts_cte() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::PageFuzzy {
            term: "rust".to_string(),
            limit: 10,
        };
        let result = compiler.compile(&ast, 10).unwrap();
        assert!(result.sql.contains("pages_fts"));
        assert!(result.sql.contains("MATCH"));
        assert!(result.sql.contains("LIMIT 10"));
    }

    #[test]
    fn test_compile_page_fuzzy_term_is_lowercased() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::PageFuzzy {
            term: "Rust".to_string(),
            limit: 10,
        };
        let result = compiler.compile(&ast, 10).unwrap();
        assert_eq!(result.params.len(), 2);
    }

    // ── G3: Temporal ──

    #[test]
    fn test_compile_temporal_today_produces_date_filter() {
        use quilt_query_core::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Today,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_compile_temporal_this_week_produces_week_filter() {
        use quilt_query_core::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::ThisWeek,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains(">="));
    }

    #[test]
    fn test_compile_temporal_custom_date_range() {
        use quilt_query_core::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Custom {
                start: "2024-01-01".to_string(),
                end: "2024-12-31".to_string(),
            },
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("BETWEEN"));
        assert_eq!(result.params.len(), 3);
    }

    #[test]
    fn test_compile_temporal_combines_with_inner() {
        use quilt_query_core::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Today,
            inner: Box::new(QueryAst::Page("Test".to_string())),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains("EXISTS"));
    }

    // ── F12: VirtualSelect ──

    #[test]
    fn test_compile_virtual_select_word_count_column() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::WordCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("LENGTH"));
    }

    #[test]
    fn test_compile_virtual_select_char_count_column() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::CharCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("char_count"));
        assert!(result.sql.contains("LENGTH(b.content)"));
    }

    #[test]
    fn test_compile_virtual_select_ref_count_column() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::RefCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("ref_count"));
        assert!(result.sql.contains("SELECT COUNT(*) FROM refs"));
    }

    #[test]
    fn test_compile_virtual_select_block_age_days_column() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::BlockAgeDays],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("block_age_days"));
        assert!(result.sql.contains("julianday"));
    }

    #[test]
    fn test_compile_virtual_select_multiple_columns() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![
                VirtualColumn::WordCount,
                VirtualColumn::CharCount,
                VirtualColumn::RefCount,
            ],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("char_count"));
        assert!(result.sql.contains("ref_count"));
    }

    #[test]
    fn test_compile_virtual_select_combines_with_inner() {
        use quilt_query_core::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::WordCount],
            inner: Box::new(QueryAst::Page("Test".to_string())),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("EXISTS"));
    }

    // ── End-to-end tests ──

    #[test]
    fn test_end_to_end_property_greater_than() {
        let parser = QueryParser;
        let ast = parser
            .parse("(property \"count\" > 5)")
            .expect("parse must succeed");
        let compiler = SqliteCompiler::new();
        let compiled = compiler.compile(&ast, 100).expect("compile must succeed");
        assert!(
            compiled
                .sql
                .contains("json_extract(properties, '$.count') > ?")
        );
        assert!(compiled.sql.contains("LIMIT 100"));
        assert_eq!(compiled.params.len(), 1);
        assert_eq!(compiled.params[0].as_string(), "5");
    }

    #[test]
    fn test_end_to_end_contains() {
        let parser = QueryParser;
        let ast = parser
            .parse("(property \"name\" contains \"ru\")")
            .expect("parse must succeed");
        let compiler = SqliteCompiler::new();
        let compiled = compiler.compile(&ast, 50).expect("compile must succeed");
        assert!(compiled.sql.contains("LIKE"));
        assert_eq!(compiled.params[0].as_string(), "%ru%");
    }

    // ── PI-1: End-to-end Aggregate ──

    #[test]
    fn test_end_to_end_aggregate_parse_and_compile() {
        let parser = QueryParser;
        let ast = parser
            .parse("(aggregate (task todo) \"status\" count)")
            .expect("parse must succeed");
        let compiler = SqliteCompiler::new();
        let compiled = compiler.compile(&ast, 50).expect("compile must succeed");
        assert!(compiled.sql.contains("GROUP BY"));
        assert!(compiled.sql.contains("COUNT(*)"));
        assert!(compiled.sql.contains("LIMIT 50"));
    }

    // ── PI-1: End-to-end SortBy ──

    #[test]
    fn test_end_to_end_sort_by_parse_and_compile() {
        let parser = QueryParser;
        let ast = parser
            .parse("(sort-by \"priority\" asc (task todo))")
            .expect("parse must succeed");
        let compiler = SqliteCompiler::new();
        let compiled = compiler.compile(&ast, 50).expect("compile must succeed");
        assert!(compiled.sql.contains("ORDER BY"));
        assert!(compiled.sql.contains("ASC"));
        assert!(compiled.sql.contains("LIMIT 50"));
    }
}
