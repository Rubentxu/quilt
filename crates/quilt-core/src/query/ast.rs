//! AST types for the Quilt Query DSL
//!
//! This module provides the Abstract Syntax Tree (AST) types used by the
//! query parser. All types derive [`serde::Serialize`] and [`serde::Deserialize`]
//! for WASM boundary passing and JSON serialization.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during query parsing.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum ParseError {
    /// Syntax error in the query string (e.g., unclosed parenthesis)
    #[error("Syntax error: {0}")]
    Syntax(String),
    /// The query is syntactically valid but semantically invalid
    #[error("Invalid query: {0}")]
    Invalid(String),
}

/// Abstract Syntax Tree (AST) for query expressions.
///
/// Each variant represents a different query operation that can be
/// performed on the knowledge graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryExpr {
    /// Boolean AND of multiple expressions
    And(Vec<QueryExpr>),
    /// Boolean OR of multiple expressions
    Or(Vec<QueryExpr>),
    /// Boolean NOT of an expression
    Not(Box<QueryExpr>),
    /// Range filter between two values
    Between {
        field: String,
        start: QueryValue,
        end: QueryValue,
    },
    /// JSON property filter
    Property { key: String, value: QueryValue },
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
    /// Aggregate with GROUP BY
    Aggregate {
        inner: Box<QueryExpr>,
        group_by: String,
        aggregate_fn: AggregateFn,
    },
    /// Statistical computation over a property
    Stats { property: String, compute: StatsFn },
    /// Group by property (no aggregation)
    GroupBy {
        inner: Box<QueryExpr>,
        property: String,
    },
    /// Analysis operator for cognitive/serendipity analysis
    Analyze {
        inner: Box<QueryExpr>,
        kind: AnalyzeKind,
    },
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

/// Aggregate functions for grouping queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFn {
    Count,
    Avg,
    Sum,
    Min,
    Max,
}

/// Statistical functions for property queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatsFn {
    Stddev,
    Variance,
    Median,
    Percentile(u8),
}

/// Analysis kinds for the analyze operator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnalyzeKind {
    StructuralMirror,
    Serendipity {
        limit: Option<usize>,
        min_confidence: Option<f32>,
        temporal_window_days: Option<i64>,
    },
}
