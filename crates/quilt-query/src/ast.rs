//! AST module - domain-independent query tree
//!
//! This module provides the Abstract Syntax Tree (AST) for query expressions.
//! The AST is independent of domain types (no Uuid, JournalDay) and can be
//! reused across different backends.

use thiserror::Error;

/// Operators available for property queries.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyOp {
    /// Equality (default when no operator is specified)
    Equals,
    /// Not equals
    NotEquals,
    /// String contains substring
    Contains,
    /// Greater than comparison
    GreaterThan,
    /// Less than comparison
    LessThan,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Less than or equal
    LessThanOrEqual,
    /// Range between two values (requires value2)
    Between,
}

/// Sort direction for ordering results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq)]
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

/// Abstract Syntax Tree (AST) for query expressions.
///
/// Each variant represents a different query operation that can be
/// performed on the knowledge graph.
#[derive(Debug, Clone, PartialEq)]
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
    /// JSON property filter with optional operator
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
}

/// Type alias for backward compatibility
#[deprecated(since = "0.1.0", note = "Use QueryAst instead")]
pub type QueryExpr = QueryAst;

/// Errors that can occur during query parsing.
#[derive(Debug, Error)]
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
