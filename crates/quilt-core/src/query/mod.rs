//! Query DSL — re-exported from `quilt-query`
//!
//! This module re-exports the canonical query AST, parser, and error types
//! from [`quilt_query`] so that WASM consumers (`quilt-core`) and native
//! consumers share a single source of truth. The previous duplicated
//! `ast.rs` and `parser.rs` modules have been deleted to eliminate drift.
//!
//! # Architecture
//!
//! - Canonical location: `quilt_query::ast` (types) and `quilt_query::parser`
//!   (parser). This module only re-exports.
//!
//! # Quick start
//!
//! ```
//! use quilt_core::query::QueryParser;
//!
//! let parser = QueryParser;
//! let expr = parser.parse("(task todo)").unwrap();
//! ```

pub use quilt_query::ast::{PropertyOp, QueryAst, QueryValue, SortDirection};
pub use quilt_query::parser::{AggregateFn, AnalyzeKind, ParseError, QueryParser, StatsFn};

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use quilt_query::{
        AggregateFn, AnalyzeKind, ParseError, PropertyOp, QueryAst, QueryValue, StatsFn,
    };

    /// S2.1 — `quilt_core::query::QueryAst` MUST be the same type as
    /// `quilt_query::QueryAst`. The previous duplicated ASTs have been
    /// collapsed into a single canonical type in `quilt-query`.
    #[test]
    fn test_query_ast_type_identity() {
        assert_eq!(
            TypeId::of::<crate::query::QueryAst>(),
            TypeId::of::<QueryAst>(),
            "quilt_core::query::QueryAst must be the canonical quilt_query::QueryAst"
        );
    }

    /// All companion types MUST also be type-identical across crates.
    #[test]
    fn test_companion_type_identity() {
        assert_eq!(
            TypeId::of::<crate::query::QueryValue>(),
            TypeId::of::<QueryValue>()
        );
        assert_eq!(
            TypeId::of::<crate::query::ParseError>(),
            TypeId::of::<ParseError>()
        );
        assert_eq!(
            TypeId::of::<crate::query::PropertyOp>(),
            TypeId::of::<PropertyOp>()
        );
        assert_eq!(
            TypeId::of::<crate::query::AggregateFn>(),
            TypeId::of::<AggregateFn>()
        );
        assert_eq!(
            TypeId::of::<crate::query::StatsFn>(),
            TypeId::of::<StatsFn>()
        );
        assert_eq!(
            TypeId::of::<crate::query::AnalyzeKind>(),
            TypeId::of::<AnalyzeKind>()
        );
    }
}
