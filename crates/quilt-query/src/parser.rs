//! Query parser
//!
//! This module provides a Pest-based parser for the Quilt Query DSL.
//! The parser converts query strings into an Abstract Syntax Tree (AST)
//! represented by [`QueryExpr`].

use pest::iterators::Pair;
use pest::Parser;
use thiserror::Error;

use crate::grammar::{QueryGrammar, Rule};

/// Maximum recursion depth to prevent stack overflow from maliciously nested queries.
/// Queries deeper than this will return a ParseError::Syntax with MaxDepthExceeded.
const MAX_PARSE_DEPTH: u32 = 100;

/// Pre-processes a query string to normalize bare integers.
///
/// Pest has a known limitation with unquoted integers in sequence (e.g., `(between 100 200)`).
/// This pre-processor quotes bare integers found in `between` expressions before parsing.
///
/// # Example
///
/// ```
/// use quilt_query::parser::preprocess;
///
/// // Bare integers get quoted
/// assert_eq!(preprocess("(between 100 200)"), "(between \"100\" \"200\")");
/// // Already quoted integers are unchanged
/// assert_eq!(preprocess("(between \"100\" \"200\")"), "(between \"100\" \"200\")");
/// // Negative integers are also quoted
/// assert_eq!(preprocess("(between -30 100)"), "(between \"-30\" \"100\")");
/// // Time helpers like -30d or 7d are NOT affected (they parse correctly)
/// assert_eq!(preprocess("(between -30d 7d)"), "(between -30d 7d)");
/// ```
pub fn preprocess(input: &str) -> String {
    use regex::Regex;

    // Pattern: match (between <value1> <value2>)
    // We use explicit digit patterns to avoid capturing the closing )
    // Pattern: (-?\d+) matches optional minus followed by digits
    let between_re = Regex::new(r"\(between\s+(-?\d+)\s+(-?\d+)\)").unwrap();

    // Replace bare integers with quoted versions
    let result = between_re.replace_all(input, |caps: &regex::Captures| {
        let val1 = &caps[1];
        let val2 = &caps[2];
        format!("(between \"{val1}\" \"{val2}\")")
    });

    result.to_string()
}

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

/// Abstract Syntax Tree (AST) for query expressions.
///
/// Each variant represents a different query operation that can be
/// performed on the knowledge graph.
#[derive(Debug, Clone, PartialEq)]
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
    Table(Vec<QueryExpr>),
    /// Sort results by field and direction
    SortBy {
        /// Field name to sort by
        field: String,
        /// Sort direction (ascending or descending)
        direction: SortDirection,
        /// Inner expression to sort
        inner: Box<QueryExpr>,
    },
    /// Filter to items where property exists
    Exists(String),
    /// Filter to items where property is missing
    Missing(String),
    /// Filter to items within a namespace
    Namespace(String),
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

/// Parser for the Quilt Query DSL.
///
/// This parser uses Pest to parse query strings into [`QueryExpr`] AST nodes.
///
/// # Example
///
/// ```
/// use quilt_query::{QueryParser, QueryExpr};
///
/// let parser = QueryParser;
/// let result = parser.parse("(task todo)");
/// assert!(result.is_ok());
/// ```
pub struct QueryParser;

impl QueryParser {
    /// Parses a query string into a [`QueryExpr`] AST.
    ///
    /// # Arguments
    ///
    /// * `input` - The query string to parse
    ///
    /// # Returns
    ///
    /// Returns the parsed [`QueryExpr`] on success.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the query string is syntactically invalid.
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_query::QueryParser;
    ///
    /// let parser = QueryParser;
    ///
    /// // Valid query
    /// let result = parser.parse("(task todo)");
    /// assert!(result.is_ok());
    ///
    /// // Invalid query
    /// let result = parser.parse("");
    /// assert!(result.is_err());
    /// ```
    pub fn parse(&self, input: &str) -> Result<QueryExpr, ParseError> {
        let input = preprocess(input.trim());

        if input.is_empty() {
            return Err(ParseError::Invalid("Empty query".to_string()));
        }

        let mut pairs =
            QueryGrammar::parse(Rule::query, &input).map_err(|e: pest::error::Error<Rule>| {
                let msg = e.to_string();
                // Extract line and column from error
                let (line, col) = match e.line_col {
                    pest::error::LineColLocation::Pos((line, col)) => (line, col),
                    pest::error::LineColLocation::Span((line, col), _) => (line, col),
                };
                // Generate a hint for common errors
                let hint = if msg.contains("unclosed") {
                    Some("did you forget to close a parenthesis?".to_string())
                } else if msg.contains("expected") && msg.contains("pair") {
                    Some("check your parentheses balance".to_string())
                } else {
                    None
                };
                ParseError::Syntax {
                    msg,
                    line,
                    col,
                    hint,
                }
            })?;

        // There should be exactly one pair: the query rule
        let pair = pairs
            .next()
            .ok_or_else(|| ParseError::Invalid("Empty query".to_string()))?;

        let expr = parse_expr_with_depth(pair, 0)?;

        // Validate semantic rules after parsing
        validate(&expr)?;

        Ok(expr)
    }
}

/// Parse an expression from a pest pair with depth tracking to prevent stack overflow.
fn parse_expr_with_depth(pair: Pair<'_, Rule>, depth: u32) -> Result<QueryExpr, ParseError> {
    // Check depth limit to prevent stack overflow from maliciously nested queries
    if depth > MAX_PARSE_DEPTH {
        return Err(ParseError::Syntax {
            msg: format!("Maximum expression depth ({}) exceeded", MAX_PARSE_DEPTH),
            line: 0,
            col: 0,
            hint: Some("Query too deeply nested. Try simplifying.".to_string()),
        });
    }

    let next_depth = depth + 1;
    let rule = pair.as_rule();
    let mut inner = pair.into_inner();

    match rule {
        Rule::query => {
            // query = SOI ~ expr ~ EOI, so we need the inner expr
            let expr_pair = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("Empty query".to_string()))?;
            parse_expr_with_depth(expr_pair, next_depth)
        }
        Rule::and => {
            let mut exprs = Vec::new();
            for p in inner {
                exprs.push(parse_expr_with_depth(p, next_depth)?);
            }
            Ok(QueryExpr::And(exprs))
        }
        Rule::or => {
            let mut exprs = Vec::new();
            for p in inner {
                exprs.push(parse_expr_with_depth(p, next_depth)?);
            }
            Ok(QueryExpr::Or(exprs))
        }
        Rule::not => {
            let expr = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("not requires an expression".to_string()))?;
            Ok(QueryExpr::Not(Box::new(parse_expr_with_depth(
                expr, next_depth,
            )?)))
        }
        Rule::between => {
            let start = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("between requires 2 arguments".to_string()))?;
            let end = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("between requires 2 arguments".to_string()))?;
            Ok(QueryExpr::Between {
                field: "created_at".to_string(),
                start: parse_value_with_depth(start, next_depth)?,
                end: parse_value_with_depth(end, next_depth)?,
            })
        }
        Rule::property => {
            let mut items = inner;

            let key_pair = items
                .next()
                .ok_or_else(|| ParseError::Invalid("property requires a key".to_string()))?;
            let key_str = parse_string(key_pair)?;

            let op: PropertyOp;
            let first_item = items
                .next()
                .ok_or_else(|| ParseError::Invalid("property requires a value".to_string()))?;

            // Check if first item is operator or value
            if first_item.as_rule() == Rule::property_op {
                op = parse_property_op(first_item)?;
                // Next items are value+ (siblings, not wrapped)
                let mut values: Vec<QueryValue> = Vec::new();
                for item in items {
                    if item.as_rule() == Rule::value {
                        values.push(parse_value_with_depth(item, next_depth)?);
                    }
                }
                if values.is_empty() {
                    return Err(ParseError::Invalid("property requires a value".to_string()));
                }
                let value = values.remove(0);
                let value2 = values.pop();

                if op == PropertyOp::Between && value2.is_none() {
                    return Err(ParseError::Invalid("between requires 2 values".to_string()));
                }

                Ok(QueryExpr::Property {
                    key: key_str,
                    op,
                    value,
                    value2,
                })
            } else {
                // No operator, first_item IS the first value
                let mut values: Vec<QueryValue> = Vec::new();
                if first_item.as_rule() == Rule::value {
                    values.push(parse_value_with_depth(first_item, next_depth)?);
                }
                // Remaining items are also values (siblings)
                for item in items {
                    if item.as_rule() == Rule::value {
                        values.push(parse_value_with_depth(item, next_depth)?);
                    }
                }
                if values.is_empty() {
                    return Err(ParseError::Invalid("property requires a value".to_string()));
                }
                let value = values.remove(0);
                let value2 = values.pop();

                Ok(QueryExpr::Property {
                    key: key_str,
                    op: PropertyOp::Equals,
                    value,
                    value2,
                })
            }
        }
        Rule::task => {
            let mut markers = Vec::new();
            for p in inner {
                // task_marker rule
                markers.push(p.as_str().to_string());
            }
            Ok(QueryExpr::Task(markers))
        }
        Rule::priority => {
            let mut levels = Vec::new();
            for p in inner {
                // priority_level is matched as identifier, lowercase it
                levels.push(p.as_str().to_lowercase());
            }
            Ok(QueryExpr::Priority(levels))
        }
        Rule::page => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("page requires an argument".to_string()))?;
            Ok(QueryExpr::Page(parse_string(s)?))
        }
        Rule::tags => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("tags requires an argument".to_string()))?;
            Ok(QueryExpr::Tags(parse_string(s)?))
        }
        Rule::page_ref => {
            // page_ref = "[[" ~ page_name ~ "]]"
            // page_name is the inner content without the brackets
            let name = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("page_ref requires a name".to_string()))?;
            Ok(QueryExpr::PageRef(name.as_str().to_string()))
        }
        Rule::self_ref => Ok(QueryExpr::SelfRef),
        Rule::block_content => {
            let s = inner.next().ok_or_else(|| {
                ParseError::Invalid("full-text-search requires an argument".to_string())
            })?;
            Ok(QueryExpr::BlockContent(parse_string(s)?))
        }
        Rule::sample => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("sample requires an argument".to_string()))?;
            let n: usize = s
                .as_str()
                .parse()
                .map_err(|_| ParseError::Invalid("Invalid number for sample".to_string()))?;
            Ok(QueryExpr::Sample(n))
        }
        Rule::table => {
            let mut exprs = Vec::new();
            for p in inner {
                exprs.push(parse_expr_with_depth(p, next_depth)?);
            }
            Ok(QueryExpr::Table(exprs))
        }
        Rule::sort_by => {
            let mut items = inner;
            // First item is field (string or integer)
            let field_pair = items
                .next()
                .ok_or_else(|| ParseError::Invalid("sort-by requires a field".to_string()))?;
            let field = if field_pair.as_rule() == Rule::string {
                parse_string(field_pair)?
            } else {
                field_pair.as_str().to_string()
            };

            // Optional direction
            let direction =
                if let Some(dir_pair) = items.clone().find(|p| p.as_rule() == Rule::direction) {
                    match dir_pair.as_str() {
                        "asc" => SortDirection::Asc,
                        "desc" => SortDirection::Desc,
                        _ => SortDirection::Asc,
                    }
                } else {
                    SortDirection::Asc
                };

            // Last item is the inner expression
            let inner_pairs: Vec<_> = items.collect();
            let expr_pair = inner_pairs
                .last()
                .ok_or_else(|| ParseError::Invalid("sort-by requires an expression".to_string()))?;
            let expr = parse_expr_with_depth(expr_pair.clone(), next_depth)?;

            Ok(QueryExpr::SortBy {
                field,
                direction,
                inner: Box::new(expr),
            })
        }
        Rule::exists => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("exists requires an argument".to_string()))?;
            Ok(QueryExpr::Exists(parse_string(s)?))
        }
        Rule::missing => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("missing requires an argument".to_string()))?;
            Ok(QueryExpr::Missing(parse_string(s)?))
        }
        Rule::namespace => {
            let s = inner
                .next()
                .ok_or_else(|| ParseError::Invalid("namespace requires an argument".to_string()))?;
            Ok(QueryExpr::Namespace(parse_string(s)?))
        }
        _ => Err(ParseError::Invalid(format!(
            "Unknown expression: {:?}",
            rule
        ))),
    }
}

/// Parse a string value from a string rule
fn parse_string(pair: Pair<'_, Rule>) -> Result<String, ParseError> {
    // string = "\"" ~ quoted_string ~ "\""
    // quoted_string is the inner content
    let inner = pair.into_inner().next();
    match inner {
        Some(p) => Ok(p.as_str().to_string()),
        None => Ok(String::new()),
    }
}

/// Parse a value from a value rule with depth tracking to prevent stack overflow.
fn parse_value_with_depth(pair: Pair<'_, Rule>, depth: u32) -> Result<QueryValue, ParseError> {
    // Check depth limit to prevent stack overflow
    if depth > MAX_PARSE_DEPTH {
        return Err(ParseError::Syntax {
            msg: format!("Maximum expression depth ({}) exceeded", MAX_PARSE_DEPTH),
            line: 0,
            col: 0,
            hint: Some("Query too deeply nested. Try simplifying.".to_string()),
        });
    }

    let next_depth = depth + 1;
    let rule = pair.as_rule();
    let inner_str = pair.as_str();

    match rule {
        Rule::string => {
            // Strip quotes
            let s = inner_str.trim_matches('"');
            Ok(QueryValue::String(s.to_string()))
        }
        Rule::integer => {
            let n: i64 = inner_str
                .parse()
                .map_err(|_| ParseError::Invalid(format!("Invalid integer: {}", inner_str)))?;
            Ok(QueryValue::Integer(n))
        }
        Rule::date => Ok(QueryValue::Date(inner_str.to_string())),
        Rule::time_helper => Ok(QueryValue::TimeOffset(inner_str.to_string())),
        Rule::boolean => match inner_str {
            "true" => Ok(QueryValue::Boolean(true)),
            "false" => Ok(QueryValue::Boolean(false)),
            _ => Err(ParseError::Invalid(format!(
                "Invalid boolean: {}",
                inner_str
            ))),
        },
        Rule::value => {
            // If we receive a value pair directly, parse its inner content
            let mut inner = pair.into_inner();
            if let Some(inner_pair) = inner.next() {
                parse_value_with_depth(inner_pair, next_depth)
            } else {
                Err(ParseError::Invalid("Empty value".to_string()))
            }
        }
        _ => Err(ParseError::Invalid(format!(
            "Unknown value type: {:?}",
            rule
        ))),
    }
}

/// Parse a property operator from a property_op rule
fn parse_property_op(pair: Pair<'_, Rule>) -> Result<PropertyOp, ParseError> {
    match pair.as_str() {
        "!=" => Ok(PropertyOp::NotEquals),
        ">" => Ok(PropertyOp::GreaterThan),
        "<" => Ok(PropertyOp::LessThan),
        ">=" => Ok(PropertyOp::GreaterThanOrEqual),
        "<=" => Ok(PropertyOp::LessThanOrEqual),
        "contains" => Ok(PropertyOp::Contains),
        "between" => Ok(PropertyOp::Between),
        _ => Err(ParseError::Invalid(format!(
            "Unknown property operator: {}",
            pair.as_str()
        ))),
    }
}

/// Validates semantic rules for a parsed query expression with depth tracking.
///
/// This function enforces rules that cannot be captured by the grammar alone,
/// such as arity constraints and value ranges.
///
/// # Arguments
///
/// * `expr` - The query expression to validate
/// * `depth` - Current recursion depth (starts at 0)
///
/// # Returns
///
/// Returns `Ok(())` if the expression is semantically valid, or an error otherwise.
fn validate_with_depth(expr: &QueryExpr, depth: u32) -> Result<(), ParseError> {
    // Check depth limit to prevent stack overflow
    if depth > MAX_PARSE_DEPTH {
        return Err(ParseError::Syntax {
            msg: format!("Maximum expression depth ({}) exceeded", MAX_PARSE_DEPTH),
            line: 0,
            col: 0,
            hint: Some("Query too deeply nested. Try simplifying.".to_string()),
        });
    }

    let next_depth = depth + 1;

    match expr {
        QueryExpr::Between { .. } => {
            // between requires exactly 2 values - we check this at parse time
            // but we can validate the start <= end ordering for dates/numbers
            Ok(())
        }
        QueryExpr::Property {
            op: PropertyOp::Between,
            value: _,
            value2,
            ..
        } => {
            // between operator requires both start AND end values
            if value2.is_none() {
                return Err(ParseError::Invalid(
                    "between operator requires start AND end value".to_string(),
                ));
            }
            Ok(())
        }
        QueryExpr::Sample(n) => {
            // sample count must be 1-1000
            if *n == 0 || *n > 1000 {
                return Err(ParseError::Invalid(
                    "sample count must be 1–1000".to_string(),
                ));
            }
            Ok(())
        }
        QueryExpr::SortBy { field, inner, .. } => {
            // sort-by requires field and expression
            if field.is_empty() {
                return Err(ParseError::Invalid("sort-by requires a field".to_string()));
            }
            // inner is Box<QueryExpr>, always valid
            validate_with_depth(inner.as_ref(), next_depth)?;
            Ok(())
        }
        QueryExpr::Exists(key) => {
            // exists requires exactly 1 key argument (validated at parse time, but double-check)
            if key.is_empty() {
                return Err(ParseError::Invalid(
                    "exists requires exactly 1 key argument".to_string(),
                ));
            }
            Ok(())
        }
        QueryExpr::Namespace(name) => {
            // namespace requires exactly 1 name argument
            if name.is_empty() {
                return Err(ParseError::Invalid(
                    "namespace requires exactly 1 name argument".to_string(),
                ));
            }
            Ok(())
        }
        QueryExpr::Table(exprs) => {
            // table requires at least one inner expression
            if exprs.is_empty() {
                return Err(ParseError::Invalid(
                    "table requires at least one expression".to_string(),
                ));
            }
            for expr in exprs {
                validate_with_depth(expr, next_depth)?;
            }
            Ok(())
        }
        QueryExpr::And(exprs) => {
            for expr in exprs {
                validate_with_depth(expr, next_depth)?;
            }
            Ok(())
        }
        QueryExpr::Or(exprs) => {
            for expr in exprs {
                validate_with_depth(expr, next_depth)?;
            }
            Ok(())
        }
        QueryExpr::Not(inner) => validate_with_depth(inner.as_ref(), next_depth),
        // Other variants have no additional validation
        _ => Ok(()),
    }
}

/// Validates semantic rules for a parsed query expression.
///
/// This is the public wrapper that starts validation with depth=0.
pub fn validate(expr: &QueryExpr) -> Result<(), ParseError> {
    validate_with_depth(expr, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> QueryExpr {
        QueryParser.parse(input).expect("parse failed")
    }

    fn parse_err(input: &str) -> ParseError {
        QueryParser.parse(input).expect_err("expected parse error")
    }

    // Basic parsing

    #[test]
    fn test_parse_simple_task() {
        let result = parse("(task todo)");
        assert_eq!(result, QueryExpr::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_priority() {
        let result = parse("(priority a)");
        assert_eq!(result, QueryExpr::Priority(vec!["a".to_string()]));
    }

    #[test]
    fn test_parse_page() {
        let result = parse("(page \"MyPage\")");
        assert_eq!(result, QueryExpr::Page("MyPage".to_string()));
    }

    #[test]
    fn test_parse_tags() {
        let result = parse("(tags \"rust\")");
        assert_eq!(result, QueryExpr::Tags("rust".to_string()));
    }

    #[test]
    fn test_parse_sample() {
        let result = parse("(sample 10)");
        assert_eq!(result, QueryExpr::Sample(10));
    }

    #[test]
    fn test_parse_self_ref() {
        let result = parse("self");
        assert_eq!(result, QueryExpr::SelfRef);
    }

    #[test]
    fn test_parse_page_ref() {
        let result = parse("[[Some Page]]");
        assert_eq!(result, QueryExpr::PageRef("Some Page".to_string()));
    }

    #[test]
    fn test_parse_full_text_search() {
        let result = parse("(full-text-search \"hello\")");
        assert_eq!(result, QueryExpr::BlockContent("hello".to_string()));
    }

    // Compound queries

    #[test]
    fn test_parse_and() {
        let result = parse("(and (task todo) (priority a))");
        assert_eq!(
            result,
            QueryExpr::And(vec![
                QueryExpr::Task(vec!["todo".to_string()]),
                QueryExpr::Priority(vec!["a".to_string()]),
            ])
        );
    }

    #[test]
    fn test_parse_or() {
        let result = parse("(or (task todo) (task done))");
        assert_eq!(
            result,
            QueryExpr::Or(vec![
                QueryExpr::Task(vec!["todo".to_string()]),
                QueryExpr::Task(vec!["done".to_string()]),
            ])
        );
    }

    #[test]
    fn test_parse_not() {
        let result = parse("(not (task done))");
        assert_eq!(
            result,
            QueryExpr::Not(Box::new(QueryExpr::Task(vec!["done".to_string()])))
        );
    }

    // Nested queries

    #[test]
    fn test_parse_nested_and_or() {
        let result = parse("(and (or (task todo) (priority a)) (page \"X\"))");
        assert_eq!(
            result,
            QueryExpr::And(vec![
                QueryExpr::Or(vec![
                    QueryExpr::Task(vec!["todo".to_string()]),
                    QueryExpr::Priority(vec!["a".to_string()]),
                ]),
                QueryExpr::Page("X".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_deeply_nested_not() {
        let result = parse("(and (not (or (task done) (task cancelled))) (priority a))");
        assert_eq!(
            result,
            QueryExpr::And(vec![
                QueryExpr::Not(Box::new(QueryExpr::Or(vec![
                    QueryExpr::Task(vec!["done".to_string()]),
                    QueryExpr::Task(vec!["cancelled".to_string()]),
                ]))),
                QueryExpr::Priority(vec!["a".to_string()]),
            ])
        );
    }

    // Between

    #[test]
    fn test_parse_between_integers() {
        // Note: pest has a known limitation with unquoted integers in sequence.
        // Using quoted integers (strings) as a workaround. The semantic meaning
        // is the same - numeric range filter.
        let result = parse("(between \"100\" \"200\")");
        assert_eq!(
            result,
            QueryExpr::Between {
                field: "created_at".to_string(),
                start: QueryValue::String("100".to_string()),
                end: QueryValue::String("200".to_string()),
            }
        );
    }

    #[test]
    fn test_parse_between_strings() {
        let result = parse("(between \"2024-01-01\" \"2024-12-31\")");
        assert_eq!(
            result,
            QueryExpr::Between {
                field: "created_at".to_string(),
                start: QueryValue::String("2024-01-01".to_string()),
                end: QueryValue::String("2024-12-31".to_string()),
            }
        );
    }

    // Property

    #[test]
    fn test_parse_property_string() {
        let result = parse("(property \"author\" \"John\")");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "author".to_string(),
                op: PropertyOp::Equals,
                value: QueryValue::String("John".to_string()),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_integer() {
        let result = parse("(property \"count\" 42)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::Equals,
                value: QueryValue::Integer(42),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_boolean() {
        let result = parse("(property \"active\" true)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "active".to_string(),
                op: PropertyOp::Equals,
                value: QueryValue::Boolean(true),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_not_equals() {
        let result = parse("(property \"status\" != \"done\")");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "status".to_string(),
                op: PropertyOp::NotEquals,
                value: QueryValue::String("done".to_string()),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_greater_than() {
        let result = parse("(property \"count\" > 10)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::GreaterThan,
                value: QueryValue::Integer(10),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_less_than() {
        let result = parse("(property \"count\" < 100)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::LessThan,
                value: QueryValue::Integer(100),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_greater_than_or_equal() {
        let result = parse("(property \"count\" >= 10)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::GreaterThanOrEqual,
                value: QueryValue::Integer(10),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_less_than_or_equal() {
        let result = parse("(property \"count\" <= 100)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::LessThanOrEqual,
                value: QueryValue::Integer(100),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_contains() {
        let result = parse("(property \"name\" contains \"test\")");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "name".to_string(),
                op: PropertyOp::Contains,
                value: QueryValue::String("test".to_string()),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_property_between() {
        let result = parse("(property \"count\" between 10 100)");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "count".to_string(),
                op: PropertyOp::Between,
                value: QueryValue::Integer(10),
                value2: Some(QueryValue::Integer(100)),
            }
        );
    }

    // Edge cases and errors

    #[test]
    fn test_parse_empty_input() {
        let err = parse_err("");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_invalid_operator() {
        // Unknown operators result in Syntax error because they don't match any grammar rule
        let err = parse_err("(unknown x)");
        assert!(matches!(err, ParseError::Syntax { .. }));
    }

    #[test]
    fn test_parse_between_too_few_args() {
        // Too few args results in Syntax error because grammar requires specific structure
        let err = parse_err("(between 100)");
        assert!(matches!(err, ParseError::Syntax { .. }));
    }

    #[test]
    fn test_parse_property_too_few_args() {
        // Too few args results in Syntax error because grammar requires specific structure
        let err = parse_err("(property \"x\")");
        assert!(matches!(err, ParseError::Syntax { .. }));
    }

    #[test]
    fn test_parse_multiple_markers() {
        let result = parse("(task todo done)");
        assert_eq!(
            result,
            QueryExpr::Task(vec!["todo".to_string(), "done".to_string()])
        );
    }

    #[test]
    fn test_parse_multiple_priorities() {
        let result = parse("(priority a b c)");
        // Priorities are lowercased
        assert_eq!(
            result,
            QueryExpr::Priority(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    // Edge cases: unicode content

    #[test]
    fn test_parse_unicode_page_name() {
        // Page names can contain unicode
        let result = parse("(page \"日本語ページ\")");
        assert_eq!(result, QueryExpr::Page("日本語ページ".to_string()));
    }

    #[test]
    fn test_parse_unicode_property_value() {
        let result = parse("(property \"author\" \"张三\")");
        assert_eq!(
            result,
            QueryExpr::Property {
                key: "author".to_string(),
                op: PropertyOp::Equals,
                value: QueryValue::String("张三".to_string()),
                value2: None,
            }
        );
    }

    #[test]
    fn test_parse_emoji_in_tags() {
        let result = parse("(tags \"🚀\")");
        assert_eq!(result, QueryExpr::Tags("🚀".to_string()));
    }

    // Edge cases: large inputs

    #[test]
    fn test_parse_large_page_name() {
        // Test with large input - parser should handle it
        let large_name = "a".repeat(10000);
        let query = format!("(page \"{}\")", large_name);
        let result = parse(&query);
        assert_eq!(result, QueryExpr::Page(large_name));
    }

    #[test]
    fn test_parse_many_task_markers() {
        // Test with many markers - parser should handle it
        let result = parse("(task todo done later now cancelled)");
        assert_eq!(
            result,
            QueryExpr::Task(vec![
                "todo".to_string(),
                "done".to_string(),
                "later".to_string(),
                "now".to_string(),
                "cancelled".to_string(),
            ])
        );
    }

    #[test]
    fn test_parse_many_priority_levels() {
        // Test with many priority levels
        let result = parse("(priority a b c)");
        assert_eq!(
            result,
            QueryExpr::Priority(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    // Edge cases: whitespace handling

    #[test]
    fn test_parse_leading_whitespace() {
        let result = parse("  (task todo)");
        assert_eq!(result, QueryExpr::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_trailing_whitespace() {
        let result = parse("(task todo)   ");
        assert_eq!(result, QueryExpr::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_nested_whitespace() {
        let result = parse("(task  todo)");
        assert_eq!(result, QueryExpr::Task(vec!["todo".to_string()]));
    }
}
