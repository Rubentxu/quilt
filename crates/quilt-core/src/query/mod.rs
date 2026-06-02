//! Query DSL — parser and AST
//!
//! This module provides the Logseq-style query DSL parser and its AST types.
//! It is a portable, dependency-light extraction of the query parser from
//! `quilt-query`, suitable for WASM targets.
//!
//! # Architecture
//!
//! - [`ast`]: AST types ([`QueryExpr`], [`QueryValue`], etc.) with serde support
//! - [`parser`]: Recursive descent parser ([`QueryParser`])
//!
//! # Quick start
//!
//! ```
//! use quilt_core::query::QueryParser;
//!
//! let parser = QueryParser;
//! let expr = parser.parse("(task todo)").unwrap();
//! ```

pub mod ast;
pub mod parser;

pub use ast::{AggregateFn, AnalyzeKind, ParseError, QueryExpr, QueryValue, StatsFn};
pub use parser::QueryParser;
