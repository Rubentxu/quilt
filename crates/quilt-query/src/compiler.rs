//! Query compiler — errors and result types.
//!
//! This module hosts the [`CompilerError`] type and the [`CompiledQuery`]
//! result struct used by [`crate::executor::QueryExecutor`] and the
//! `QueryCompiler` trait (introduced in F11, fully implemented in Work
//! Unit C). For Work Unit B, only the error type is added so that
//! `build_where` can return `Result` instead of `panic!`-ing for
//! `Stats` and `Analyze` variants.

use thiserror::Error;

use crate::executor::SqlParam;

/// Errors that can occur while compiling a `QueryAst` to SQL.
#[derive(Debug, Error, PartialEq)]
pub enum CompilerError {
    /// The requested operator / AST variant is not supported by the
    /// compiler. Used as the F1 replacement for the two historical
    /// `panic!`s in `build_where()` for `Stats` and `Analyze`.
    #[error("Unsupported operator: {op}")]
    UnsupportedOperator {
        /// Name of the operator (e.g., `"Stats"`, `"Analyze"`, `"Aggregate"`)
        op: &'static str,
    },
    /// Generic compilation failure (reserved for F11 / F3).
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
