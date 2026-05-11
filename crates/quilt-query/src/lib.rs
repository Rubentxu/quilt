//! Quilt Query DSL
//!
//! This crate provides the query language parser, AST, and executor
//! for Logseq-style queries.
//!
//! # Architecture
//!
//! - [`parser`]: Recursive descent parser for the query DSL
//! - [`executor`]: SQL query generator from AST
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
//! use quilt_query::{QueryParser, QueryExecutor};
//!
//! let parser = QueryParser;
//! let executor = QueryExecutor::new();
//!
//! // Parse a query
//! let expr = parser.parse("(task todo)").unwrap();
//!
//! // Generate SQL
//! let (sql, params) = executor.build_sql(&expr, 100);
//! ```

pub mod executor;
pub mod grammar;
pub mod parser;
pub mod time_helpers;

pub use executor::QueryExecutor;
pub use grammar::QueryGrammar;
pub use parser::{
    preprocess, validate, ParseError, PropertyOp, QueryExpr, QueryParser, SortDirection,
};
