//! AST module - domain-independent query tree
//!
//! This module provides the Abstract Syntax Tree (AST) for query expressions.
//! The AST is independent of domain types (no Uuid, JournalDay) and can be
//! reused across different backends.
//!
//! All types derive [`serde::Serialize`]/[`serde::Deserialize`] so the
//! canonical AST can be re-exported by `quilt-core` (WASM) and serialized
//! across the JS boundary via `serde-wasm-bindgen`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export `PropertyOp` from its own module (extracted in F3) so all
// existing import paths (`crate::ast::PropertyOp`, `quilt_query::PropertyOp`)
// continue to work. The canonical enum lives in `crate::property_op`.
pub use crate::property_op::PropertyOp;

/// Sort direction for ordering results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl std::fmt::Display for SortDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortDirection::Asc => write!(f, "asc"),
            SortDirection::Desc => write!(f, "desc"),
        }
    }
}

/// Values that can be used in query expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Date string (YYYY-MM-DD format)
    Date(String),
    /// Time offset (relative time like "-1w" or "+3d")
    TimeOffset(String),
    /// Boolean value
    Boolean(bool),
}

impl std::fmt::Display for QueryValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryValue::String(s) => write!(f, "{}", s),
            QueryValue::Integer(n) => write!(f, "{}", n),
            QueryValue::Date(d) => write!(f, "{}", d),
            QueryValue::TimeOffset(t) => write!(f, "{}", t),
            QueryValue::Boolean(b) => write!(f, "{}", b),
        }
    }
}

impl From<QueryValue> for String {
    fn from(val: QueryValue) -> Self {
        val.to_string()
    }
}

/// Aggregate functions for grouping queries (F2 — moved from `parser`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFn {
    Count,
    Avg,
    Sum,
    Min,
    Max,
}

/// Statistical functions for property queries (F2 — moved from `parser`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatsFn {
    Stddev,
    Variance,
    Median,
    Percentile(u8),
}

/// Analysis kinds for the analyze operator (F2 — moved from `parser`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalyzeKind {
    StructuralMirror,
    Serendipity {
        limit: Option<usize>,
        min_confidence: Option<f32>,
        temporal_window_days: Option<i64>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// G3: TemporalRange — temporal classification for queries
// ─────────────────────────────────────────────────────────────────────────────

/// Time range for temporal classification queries (G3).
///
/// Variants cover common temporal ranges used in knowledge management:
/// - `Today` / `Yesterday`: exact day boundaries
/// - `ThisWeek` / `LastWeek`: week boundaries (week starts Monday per convention)
/// - `ThisMonth` / `LastMonth`: month boundaries
/// - `Custom`: explicit date range
/// - `Relative`: dynamic offset from current time
///
/// # Week Convention
///
/// **Weeks start on Monday** (ISO 8601 standard). This convention is shared
/// between `compile_temporal` and `SqlDialect::temporal_range_sql`. Documented
/// here to prevent hidden meaning connascence (~0.8 bits per spec discovery).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemporalRange {
    /// Today (the current calendar day)
    Today,
    /// Yesterday (one day before today)
    Yesterday,
    /// This week (Monday 00:00 to today)
    ThisWeek,
    /// Last week (Monday 00:00 to Sunday 23:59)
    LastWeek,
    /// This month (day 1 to today)
    ThisMonth,
    /// Last month (day 1 to last day of previous month)
    LastMonth,
    /// Custom date range with explicit start and end dates
    Custom {
        /// Start date (inclusive)
        start: String,
        /// End date (inclusive)
        end: String,
    },
    /// Relative time offset from now
    Relative(crate::time_helpers::TimeOffset),
}

// ─────────────────────────────────────────────────────────────────────────────
// F12: VirtualColumn — SQL-computed column types
// ─────────────────────────────────────────────────────────────────────────────

/// Virtual columns computed from block data at query time (F12).
///
/// These columns are not stored in the database but computed from block content
/// using SQL expressions. Each variant maps to a specific SQL computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VirtualColumn {
    /// Word count: `LENGTH(content) - LENGTH(REPLACE(content, ' ', '')) + 1`
    WordCount,
    /// Character count: `LENGTH(content)`
    CharCount,
    /// Reference count: subquery on refs table
    RefCount,
    /// Block age in days: `CAST(julianday('now') - julianday(created_at) AS INTEGER)`
    BlockAgeDays,
}

/// Date predicate for scheduled/deadline queries (T5).
///
/// Represents the different ways to specify a date in `(scheduled ...)` and
/// `(deadline ...)` predicates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatePredicate {
    /// The current day (today)
    Today,
    /// The next day (tomorrow)
    Tomorrow,
    /// The previous day (yesterday)
    Yesterday,
    /// A relative offset from today (e.g., `-3d`, `+1w`)
    Relative(crate::time_helpers::TimeOffset),
}

/// Abstract Syntax Tree (AST) for query expressions.
///
/// Each variant represents a different query operation that can be
/// performed on the knowledge graph. F2 (Work Unit A) merged the
/// previously separate `parser::QueryExpr` variants (Aggregate, Stats,
/// GroupBy, Analyze) with the original `ast::QueryAst` variants
/// (Table, SortBy, Exists, Missing, Namespace) into a single
/// canonical type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryAst {
    /// Boolean AND of multiple expressions
    And(Vec<QueryAst>),
    /// Boolean OR of multiple expressions
    Or(Vec<QueryAst>),
    /// Boolean NOT of an expression
    Not(Box<QueryAst>),
    /// Range filter between two values
    Between {
        field: String,
        start: QueryValue,
        end: QueryValue,
    },
    /// JSON property filter with optional operator (F3)
    Property {
        key: String,
        op: PropertyOp,
        value: QueryValue,
        value2: Option<QueryValue>,
    },
    /// Task marker filter (e.g., todo, done, now, later, cancelled)
    Task(Vec<String>),
    /// Priority filter (a, b, c)
    Priority(Vec<String>),
    /// Page name filter
    Page(String),
    /// Tag filter
    Tags(String),
    /// Page reference (e.g., `[[Page Name]]`)
    PageRef(String),
    /// Self-reference (current block)
    SelfRef,
    /// Full-text search content
    BlockContent(String),
    /// Random sample of N results
    Sample(usize),
    /// Aggregate with GROUP BY (F2 — moved from `parser::QueryExpr`)
    Aggregate {
        inner: Box<QueryAst>,
        group_by: String,
        aggregate_fn: AggregateFn,
    },
    /// Statistical computation over a property (F2)
    Stats { property: String, compute: StatsFn },
    /// Group by property (no aggregation) (F2)
    GroupBy {
        inner: Box<QueryAst>,
        property: String,
    },
    /// Analysis operator for cognitive/serendipity analysis (F2)
    Analyze {
        inner: Box<QueryAst>,
        kind: AnalyzeKind,
    },
    /// Table view with structured columns from inner expressions
    Table(Vec<QueryAst>),
    /// Sort results by field and direction
    SortBy {
        /// Field name to sort by
        field: String,
        /// Sort direction (ascending or descending)
        direction: SortDirection,
        /// Inner expression to sort
        inner: Box<QueryAst>,
    },
    /// Filter to items where property exists
    Exists(String),
    /// Filter to items where property is missing
    Missing(String),
    /// Filter to items within a namespace
    Namespace(String),

    // ─────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy — fuzzy page name matching
    // ─────────────────────────────────────────────────────────────────────
    /// Fuzzy page name search (G5).
    ///
    /// Uses FTS5 prefix-first matching: try `term*` first, fall back to LIKE `%term%`.
    PageFuzzy {
        /// The search term
        term: String,
        /// Maximum results to return
        limit: usize,
    },

    // ─────────────────────────────────────────────────────────────────────
    // G3: Temporal — temporal classification
    // ─────────────────────────────────────────────────────────────────────
    /// Filter by temporal range (G3).
    ///
    /// Combines a [`TemporalRange`] with an inner expression.
    /// The compiler generates SQL that filters by the temporal range.
    Temporal {
        /// The temporal range to filter by
        range: TemporalRange,
        /// Inner expression to filter
        inner: Box<QueryAst>,
    },

    // ─────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect — virtual column selection
    // ─────────────────────────────────────────────────────────────────────
    /// Select with virtual columns computed at query time (F12).
    ///
    /// Emits SQL with computed columns (word_count, char_count, ref_count, block_age)
    /// alongside the inner query results.
    VirtualSelect {
        /// Virtual columns to compute
        columns: Vec<VirtualColumn>,
        /// Inner expression
        inner: Box<QueryAst>,
    },

    // ─────────────────────────────────────────────────────────────────────────
    // T5: Journal Aggregation Predicates
    // ─────────────────────────────────────────────────────────────────────────

    /// Filter by scheduled date predicate (T5).
    ///
    /// `(scheduled today)`, `(scheduled tomorrow)`, `(scheduled yesterday)`,
    /// `(scheduled -3d)`, etc.
    Scheduled {
        /// The date predicate to match
        predicate: DatePredicate,
    },

    /// Filter by deadline date predicate (T5).
    ///
    /// `(deadline today)`, `(deadline tomorrow)`, `(deadline yesterday)`,
    /// `(deadline +1w)`, etc.
    Deadline {
        /// The date predicate to match
        predicate: DatePredicate,
    },

    /// Filter to overdue blocks (T5).
    ///
    /// `(overdue)` — blocks where `deadline < now` AND `marker NOT IN (done, cancelled)`.
    Overdue,

    /// Filter to in-progress blocks (T5).
    ///
    /// `(in-progress)` — blocks where `marker IN (now, doing)`.
    InProgress,
}

/// Type alias for backward compatibility
#[deprecated(since = "0.1.0", note = "Use QueryAst instead")]
pub type QueryExpr = QueryAst;

/// Errors that can occur during query parsing.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ParseError {
    /// Syntax error in the query string (e.g., unclosed parenthesis)
    Syntax {
        /// Error message
        msg: String,
        /// Line number (1-indexed)
        line: usize,
        /// Column number (1-indexed)
        col: usize,
        /// Optional hint for recovery
        hint: Option<String>,
    },
    /// The query is syntactically valid but semantically invalid
    Invalid(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Syntax {
                msg,
                line,
                col,
                hint,
            } => {
                write!(f, "Syntax error at {}:{}: {}", line, col, msg)?;
                if let Some(h) = hint {
                    write!(f, " ({})", h)?;
                }
                Ok(())
            }
            ParseError::Invalid(s) => write!(f, "Invalid query: {}", s),
        }
    }
}
