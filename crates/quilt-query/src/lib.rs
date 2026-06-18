//! Quilt Query DSL
//!
//! This crate provides the query language parser, AST, and executor
//! for Quilt-style queries.
//!
//! # Architecture
//!
//! - [`parser`]: Recursive descent parser for the query DSL (from `quilt-query-core`)
//! - [`ast`]: AST types (from `quilt-query-core`)
//! - [`compiler`]: SQL query generator from AST (sqlx-based)
//! - [`executor`]: SQL query executor (sqlx-based)
//! - [`time_helpers`]: Time offset parsing utilities (from `quilt-query-core`)
//!
//! # Query DSL Syntax
//!
//! The query DSL supports the following expressions:
//!
//! - `(task todo done)` - Filter by task markers
//! - `(priority a b c)` - Filter by priority levels
//! - `(page "Name")` - Filter by page name
//! - `(property "key" "value")` - Filter by JSON property
//! - `(tags "tag")` - Filter by tag
//! - `(between 1000 2000)` - Numeric range filter
//! - `(full-text-search "keyword")` - FTS search
//! - `(sample N)` - Random sample
//! - `[[Page Name]]` - Page reference
//! - `self` - Current block reference
//! - `(and ...)` / `(or ...)` / `(not ...)` - Boolean logic
//!
//! # Example
//!
//! ```
//! use quilt_query::{QueryParser, QueryExecutor};
//!
//! let parser = QueryParser;
//! let executor = QueryExecutor::new();
//!
//! // Parse a query
//! let expr = parser.parse("(task todo)").unwrap();
//!
//! // Generate SQL
//! let (sql, params) = executor.build_sql(&expr, 100).unwrap();
//! ```

// Re-export core types from quilt-query-core
pub use quilt_query_core::ast::{PropertyOp, QueryAst, QueryValue, SortDirection, SqlParam};
pub use quilt_query_core::dialect::{SqlDialect, SqliteDialect, WindowFnKind};
pub use quilt_query_core::parser::{AggregateFn, AnalyzeKind, ParseError, QueryError, QueryParser, StatsFn};
pub use quilt_query_core::time_helpers::TimeOffset;

// Modules that use sqlx (compiler and executor stay in this crate)
pub mod compiler;
pub mod executor;

// Re-export QueryExecutor from executor module
pub use executor::QueryExecutor;

// Tests
#[cfg(test)]
mod tests {
    // Re-export test helpers from quilt-query-core for integration tests
    pub use quilt_query_core::ast::{PropertyOp, QueryAst, QueryValue, SortDirection};
    pub use quilt_query_core::parser::{AggregateFn, AnalyzeKind, ParseError, QueryParser, StatsFn};
}
