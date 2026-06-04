//! Property operator enum — single canonical source of truth.
//!
//! Extracted from `crate::ast` in F3 (Work Unit C) so it can be shared
//! between the parser (produces `PropertyOp`) and the dialect trait
//! (generates SQL fragments). All 8 operators are wired and reachable.

use serde::{Deserialize, Serialize};

/// Operators available for property comparisons.
///
/// Each variant corresponds to a distinct SQL fragment emitted by
/// [`crate::dialect::SqlDialect::property_op_sql`]. Parser produces
/// this enum via `(property "k" <op> <value>)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropertyOp {
    /// Equality — `(property "k" "v")` is the default 2-arg form.
    Equals,
    /// Not equal — `(property "k" != "v")`.
    NotEquals,
    /// Substring — `(property "k" contains "v")`. The value is wrapped
    /// as `%v%` at compile time (not in the dialect).
    Contains,
    /// Greater than — `(property "k" > 5)`.
    GreaterThan,
    /// Less than — `(property "k" < 5)`.
    LessThan,
    /// Greater than or equal — `(property "k" >= 5)`.
    GreaterThanOrEqual,
    /// Less than or equal — `(property "k" <= 5)`.
    LessThanOrEqual,
    /// Range — `(property "k" 1 10)`. The `value2` carries the upper
    /// bound; the SQL fragment is `BETWEEN ? AND ?`.
    Between,
}
