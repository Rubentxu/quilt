//! Merge contract for `dsl-aggregates` and `dsl-analyze` extensions.
//!
//! F11 (Work Unit C) — `QueryCompiler` is the single SQL-generation
//! entry point. Downstream crates extend it by overriding the
//! extension hooks. This module documents that contract so both
//! branches can be reviewed against the same expectations.
//!
//! # Merge order (Q1 — critical)
//!
//! 1. `query-refactor-v1` merges into `main` FIRST.
//! 2. `dsl-aggregates` rebases onto `query-refactor-v1` tip, overrides
//!    [`crate::compiler::QueryCompiler::compile_aggregate`].
//! 3. `dsl-analyze` rebases onto `query-refactor-v1` tip, overrides
//!    [`crate::compiler::QueryCompiler::compile_stats`] and
//!    [`crate::compiler::QueryCompiler::compile_analyze`].
//!
//! Both `dsl-*` branches MUST NOT merge into `main` until they've
//! rebased onto `query-refactor-v1` tip. This prevents the "3 methods
//! move to trait" conflict on the original `build_sql` arms.
//!
//! # Contract
//!
//! - All `compile_*` methods MUST be `Result`-returning. They MUST NOT
//!   panic. Implementors MAY delegate to other compilers via composition.
//! - Default implementations return `Err(UnsupportedOperator { op })`.
//! - `compile_where` is the building block for ordinary expressions.
//!   Extension impls that want to chain the default behaviour should
//!   call `SqliteCompiler::compile_where` (or the equivalent for
//!   their dialect) before adding their own logic.
