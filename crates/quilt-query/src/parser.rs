//! Query parser
//!
//! This module provides a recursive descent parser for the Quilt Query DSL.
//! The parser converts query strings into an Abstract Syntax Tree (AST)
//! represented by [`crate::ast::QueryAst`].

use thiserror::Error;

// Re-export the canonical AST and companion types from `crate::ast`.
// The parser produces the same types, so the public API is unchanged
// for existing callers.
pub use crate::ast::{
    AggregateFn, AnalyzeKind, QueryAst, QueryValue, SortDirection, StatsFn,
};
// Re-export `PropertyOp` (F3) — the parser uses it in `parse_property`
// to record the operator.
pub use crate::property_op::PropertyOp;
// `QueryExpr` is preserved as a backward-compatible alias.
#[deprecated(since = "0.1.0", note = "Use QueryAst instead")]
pub use crate::ast::QueryExpr;

/// Errors that can occur during query parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Syntax error in the query string (e.g., unclosed parenthesis)
    #[error("Syntax error: {0}")]
    Syntax(String),
    /// The query is syntactically valid but semantically invalid
    #[error("Invalid query: {0}")]
    Invalid(String),
}

/// Errors that can occur during query execution.
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("Syntax error: {0}")]
    Syntax(String),
    #[error("Invalid query: {0}")]
    Invalid(String),
    #[error("stats over array properties not supported")]
    ArrayPropertyNotSupported,
}

/// Parser for the Quilt Query DSL.
///
/// This parser implements a recursive descent parsing strategy to convert
/// query strings into [`QueryExpr`] AST nodes.
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
        // Simple recursive descent parser for the query DSL
        let input = input.trim();

        if input.is_empty() {
            return Err(ParseError::Invalid("Empty query".to_string()));
        }

        self.parse_expr(input)
    }

    fn parse_expr(&self, input: &str) -> Result<QueryExpr, ParseError> {
        let input = input.trim();

        // Handle parentheses
        if input.starts_with('(') {
            return self.parse_compound(input);
        }

        // Handle page refs
        if input.starts_with("[[") {
            return self.parse_page_ref(input);
        }

        // Handle self ref
        if input == "self" {
            return Ok(QueryExpr::SelfRef);
        }

        Err(ParseError::Invalid(format!(
            "Unknown expression: {}",
            input
        )))
    }

    fn parse_compound(&self, input: &str) -> Result<QueryExpr, ParseError> {
        if !input.starts_with('(') || !input.ends_with(')') {
            return Err(ParseError::Invalid("Expected parentheses".to_string()));
        }

        let inner = &input[1..input.len() - 1];
        let trimmed = inner.trim();

        // Find the first space to determine the operator
        if let Some(space_idx) = trimmed.find(' ') {
            let op = &trimmed[..space_idx];
            let rest = trimmed[space_idx..].trim();

            match op {
                "and" => self.parse_and(rest),
                "or" => self.parse_or(rest),
                "not" => self.parse_not(rest),
                "between" => self.parse_between(rest),
                "property" => self.parse_property(rest),
                "task" => self.parse_task(rest),
                "priority" => self.parse_priority(rest),
                "page" => self.parse_page(rest),
                "tags" => self.parse_tags(rest),
                "full-text-search" => self.parse_full_text_search(rest),
                "sample" => self.parse_sample(rest),
                "aggregate" => self.parse_aggregate(rest),
                "stats" => self.parse_stats(rest),
                "group_by" => self.parse_group_by(rest),
                "analyze" => self.parse_analyze(rest),
                _ => Err(ParseError::Invalid(format!("Unknown operator: {}", op))),
            }
        } else {
            Err(ParseError::Invalid("Expected operator".to_string()))
        }
    }

    fn parse_and(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        let exprs: Result<Vec<_>, _> = args.iter().map(|s| self.parse_expr(s)).collect();
        Ok(QueryExpr::And(exprs?))
    }

    fn parse_or(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        let exprs: Result<Vec<_>, _> = args.iter().map(|s| self.parse_expr(s)).collect();
        Ok(QueryExpr::Or(exprs?))
    }

    fn parse_not(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        Ok(QueryExpr::Not(Box::new(self.parse_expr(rest)?)))
    }

    fn parse_between(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "between requires 2 arguments".to_string(),
            ));
        }
        Ok(QueryExpr::Between {
            field: "created_at".to_string(),
            start: self.parse_value(&args[0])?,
            end: self.parse_value(&args[1])?,
        })
    }

    fn parse_property(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() == 2 {
            return Ok(QueryAst::Property {
                key: args[0].trim_matches('"').to_string(),
                op: PropertyOp::Equals,
                value: self.parse_value(&args[1])?,
                value2: None,
            });
        }
        if args.len() == 3 {
            let key = args[0].trim_matches('"').to_string();
            let op_str = args[1].trim();
            if let Some(op) = parse_property_op_token(op_str) {
                let value = self.parse_value(&args[2])?;
                return Ok(QueryAst::Property {
                    key,
                    op,
                    value,
                    value2: None,
                });
            }
            return Ok(QueryAst::Property {
                key,
                op: PropertyOp::Between,
                value: self.parse_value(&args[1])?,
                value2: Some(self.parse_value(&args[2])?),
            });
        }
        Err(ParseError::Invalid(
            "property requires 2 (key value) or 3 (key op value | key lo hi) arguments"
                .to_string(),
        ))
    }

    fn parse_task(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        Ok(QueryExpr::Task(
            args.iter().map(|s| s.to_string()).collect(),
        ))
    }

    fn parse_priority(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        Ok(QueryExpr::Priority(
            args.iter().map(|s| s.to_lowercase()).collect(),
        ))
    }

    fn parse_page(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let name = rest.trim_matches('"');
        Ok(QueryExpr::Page(name.to_string()))
    }

    fn parse_tags(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let name = rest.trim_matches('"');
        Ok(QueryExpr::Tags(name.to_string()))
    }

    fn parse_page_ref(&self, input: &str) -> Result<QueryExpr, ParseError> {
        if !input.starts_with("[[") || !input.ends_with("]]") {
            return Err(ParseError::Invalid("Expected page ref".to_string()));
        }
        let name = &input[2..input.len() - 2];
        Ok(QueryExpr::PageRef(name.to_string()))
    }

    fn parse_full_text_search(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let content = rest.trim_matches('"');
        Ok(QueryExpr::BlockContent(content.to_string()))
    }

    fn parse_sample(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let n: usize = rest
            .parse()
            .map_err(|_| ParseError::Invalid("Invalid number for sample".to_string()))?;
        Ok(QueryExpr::Sample(n))
    }

    fn parse_aggregate(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 3 {
            return Err(ParseError::Invalid(
                "aggregate requires (inner), (property ...), and (fn ...)".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let prop = Self::extract_property_arg(&args[1])?;
        let afn = Self::parse_aggregate_fn(&args[2])?;
        Ok(QueryExpr::Aggregate {
            inner: Box::new(inner),
            group_by: prop,
            aggregate_fn: afn,
        })
    }

    fn extract_property_arg(s: &str) -> Result<String, ParseError> {
        let trimmed = s.trim();
        if !trimmed.starts_with("(property ") || !trimmed.ends_with(')') {
            return Err(ParseError::Invalid(
                "expected (property <name>)".to_string(),
            ));
        }
        let inner = &trimmed[10..trimmed.len() - 1];
        Ok(inner.trim().trim_matches('"').to_string())
    }

    fn parse_aggregate_fn(s: &str) -> Result<AggregateFn, ParseError> {
        let trimmed = s.trim();
        if !trimmed.starts_with("(fn ") || !trimmed.ends_with(')') {
            return Err(ParseError::Invalid("expected (fn <name>)".to_string()));
        }
        let inner = &trimmed[4..trimmed.len() - 1];
        match inner.trim() {
            "count" => Ok(AggregateFn::Count),
            "avg" => Ok(AggregateFn::Avg),
            "sum" => Ok(AggregateFn::Sum),
            "min" => Ok(AggregateFn::Min),
            "max" => Ok(AggregateFn::Max),
            _ => Err(ParseError::Invalid(format!(
                "unknown aggregate fn: {}",
                inner
            ))),
        }
    }

    fn parse_stats(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "stats requires (property ...) and (fn ...)".to_string(),
            ));
        }
        let prop = Self::extract_property_arg(&args[0])?;
        let sfn = Self::parse_stats_fn(&args[1])?;
        Ok(QueryExpr::Stats {
            property: prop,
            compute: sfn,
        })
    }

    fn parse_stats_fn(s: &str) -> Result<StatsFn, ParseError> {
        let trimmed = s.trim();
        if !trimmed.starts_with("(fn ") || !trimmed.ends_with(')') {
            return Err(ParseError::Invalid("expected (fn ...)".to_string()));
        }
        let inner = &trimmed[4..trimmed.len() - 1];
        let parts: Vec<_> = inner.split_whitespace().collect();
        match parts.as_slice() {
            ["stddev"] => Ok(StatsFn::Stddev),
            ["variance"] => Ok(StatsFn::Variance),
            ["median"] => Ok(StatsFn::Median),
            ["percentile", v] => {
                let n: u8 = v
                    .parse()
                    .map_err(|_| ParseError::Invalid("percentile must be 0-100".to_string()))?;
                if n > 100 {
                    return Err(ParseError::Invalid("percentile must be 0-100".to_string()));
                }
                Ok(StatsFn::Percentile(n))
            }
            _ => Err(ParseError::Invalid(format!("unknown stats fn: {}", inner))),
        }
    }

    fn parse_group_by(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "group_by requires (inner) and (property ...)".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let prop = Self::extract_property_arg(&args[1])?;
        Ok(QueryExpr::GroupBy {
            inner: Box::new(inner),
            property: prop,
        })
    }

    fn parse_analyze(&self, rest: &str) -> Result<QueryExpr, ParseError> {
        let args = self.split_args(rest);
        if args.len() < 2 {
            return Err(ParseError::Invalid(
                "analyze requires inner expression and kind".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let kind = self.parse_analyze_kind(&args[1], &args[2..])?;
        Ok(QueryExpr::Analyze {
            inner: Box::new(inner),
            kind,
        })
    }

    fn parse_analyze_kind(
        &self,
        kind_str: &str,
        rest: &[String],
    ) -> Result<AnalyzeKind, ParseError> {
        match kind_str {
            "structural_mirror" => {
                if !rest.is_empty() {
                    return Err(ParseError::Invalid(
                        "structural_mirror takes no keyword arguments".to_string(),
                    ));
                }
                Ok(AnalyzeKind::StructuralMirror)
            }
            "serendipity" => {
                let mut limit = None;
                let mut min_confidence = None;
                let mut temporal_window_days = None;

                let mut i = 0;
                while i < rest.len() {
                    match rest[i].as_str() {
                        ":limit" => {
                            i += 1;
                            if i >= rest.len() {
                                return Err(ParseError::Invalid(
                                    "limit requires a number".to_string(),
                                ));
                            }
                            limit = Some(rest[i].parse().map_err(|_| {
                                ParseError::Invalid("limit requires a number".to_string())
                            })?);
                        }
                        ":min-confidence" => {
                            i += 1;
                            if i >= rest.len() {
                                return Err(ParseError::Invalid(
                                    "min-confidence requires a float".to_string(),
                                ));
                            }
                            min_confidence = Some(rest[i].parse().map_err(|_| {
                                ParseError::Invalid("min-confidence requires a float".to_string())
                            })?);
                        }
                        ":temporal-window-days" => {
                            i += 1;
                            if i >= rest.len() {
                                return Err(ParseError::Invalid(
                                    "temporal-window-days requires an integer".to_string(),
                                ));
                            }
                            temporal_window_days = Some(rest[i].parse().map_err(|_| {
                                ParseError::Invalid(
                                    "temporal-window-days requires an integer".to_string(),
                                )
                            })?);
                        }
                        _ => {
                            return Err(ParseError::Invalid(format!(
                                "Unknown keyword in analyze: {}",
                                rest[i]
                            )));
                        }
                    }
                    i += 1;
                }

                Ok(AnalyzeKind::Serendipity {
                    limit,
                    min_confidence,
                    temporal_window_days,
                })
            }
            _ => Err(ParseError::Invalid(format!(
                "Unknown analysis kind: {}",
                kind_str
            ))),
        }
    }

    fn parse_value(&self, s: &str) -> Result<QueryValue, ParseError> {
        let s = s.trim();

        if s.starts_with('"') && s.ends_with('"') {
            return Ok(QueryValue::String(s[1..s.len() - 1].to_string()));
        }

        if s == "true" {
            return Ok(QueryValue::Boolean(true));
        }
        if s == "false" {
            return Ok(QueryValue::Boolean(false));
        }

        if let Ok(n) = s.parse::<i64>() {
            return Ok(QueryValue::Integer(n));
        }

        Ok(QueryValue::String(s.to_string()))
    }

    fn split_args(&self, input: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut current = String::new();

        for c in input.chars() {
            match c {
                '(' => {
                    depth += 1;
                    current.push(c);
                }
                ')' => {
                    depth -= 1;
                    current.push(c);
                }
                ' ' if depth == 0 => {
                    if !current.is_empty() {
                        result.push(current.trim().to_string());
                        current = String::new();
                    }
                }
                _ => current.push(c),
            }
        }

        if !current.is_empty() {
            result.push(current.trim().to_string());
        }

        result
    }
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
        let result = parse("(between 100 200)");
        assert_eq!(
            result,
            QueryExpr::Between {
                field: "created_at".to_string(),
                start: QueryValue::Integer(100),
                end: QueryValue::Integer(200),
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
            QueryAst::Property {

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
            QueryAst::Property {

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
            QueryAst::Property {

                key: "active".to_string(),

                op: PropertyOp::Equals,

                value: QueryValue::Boolean(true),

                value2: None,

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
        let err = parse_err("(unknown x)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_between_too_few_args() {
        let err = parse_err("(between 100)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_property_too_few_args() {
        let err = parse_err("(property \"x\")");
        assert!(matches!(err, ParseError::Invalid(_)));
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
            QueryAst::Property {

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

    // Analyze tests

    #[test]
    fn test_parse_analyze_structural_mirror() {
        let result = parse("(analyze (task todo) structural_mirror)");
        assert_eq!(
            result,
            QueryExpr::Analyze {
                inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
                kind: AnalyzeKind::StructuralMirror,
            }
        );
    }

    #[test]
    fn test_parse_analyze_serendipity_defaults() {
        let result = parse("(analyze (page \"X\") serendipity)");
        assert_eq!(
            result,
            QueryExpr::Analyze {
                inner: Box::new(QueryExpr::Page("X".to_string())),
                kind: AnalyzeKind::Serendipity {
                    limit: None,
                    min_confidence: None,
                    temporal_window_days: None,
                },
            }
        );
    }

    #[test]
    fn test_parse_analyze_serendipity_with_limit() {
        let result = parse("(analyze (task todo) serendipity :limit 20)");
        match result {
            QueryExpr::Analyze {
                kind: AnalyzeKind::Serendipity { limit, .. },
                ..
            } => {
                assert_eq!(limit, Some(20));
            }
            _ => panic!("expected Serendipity with limit"),
        }
    }

    #[test]
    fn test_parse_analyze_serendipity_full() {
        let result = parse("(analyze (task todo) serendipity :limit 10 :min-confidence 0.4 :temporal-window-days 14)");
        match result {
            QueryExpr::Analyze {
                kind:
                    AnalyzeKind::Serendipity {
                        limit,
                        min_confidence,
                        temporal_window_days,
                    },
                ..
            } => {
                assert_eq!(limit, Some(10));
                assert_eq!(min_confidence, Some(0.4));
                assert_eq!(temporal_window_days, Some(14));
            }
            _ => panic!("expected full Serendipity"),
        }
    }

    #[test]
    fn test_parse_analyze_empty() {
        let err = parse_err("(analyze)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_missing_kind() {
        let err = parse_err("(analyze (task todo))");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_unknown_kind() {
        let err = parse_err("(analyze (task todo) unknown_kind)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_limit_no_value() {
        let err = parse_err("(analyze (task todo) serendipity :limit)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_min_confidence_no_value() {
        let err = parse_err("(analyze (task todo) serendipity :min-confidence)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_temporal_window_no_value() {
        let err = parse_err("(analyze (task todo) serendipity :temporal-window-days)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_analyze_structural_mirror_with_kwargs() {
        let err = parse_err("(analyze (task todo) structural_mirror :limit 5)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }
}

/// F3 — Map a DSL operator token to a [`PropertyOp`] variant. Returns
/// `None` for tokens that aren't operators (e.g., numeric literals
/// which the parser then interprets as the start of a `Between` range).
fn parse_property_op_token(token: &str) -> Option<PropertyOp> {
    match token {
        ">" => Some(PropertyOp::GreaterThan),
        "<" => Some(PropertyOp::LessThan),
        ">=" => Some(PropertyOp::GreaterThanOrEqual),
        "<=" => Some(PropertyOp::LessThanOrEqual),
        "!=" => Some(PropertyOp::NotEquals),
        "contains" => Some(PropertyOp::Contains),
        _ => None,
    }
}
