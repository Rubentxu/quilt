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
    AggregateFn, AnalyzeKind, QueryAst, QueryValue, SortDirection, StatsFn, TemporalRange,
    VirtualColumn,
};
// Re-export `PropertyOp` (F3) — the parser uses it in `parse_property`
// to record the operator.
pub use crate::property_op::PropertyOp;
// TimeOffset is used in tests via `use super::*`
#[cfg(test)]
use crate::time_helpers::TimeOffset;

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
/// query strings into [`QueryAst`] AST nodes.
///
/// # Example
///
/// ```
/// use quilt_query::{QueryParser, QueryAst};
///
/// let parser = QueryParser;
/// let result = parser.parse("(task todo)");
/// assert!(result.is_ok());
/// ```
pub struct QueryParser;

impl QueryParser {
    /// Parses a query string into a [`QueryAst`] AST.
    ///
    /// # Arguments
    ///
    /// * `input` - The query string to parse
    ///
    /// # Returns
    ///
    /// Returns the parsed [`QueryAst`] on success.
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
    pub fn parse(&self, input: &str) -> Result<QueryAst, ParseError> {
        // Simple recursive descent parser for the query DSL
        let input = input.trim();

        if input.is_empty() {
            return Err(ParseError::Invalid("Empty query".to_string()));
        }

        self.parse_expr(input)
    }

    fn parse_expr(&self, input: &str) -> Result<QueryAst, ParseError> {
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
            return Ok(QueryAst::SelfRef);
        }

        Err(ParseError::Invalid(format!(
            "Unknown expression: {}",
            input
        )))
    }

    fn parse_compound(&self, input: &str) -> Result<QueryAst, ParseError> {
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
                "page-fuzzy" => self.parse_page_fuzzy(rest),
                "temporal" => self.parse_temporal(rest),
                "virtual-select" => self.parse_virtual_select(rest),
                _ => Err(ParseError::Invalid(format!("Unknown operator: {}", op))),
            }
        } else {
            Err(ParseError::Invalid("Expected operator".to_string()))
        }
    }

    fn parse_and(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        let exprs: Result<Vec<_>, _> = args.iter().map(|s| self.parse_expr(s)).collect();
        Ok(QueryAst::And(exprs?))
    }

    fn parse_or(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        let exprs: Result<Vec<_>, _> = args.iter().map(|s| self.parse_expr(s)).collect();
        Ok(QueryAst::Or(exprs?))
    }

    fn parse_not(&self, rest: &str) -> Result<QueryAst, ParseError> {
        Ok(QueryAst::Not(Box::new(self.parse_expr(rest)?)))
    }

    fn parse_between(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "between requires 2 arguments".to_string(),
            ));
        }
        Ok(QueryAst::Between {
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
            "property requires 2 (key value) or 3 (key op value | key lo hi) arguments".to_string(),
        ))
    }

    fn parse_task(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        Ok(QueryAst::Task(
            args.iter().map(|s| s.to_string()).collect(),
        ))
    }

    fn parse_priority(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        Ok(QueryAst::Priority(
            args.iter().map(|s| s.to_lowercase()).collect(),
        ))
    }

    fn parse_page(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let name = rest.trim_matches('"');
        Ok(QueryAst::Page(name.to_string()))
    }

    fn parse_tags(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let name = rest.trim_matches('"');
        Ok(QueryAst::Tags(name.to_string()))
    }

    fn parse_page_ref(&self, input: &str) -> Result<QueryAst, ParseError> {
        if !input.starts_with("[[") || !input.ends_with("]]") {
            return Err(ParseError::Invalid("Expected page ref".to_string()));
        }
        let name = &input[2..input.len() - 2];
        Ok(QueryAst::PageRef(name.to_string()))
    }

    fn parse_full_text_search(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let content = rest.trim_matches('"');
        Ok(QueryAst::BlockContent(content.to_string()))
    }

    fn parse_sample(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let n: usize = rest
            .parse()
            .map_err(|_| ParseError::Invalid("Invalid number for sample".to_string()))?;
        Ok(QueryAst::Sample(n))
    }

    fn parse_aggregate(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 3 {
            return Err(ParseError::Invalid(
                "aggregate requires (inner), (property ...), and (fn ...)".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let prop = Self::extract_property_arg(&args[1])?;
        let afn = Self::parse_aggregate_fn(&args[2])?;
        Ok(QueryAst::Aggregate {
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

    fn parse_stats(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "stats requires (property ...) and (fn ...)".to_string(),
            ));
        }
        let prop = Self::extract_property_arg(&args[0])?;
        let sfn = Self::parse_stats_fn(&args[1])?;
        Ok(QueryAst::Stats {
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

    fn parse_group_by(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "group_by requires (inner) and (property ...)".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let prop = Self::extract_property_arg(&args[1])?;
        Ok(QueryAst::GroupBy {
            inner: Box::new(inner),
            property: prop,
        })
    }

    fn parse_analyze(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() < 2 {
            return Err(ParseError::Invalid(
                "analyze requires inner expression and kind".to_string(),
            ));
        }
        let inner = self.parse_expr(&args[0])?;
        let kind = self.parse_analyze_kind(&args[1], &args[2..])?;
        Ok(QueryAst::Analyze {
            inner: Box::new(inner),
            kind,
        })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy — fuzzy page name matching
    // ─────────────────────────────────────────────────────────────────────────

    /// Parses `(page-fuzzy "term" limit)` into `QueryAst::PageFuzzy`.
    fn parse_page_fuzzy(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "page-fuzzy requires exactly 2 arguments: term and limit".to_string(),
            ));
        }
        let term = args[0].trim_matches('"').to_string();
        let limit: usize = args[1]
            .parse()
            .map_err(|_| ParseError::Invalid("limit must be a positive integer".to_string()))?;
        Ok(QueryAst::PageFuzzy { term, limit })
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal — temporal classification
    // ─────────────────────────────────────────────────────────────────────────

    /// Parses `(temporal :today (page "x"))` into `QueryAst::Temporal`.
    fn parse_temporal(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() < 2 {
            return Err(ParseError::Invalid(
                "temporal requires a range and an inner expression".to_string(),
            ));
        }

        // Determine how many args the range takes
        let range_arg_count = match args[0].trim() {
            ":custom" => 3,   // :custom + start + end
            ":relative" => 2, // :relative + offset
            _ => 1,           // simple keywords like :today
        };

        if args.len() < range_arg_count + 1 {
            return Err(ParseError::Invalid(
                "temporal requires a range and an inner expression".to_string(),
            ));
        }

        // Extract range args
        let range_args = &args[..range_arg_count];
        let range = self.parse_temporal_range(range_args)?;

        // Inner expression starts at index range_arg_count
        let inner_str = &args[range_arg_count..].join(" ");
        let inner = self.parse_expr(inner_str)?;
        Ok(QueryAst::Temporal {
            range,
            inner: Box::new(inner),
        })
    }

    /// Parses temporal range arguments into `TemporalRange`.
    ///
    /// Handles:
    /// - Simple keywords: `:today`, `:yesterday`, `:this-week`, etc.
    /// - Custom range: `:custom "start" "end"`
    /// - Relative offset: `:relative "-7d"`
    fn parse_temporal_range(&self, args: &[String]) -> Result<TemporalRange, ParseError> {
        if args.is_empty() {
            return Err(ParseError::Invalid(
                "temporal range keyword missing".to_string(),
            ));
        }
        let first = args[0].trim();
        match first {
            ":today" => Ok(TemporalRange::Today),
            ":yesterday" => Ok(TemporalRange::Yesterday),
            ":this-week" => Ok(TemporalRange::ThisWeek),
            ":last-week" => Ok(TemporalRange::LastWeek),
            ":this-month" => Ok(TemporalRange::ThisMonth),
            ":last-month" => Ok(TemporalRange::LastMonth),
            ":custom" => {
                // args: [":custom", "start", "end"]
                if args.len() != 3 {
                    return Err(ParseError::Invalid(
                        ":custom requires two date arguments".to_string(),
                    ));
                }
                let start = args[1].trim_matches('"').to_string();
                let end = args[2].trim_matches('"').to_string();
                Ok(TemporalRange::Custom { start, end })
            }
            ":relative" => {
                // args: [":relative", "offset"]
                if args.len() != 2 {
                    return Err(ParseError::Invalid(
                        ":relative requires an offset argument".to_string(),
                    ));
                }
                let offset_str = args[1].trim_matches('"').to_string();
                let offset =
                    crate::time_helpers::TimeOffset::parse(&offset_str).ok_or_else(|| {
                        ParseError::Invalid(format!(
                            "invalid time offset: {} (expected format like \"-7d\", \"2w\", etc.)",
                            offset_str
                        ))
                    })?;
                Ok(TemporalRange::Relative(offset))
            }
            _ => Err(ParseError::Invalid(format!(
                "unknown temporal range: {} (expected :today, :yesterday, :this-week, :last-week, :this-month, :last-month, :custom, or :relative",
                first
            ))),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect — virtual column selection
    // ─────────────────────────────────────────────────────────────────────────

    /// Parses `(virtual-select [word_count ref_count] (page "x"))` into
    /// `QueryAst::VirtualSelect`.
    fn parse_virtual_select(&self, rest: &str) -> Result<QueryAst, ParseError> {
        let args = self.split_args(rest);
        if args.len() != 2 {
            return Err(ParseError::Invalid(
                "virtual-select requires a column list and an inner expression".to_string(),
            ));
        }

        let columns = self.parse_virtual_column_list(&args[0])?;
        let inner = self.parse_expr(&args[1])?;
        Ok(QueryAst::VirtualSelect {
            columns,
            inner: Box::new(inner),
        })
    }

    /// Parses `[word_count ref_count]` into `Vec<VirtualColumn>`.
    fn parse_virtual_column_list(&self, s: &str) -> Result<Vec<VirtualColumn>, ParseError> {
        let s = s.trim();
        if !s.starts_with('[') || !s.ends_with(']') {
            return Err(ParseError::Invalid(
                "virtual-select column list must be enclosed in brackets".to_string(),
            ));
        }
        let inner = &s[1..s.len() - 1];
        let names: Vec<&str> = inner
            .split_whitespace()
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .collect();

        if names.is_empty() {
            return Err(ParseError::Invalid(
                "virtual-select requires at least one column".to_string(),
            ));
        }

        let mut columns = Vec::with_capacity(names.len());
        for name in names {
            let col = match name {
                "word_count" => VirtualColumn::WordCount,
                "char_count" => VirtualColumn::CharCount,
                "ref_count" => VirtualColumn::RefCount,
                "block_age_days" => VirtualColumn::BlockAgeDays,
                _ => {
                    return Err(ParseError::Invalid(format!(
                        "unknown virtual column: {} (expected word_count, char_count, ref_count, block_age_days)",
                        name
                    )));
                }
            };
            columns.push(col);
        }
        Ok(columns)
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
        let mut bracket_depth = 0;
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
                '[' => {
                    bracket_depth += 1;
                    current.push(c);
                }
                ']' => {
                    bracket_depth -= 1;
                    current.push(c);
                }
                ' ' if depth == 0 && bracket_depth == 0 => {
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

    fn parse(input: &str) -> QueryAst {
        QueryParser.parse(input).expect("parse failed")
    }

    fn parse_err(input: &str) -> ParseError {
        QueryParser.parse(input).expect_err("expected parse error")
    }

    // Basic parsing

    #[test]
    fn test_parse_simple_task() {
        let result = parse("(task todo)");
        assert_eq!(result, QueryAst::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_priority() {
        let result = parse("(priority a)");
        assert_eq!(result, QueryAst::Priority(vec!["a".to_string()]));
    }

    #[test]
    fn test_parse_page() {
        let result = parse("(page \"MyPage\")");
        assert_eq!(result, QueryAst::Page("MyPage".to_string()));
    }

    #[test]
    fn test_parse_tags() {
        let result = parse("(tags \"rust\")");
        assert_eq!(result, QueryAst::Tags("rust".to_string()));
    }

    #[test]
    fn test_parse_sample() {
        let result = parse("(sample 10)");
        assert_eq!(result, QueryAst::Sample(10));
    }

    #[test]
    fn test_parse_self_ref() {
        let result = parse("self");
        assert_eq!(result, QueryAst::SelfRef);
    }

    #[test]
    fn test_parse_page_ref() {
        let result = parse("[[Some Page]]");
        assert_eq!(result, QueryAst::PageRef("Some Page".to_string()));
    }

    #[test]
    fn test_parse_full_text_search() {
        let result = parse("(full-text-search \"hello\")");
        assert_eq!(result, QueryAst::BlockContent("hello".to_string()));
    }

    // Compound queries

    #[test]
    fn test_parse_and() {
        let result = parse("(and (task todo) (priority a))");
        assert_eq!(
            result,
            QueryAst::And(vec![
                QueryAst::Task(vec!["todo".to_string()]),
                QueryAst::Priority(vec!["a".to_string()]),
            ])
        );
    }

    #[test]
    fn test_parse_or() {
        let result = parse("(or (task todo) (task done))");
        assert_eq!(
            result,
            QueryAst::Or(vec![
                QueryAst::Task(vec!["todo".to_string()]),
                QueryAst::Task(vec!["done".to_string()]),
            ])
        );
    }

    #[test]
    fn test_parse_not() {
        let result = parse("(not (task done))");
        assert_eq!(
            result,
            QueryAst::Not(Box::new(QueryAst::Task(vec!["done".to_string()])))
        );
    }

    // Nested queries

    #[test]
    fn test_parse_nested_and_or() {
        let result = parse("(and (or (task todo) (priority a)) (page \"X\"))");
        assert_eq!(
            result,
            QueryAst::And(vec![
                QueryAst::Or(vec![
                    QueryAst::Task(vec!["todo".to_string()]),
                    QueryAst::Priority(vec!["a".to_string()]),
                ]),
                QueryAst::Page("X".to_string()),
            ])
        );
    }

    #[test]
    fn test_parse_deeply_nested_not() {
        let result = parse("(and (not (or (task done) (task cancelled))) (priority a))");
        assert_eq!(
            result,
            QueryAst::And(vec![
                QueryAst::Not(Box::new(QueryAst::Or(vec![
                    QueryAst::Task(vec!["done".to_string()]),
                    QueryAst::Task(vec!["cancelled".to_string()]),
                ]))),
                QueryAst::Priority(vec!["a".to_string()]),
            ])
        );
    }

    // Between

    #[test]
    fn test_parse_between_integers() {
        let result = parse("(between 100 200)");
        assert_eq!(
            result,
            QueryAst::Between {
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
            QueryAst::Between {
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
            QueryAst::Task(vec!["todo".to_string(), "done".to_string()])
        );
    }

    #[test]
    fn test_parse_multiple_priorities() {
        let result = parse("(priority a b c)");
        // Priorities are lowercased
        assert_eq!(
            result,
            QueryAst::Priority(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    // Edge cases: unicode content

    #[test]
    fn test_parse_unicode_page_name() {
        // Page names can contain unicode
        let result = parse("(page \"日本語ページ\")");
        assert_eq!(result, QueryAst::Page("日本語ページ".to_string()));
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
        assert_eq!(result, QueryAst::Tags("🚀".to_string()));
    }

    // Edge cases: large inputs

    #[test]
    fn test_parse_large_page_name() {
        let large_name = "a".repeat(10000);
        let query = format!("(page \"{}\")", large_name);
        let result = parse(&query);
        assert_eq!(result, QueryAst::Page(large_name));
    }

    #[test]
    fn test_parse_many_task_markers() {
        // Test with many markers - parser should handle it
        let result = parse("(task todo done later now cancelled)");
        assert_eq!(
            result,
            QueryAst::Task(vec![
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
            QueryAst::Priority(vec!["a".to_string(), "b".to_string(), "c".to_string()])
        );
    }

    // Edge cases: whitespace handling

    #[test]
    fn test_parse_leading_whitespace() {
        let result = parse("  (task todo)");
        assert_eq!(result, QueryAst::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_trailing_whitespace() {
        let result = parse("(task todo)   ");
        assert_eq!(result, QueryAst::Task(vec!["todo".to_string()]));
    }

    #[test]
    fn test_parse_nested_whitespace() {
        let result = parse("(task  todo)");
        assert_eq!(result, QueryAst::Task(vec!["todo".to_string()]));
    }

    // Analyze tests

    #[test]
    fn test_parse_analyze_structural_mirror() {
        let result = parse("(analyze (task todo) structural_mirror)");
        assert_eq!(
            result,
            QueryAst::Analyze {
                inner: Box::new(QueryAst::Task(vec!["todo".to_string()])),
                kind: AnalyzeKind::StructuralMirror,
            }
        );
    }

    #[test]
    fn test_parse_analyze_serendipity_defaults() {
        let result = parse("(analyze (page \"X\") serendipity)");
        assert_eq!(
            result,
            QueryAst::Analyze {
                inner: Box::new(QueryAst::Page("X".to_string())),
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
            QueryAst::Analyze {
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
        let result = parse(
            "(analyze (task todo) serendipity :limit 10 :min-confidence 0.4 :temporal-window-days 14)",
        );
        match result {
            QueryAst::Analyze {
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

    // ─────────────────────────────────────────────────────────────────────────
    // G5: PageFuzzy tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_page_fuzzy_basic() {
        let result = parse("(page-fuzzy \"rust\" 10)");
        match result {
            QueryAst::PageFuzzy { term, limit } => {
                assert_eq!(term, "rust");
                assert_eq!(limit, 10);
            }
            other => panic!("expected PageFuzzy, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_fuzzy_default_limit() {
        let result = parse("(page-fuzzy \"rust\" 50)");
        match result {
            QueryAst::PageFuzzy { term, limit } => {
                assert_eq!(term, "rust");
                assert_eq!(limit, 50);
            }
            other => panic!("expected PageFuzzy, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_fuzzy_requires_two_args() {
        let err = parse_err("(page-fuzzy \"rust\")");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_page_fuzzy_limit_must_be_integer() {
        let err = parse_err("(page-fuzzy \"rust\" abc)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_temporal_today() {
        let result = parse("(temporal :today (page \"x\"))");
        match result {
            QueryAst::Temporal { range, inner } => {
                assert_eq!(range, TemporalRange::Today);
                assert_eq!(*inner, QueryAst::Page("x".to_string()));
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_this_week() {
        let result = parse("(temporal :this-week (page \"x\"))");
        match result {
            QueryAst::Temporal { range, inner: _ } => {
                assert_eq!(range, TemporalRange::ThisWeek);
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_last_week() {
        let result = parse("(temporal :last-week (task todo))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::LastWeek);
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_yesterday() {
        let result = parse("(temporal :yesterday (priority a))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::Yesterday);
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_this_month() {
        let result = parse("(temporal :this-month (page \"test\"))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::ThisMonth);
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_last_month() {
        let result = parse("(temporal :last-month (page \"test\"))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::LastMonth);
            }
            other => panic!("expected Temporal, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_custom() {
        let result = parse("(temporal :custom \"2024-01-01\" \"2024-12-31\" (page \"x\"))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(
                    range,
                    TemporalRange::Custom {
                        start: "2024-01-01".to_string(),
                        end: "2024-12-31".to_string(),
                    }
                );
            }
            other => panic!("expected Temporal with Custom, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_relative() {
        let result = parse("(temporal :relative \"-7d\" (page \"x\"))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::Relative(TimeOffset::Days(-7)));
            }
            other => panic!("expected Temporal with Relative, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_relative_weeks() {
        let result = parse("(temporal :relative \"2w\" (page \"x\"))");
        match result {
            QueryAst::Temporal { range, .. } => {
                assert_eq!(range, TemporalRange::Relative(TimeOffset::Weeks(2)));
            }
            other => panic!("expected Temporal with Relative, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_temporal_requires_range_and_inner() {
        let err = parse_err("(temporal :today)");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_temporal_unknown_range() {
        let err = parse_err("(temporal :unknown (page \"x\"))");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // F12: VirtualSelect tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn test_parse_virtual_select_word_count() {
        let result = parse("(virtual-select [word_count] (page \"x\"))");
        match result {
            QueryAst::VirtualSelect { columns, inner } => {
                assert_eq!(columns, vec![VirtualColumn::WordCount]);
                assert_eq!(*inner, QueryAst::Page("x".to_string()));
            }
            other => panic!("expected VirtualSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_virtual_select_multiple_columns() {
        let result = parse("(virtual-select [word_count ref_count] (page \"x\"))");
        match result {
            QueryAst::VirtualSelect { columns, .. } => {
                assert_eq!(
                    columns,
                    vec![VirtualColumn::WordCount, VirtualColumn::RefCount]
                );
            }
            other => panic!("expected VirtualSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_virtual_select_all_columns() {
        let result =
            parse("(virtual-select [word_count char_count ref_count block_age_days] (page \"x\"))");
        match result {
            QueryAst::VirtualSelect { columns, .. } => {
                assert_eq!(
                    columns,
                    vec![
                        VirtualColumn::WordCount,
                        VirtualColumn::CharCount,
                        VirtualColumn::RefCount,
                        VirtualColumn::BlockAgeDays
                    ]
                );
            }
            other => panic!("expected VirtualSelect, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_virtual_select_requires_two_args() {
        let err = parse_err("(virtual-select [word_count])");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_virtual_select_requires_brackets() {
        let err = parse_err("(virtual-select word_count (page \"x\"))");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_virtual_select_unknown_column() {
        let err = parse_err("(virtual-select [unknown_col] (page \"x\"))");
        assert!(matches!(err, ParseError::Invalid(_)));
    }

    #[test]
    fn test_parse_virtual_select_requires_at_least_one_column() {
        let err = parse_err("(virtual-select [] (page \"x\"))");
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
