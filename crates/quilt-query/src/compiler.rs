//! Query compiler — errors, result types, and the `QueryCompiler` trait.
//!
//! F11 (Work Unit C) — the single SQL-generation entry point.

use thiserror::Error;

use crate::ast::{AggregateFn, AnalyzeKind, QueryAst, StatsFn};
use crate::dialect::SqliteDialect;
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
    use crate::ast::{AnalyzeKind, PropertyOp, QueryValue};
    use crate::parser::QueryParser;

    // F3 — all 8 PropertyOp SQL fragments via property_op_sql on dialect
    fn fragment(op: PropertyOp) -> String {
        use crate::dialect::SqlDialect;
        SqliteDialect.property_op_sql(op, "json_extract(properties, '$.count')")
    }

    #[test]
    fn test_property_op_sql_equals() {
        assert_eq!(fragment(PropertyOp::Equals), "json_extract(properties, '$.count') = ?");
    }

    #[test]
    fn test_property_op_sql_not_equals() {
        assert_eq!(fragment(PropertyOp::NotEquals), "json_extract(properties, '$.count') != ?");
    }

    #[test]
    fn test_property_op_sql_contains_is_like() {
        assert_eq!(fragment(PropertyOp::Contains), "json_extract(properties, '$.count') LIKE ?");
    }

    #[test]
    fn test_property_op_sql_greater_than() {
        assert_eq!(fragment(PropertyOp::GreaterThan), "json_extract(properties, '$.count') > ?");
    }

    #[test]
    fn test_property_op_sql_less_than() {
        assert_eq!(fragment(PropertyOp::LessThan), "json_extract(properties, '$.count') < ?");
    }

    #[test]
    fn test_property_op_sql_greater_than_or_equal() {
        assert_eq!(fragment(PropertyOp::GreaterThanOrEqual), "json_extract(properties, '$.count') >= ?");
    }

    #[test]
    fn test_property_op_sql_less_than_or_equal() {
        assert_eq!(fragment(PropertyOp::LessThanOrEqual), "json_extract(properties, '$.count') <= ?");
    }

    #[test]
    fn test_property_op_sql_between() {
        assert_eq!(fragment(PropertyOp::Between), "json_extract(properties, '$.count') BETWEEN ? AND ?");
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
        assert!(result.sql.contains("json_extract(properties, '$.count') > ?"));
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
        let compiled = compiler
            .compile(&ast, 100)
            .expect("compile must succeed");
        assert!(
            compiled.sql.contains("json_extract(properties, '$.count') > ?"),
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
        let compiled = compiler
            .compile(&ast, 50)
            .expect("compile must succeed");
        assert!(compiled.sql.contains("LIKE"));
        // Caller wraps the value as %v% at compile time.
        assert_eq!(compiled.params[0].as_string(), "%ru%");
    }
}
