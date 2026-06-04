//! Query compiler — errors, result types, and the `QueryCompiler` trait.
//!
//! F11 (Work Unit C) — the single SQL-generation entry point.

use thiserror::Error;

use crate::ast::{AggregateFn, AnalyzeKind, QueryAst, StatsFn};
use crate::dialect::{SqlDialect, SqliteDialect};
use crate::executor::SqlParam;

/// Errors that can occur while compiling a `QueryAst` to SQL.
#[derive(Debug, Error, PartialEq)]
pub enum CompilerError {
    /// The requested operator / AST variant is not supported by the
    /// compiler.
    #[error("Unsupported operator: {op}")]
    UnsupportedOperator {
        /// Name of the operator (e.g., `"Stats"`, `"Analyze"`, `"Aggregate"`)
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
/// crates (`dsl-aggregates`, `dsl-analyze`).
pub trait QueryCompiler: Send + Sync + std::fmt::Debug {
    /// Compile a [`QueryAst`] into a [`CompiledQuery`] with the given
    /// row limit. The default impl dispatches to `compile_aggregate`,
    /// `compile_stats`, `compile_group_by`, `compile_analyze` (each
    /// defaulting to `Err(UnsupportedOperator)`), and falls back to
    /// wrapping `compile_where` in `SELECT ... FROM blocks b JOIN
    /// pages p ON ... WHERE ... LIMIT ...` for ordinary expressions.
    fn compile(&self, ast: &QueryAst, limit: usize) -> Result<CompiledQuery, CompilerError>;

    /// Compile the WHERE clause for an inner expression.
    fn compile_where(&self, ast: &QueryAst) -> Result<(String, Vec<SqlParam>), CompilerError>;

    /// Extension hook for `QueryAst::Aggregate`. Default returns
    /// `Err(UnsupportedOperator { op: "Aggregate" })`.
    fn compile_aggregate(
        &self,
        _inner: &QueryAst,
        _group_by: &str,
        _aggregate_fn: &AggregateFn,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "Aggregate" })
    }

    /// Extension hook for `QueryAst::Stats`. Default returns
    /// `Err(UnsupportedOperator { op: "Stats" })`.
    fn compile_stats(
        &self,
        _property: &str,
        _compute: &StatsFn,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "Stats" })
    }

    /// Extension hook for `QueryAst::GroupBy`. Default returns
    /// `Err(UnsupportedOperator { op: "GroupBy" })`.
    fn compile_group_by(
        &self,
        _inner: &QueryAst,
        _property: &str,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator { op: "GroupBy" })
    }

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
    ///
    /// The actual implementation is in `SqliteCompiler::compile_page_fuzzy`
    /// which provides FTS5 CTE with LIKE fallback.
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
    ///
    /// The actual implementation is in `SqliteCompiler::compile_temporal`.
    fn compile_temporal(
        &self,
        _range: &crate::ast::TemporalRange,
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
    ///
    /// The actual implementation is in `SqliteCompiler::compile_virtual_select`.
    fn compile_virtual_select(
        &self,
        _columns: &[crate::ast::VirtualColumn],
        _inner: &QueryAst,
        _limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        Err(CompilerError::UnsupportedOperator {
            op: "VirtualSelect",
        })
    }
}

/// Default SQLite compiler — implements [`QueryCompiler`] with a
/// SQLite-compatible `compile_where` and dispatches all extension
/// hooks to `Err(UnsupportedOperator)` (Q1 contract).
#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteCompiler;

impl SqliteCompiler {
    /// Creates a new `SqliteCompiler` with default settings.
    pub fn new() -> Self {
        Self
    }
}

impl QueryCompiler for SqliteCompiler {
    fn compile(&self, ast: &QueryAst, limit: usize) -> Result<CompiledQuery, CompilerError> {
        match ast {
            QueryAst::Aggregate {
                inner,
                group_by,
                aggregate_fn,
            } => self.compile_aggregate(inner, group_by, aggregate_fn),
            QueryAst::Stats { property, compute } => self.compile_stats(property, compute),
            QueryAst::GroupBy { inner, property } => self.compile_group_by(inner, property),
            QueryAst::Analyze { inner, kind } => self.compile_analyze(inner, kind),
            QueryAst::PageFuzzy { term, limit: _ } => self.compile_page_fuzzy(term, limit),
            QueryAst::Temporal { range, inner } => self.compile_temporal(range, inner, limit),
            QueryAst::VirtualSelect { columns, inner } => {
                self.compile_virtual_select(columns, inner, limit)
            }
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

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy implementation
    // ─────────────────────────────────────────────────────────────────────────

    /// Compiles `PageFuzzy` using FTS5 prefix-first matching with LIKE fallback.
    ///
    /// Strategy:
    /// 1. Try FTS5 `term*` prefix search on `pages_fts`
    /// 2. If FTS5 returns fewer than `limit` results, fall back to LIKE on page names
    fn compile_page_fuzzy(&self, term: &str, limit: usize) -> Result<CompiledQuery, CompilerError> {
        use SqliteDialect;

        // FTS5 CTE: try prefix search first
        let fts_sql = format!(
            "WITH fts_results AS ( \
                SELECT p.id, p.name FROM pages p \
                JOIN pages_fts f ON p.id = f.rowid \
                WHERE pages_fts MATCH '{{term}}*' \
                LIMIT {} \
            )",
            limit
        );

        // LIKE fallback for when FTS5 returns too few results
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
    fn compile_temporal(
        &self,
        range: &crate::ast::TemporalRange,
        inner: &QueryAst,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        use SqliteDialect;

        // Get temporal range SQL
        let (temporal_sql, temporal_params) = SqliteDialect.temporal_range_sql(range);

        // Get inner WHERE clause
        let (inner_sql, inner_params) = self.compile_where(inner)?;

        // Combine
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

    /// Compiles `VirtualSelect` with computed columns:
    /// - word_count: `LENGTH(content) - LENGTH(REPLACE(content, ' ', '')) + 1`
    /// - char_count: `LENGTH(content)`
    /// - ref_count: subquery on refs table
    /// - block_age_days: `CAST(julianday('now') - julianday(created_at/1000, 'unixepoch') AS INTEGER)`
    fn compile_virtual_select(
        &self,
        columns: &[crate::ast::VirtualColumn],
        inner: &QueryAst,
        limit: usize,
    ) -> Result<CompiledQuery, CompilerError> {
        // Build SELECT columns
        let mut select_cols = vec!["b.*".to_string(), "p.name as page_name".to_string()];

        for col in columns {
            let expr = match col {
                crate::ast::VirtualColumn::WordCount => {
                    "LENGTH(b.content) - LENGTH(REPLACE(b.content, ' ', '')) + 1 AS word_count"
                        .to_string()
                }
                crate::ast::VirtualColumn::CharCount => {
                    "LENGTH(b.content) AS char_count".to_string()
                }
                crate::ast::VirtualColumn::RefCount => {
                    "(SELECT COUNT(*) FROM refs r WHERE r.block_id = b.id) AS ref_count".to_string()
                }
                crate::ast::VirtualColumn::BlockAgeDays => {
                    "CAST(julianday('now') - julianday(b.created_at/1000, 'unixepoch') AS INTEGER) AS block_age_days".to_string()
                }
            };
            select_cols.push(expr);
        }

        // Get inner WHERE clause
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{AnalyzeKind, PropertyOp, QueryValue};
    use crate::parser::QueryParser;

    // F3 — all 8 PropertyOp SQL fragments via property_op_sql on dialect
    fn fragment(op: PropertyOp) -> String {
        use crate::dialect::SqlDialect;
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

    // F11 — SqliteCompiler::compile returns SQL with LIMIT N and param count

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

    #[test]
    fn test_compile_aggregate_default_returns_unsupported() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Aggregate {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            group_by: "author".to_string(),
            aggregate_fn: AggregateFn::Count,
        };
        assert_eq!(
            compiler.compile(&ast, 100),
            Err(CompilerError::UnsupportedOperator { op: "Aggregate" })
        );
    }

    #[test]
    fn test_compile_stats_default_returns_unsupported() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Stats {
            property: "count".to_string(),
            compute: StatsFn::Stddev,
        };
        assert_eq!(
            compiler.compile(&ast, 100),
            Err(CompilerError::UnsupportedOperator { op: "Stats" })
        );
    }

    #[test]
    fn test_compile_group_by_default_returns_unsupported() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::GroupBy {
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
            property: "author".to_string(),
        };
        assert_eq!(
            compiler.compile(&ast, 100),
            Err(CompilerError::UnsupportedOperator { op: "GroupBy" })
        );
    }

    #[test]
    fn test_compile_analyze_default_returns_unsupported() {
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

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy implementation tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_compile_page_fuzzy_produces_fts_cte() {
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::PageFuzzy {
            term: "rust".to_string(),
            limit: 10,
        };
        let result = compiler.compile(&ast, 10).unwrap();
        // Should contain FTS5 CTE
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
        // Term should be lowercased in params
        assert_eq!(result.params.len(), 2);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal implementation tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_compile_temporal_today_produces_date_filter() {
        use crate::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Today,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain date filter with today
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_compile_temporal_this_week_produces_week_filter() {
        use crate::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::ThisWeek,
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain date filter with >=
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains(">="));
    }

    #[test]
    fn test_compile_temporal_custom_date_range() {
        use crate::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Custom {
                start: "2024-01-01".to_string(),
                end: "2024-12-31".to_string(),
            },
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain BETWEEN for custom range
        assert!(result.sql.contains("BETWEEN"));
        assert_eq!(result.params.len(), 3); // 2 dates + 1 task marker
    }

    #[test]
    fn test_compile_temporal_combines_with_inner() {
        use crate::ast::TemporalRange;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::Temporal {
            range: TemporalRange::Today,
            inner: Box::new(QueryAst::Page("Test".to_string())),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain both temporal and inner conditions
        assert!(result.sql.contains("date(b.created_at"));
        assert!(result.sql.contains("EXISTS"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect implementation tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_compile_virtual_select_word_count_column() {
        use crate::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::WordCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain word_count SQL expression
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("LENGTH"));
    }

    #[test]
    fn test_compile_virtual_select_char_count_column() {
        use crate::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::CharCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain char_count SQL expression
        assert!(result.sql.contains("char_count"));
        assert!(result.sql.contains("LENGTH(b.content)"));
    }

    #[test]
    fn test_compile_virtual_select_ref_count_column() {
        use crate::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::RefCount],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain ref_count with subquery
        assert!(result.sql.contains("ref_count"));
        assert!(result.sql.contains("SELECT COUNT(*) FROM refs"));
    }

    #[test]
    fn test_compile_virtual_select_block_age_days_column() {
        use crate::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::BlockAgeDays],
            inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain block_age_days with julianday
        assert!(result.sql.contains("block_age_days"));
        assert!(result.sql.contains("julianday"));
    }

    #[test]
    fn test_compile_virtual_select_multiple_columns() {
        use crate::ast::VirtualColumn;
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
        // Should contain all three column expressions
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("char_count"));
        assert!(result.sql.contains("ref_count"));
    }

    #[test]
    fn test_compile_virtual_select_combines_with_inner() {
        use crate::ast::VirtualColumn;
        let compiler = SqliteCompiler::new();
        let ast = QueryAst::VirtualSelect {
            columns: vec![VirtualColumn::WordCount],
            inner: Box::new(QueryAst::Page("Test".to_string())),
        };
        let result = compiler.compile(&ast, 100).unwrap();
        // Should contain both virtual column and inner filter
        assert!(result.sql.contains("word_count"));
        assert!(result.sql.contains("EXISTS"));
    }

    // T-D.3 — End-to-end integration: parse → compile → SQL.
    // Asserts that `(property "count" > 5)` produces
    //   `json_extract(properties,'$.count') > ?`
    // with the bound value `5`.

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
                .contains("json_extract(properties, '$.count') > ?"),
            "expected '> ?' in SQL, got: {}",
            compiled.sql
        );
        assert!(
            compiled.sql.contains("LIMIT 100"),
            "expected LIMIT 100 in SQL, got: {}",
            compiled.sql
        );
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
        // Caller wraps the value as %v% at compile time.
        assert_eq!(compiled.params[0].as_string(), "%ru%");
    }
}
