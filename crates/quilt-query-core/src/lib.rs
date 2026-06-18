//! Quilt Query DSL — Core (Parser + AST)
//!
//! This crate provides the query language parser and AST for Quilt-style queries.
//! It is WASM-compatible and has no runtime dependencies (no tokio, no sqlx).
//!
//! # Architecture
//!
//! - [`parser`]: Recursive descent parser for the query DSL
//! - [`ast`]: AST types for the query language
//! - [`dialect`]: SQL dialect abstractions
//! - [`time_helpers`]: Time offset parsing utilities
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
//! use quilt_query_core::{QueryParser, QueryAst, PropertyOp, QueryValue, SortDirection};
//!
//! let parser = QueryParser;
//! let expr = parser.parse("(task todo)").unwrap();
//! ```

pub mod ast;
pub mod dialect;
pub mod grammar;
pub mod merge;
pub mod parser;
pub mod property_op;
pub mod time_helpers;

// NOTE: Grammar pest integration is deferred.
// The grammar/ directory exists but parser.rs uses a hand-written
// recursive descent parser instead of pest.

pub use ast::{PropertyOp, QueryAst, QueryValue, SortDirection, SqlParam};
pub use dialect::{SqlDialect, SqliteDialect, WindowFnKind};
pub use parser::{AggregateFn, AnalyzeKind, ParseError, QueryError, QueryParser, StatsFn};
