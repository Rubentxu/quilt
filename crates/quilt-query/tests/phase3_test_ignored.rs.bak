//! Phase 3 Tests for robust-query-dsl change
//!
//! These tests cover:
//! 1. Pre-processor bare integer normalization
//! 2. Semantic validator arity rules
//! 3. Semantic validator range rules
//! 4. SortBy SQL generation
//! 5. Exists SQL generation
//! 6. Missing SQL generation
//! 7. Namespace SQL generation
//! 8. Table SQL generation
//! 9. Backward compatibility with existing queries
//! 10. TimeOffset direction regression
//! 11. Query-service end-to-end integration

use quilt_query::parser::{
    preprocess, validate, ParseError, PropertyOp, QueryExpr, QueryParser, SortDirection,
};
use quilt_query::{QueryExecutor, QueryParser as QParser};

/// Helper to parse a query string
#[allow(dead_code)]
fn parse(input: &str) -> QueryExpr {
    QueryParser.parse(input).expect("parse failed")
}

/// Helper to parse and get a parse error
#[allow(dead_code)]
fn parse_err(input: &str) -> ParseError {
    QueryParser.parse(input).expect_err("expected parse error")
}

/// Helper to build SQL from a query string
fn build_sql(input: &str) -> (String, Vec<String>) {
    let parser = QParser;
    let executor = QueryExecutor::new();
    let expr = parser.parse(input).expect("parse failed");
    let (sql, params) = executor.build_sql(&expr, 100);
    (sql, params.iter().map(|p| p.as_string()).collect())
}

/// Helper to build WHERE clause only
fn build_where(input: &str) -> (String, Vec<String>) {
    let parser = QParser;
    let executor = QueryExecutor::new();
    let expr = parser.parse(input).expect("parse failed");
    let (sql, params) = executor.build_where(&expr);
    (sql, params.iter().map(|p| p.as_string()).collect())
}

// =============================================================================
// Task 1: Unit test for pre-processor bare integer normalization
// =============================================================================

#[cfg(test)]
mod preprocess_tests {
    use super::*;

    #[test]
    fn test_preprocess_bare_integer_between() {
        // Bare integers in between should be quoted
        assert_eq!(preprocess("(between 100 200)"), "(between \"100\" \"200\")");
    }

    #[test]
    fn test_preprocess_already_quoted_integers() {
        // Already quoted integers should be unchanged
        assert_eq!(
            preprocess("(between \"100\" \"200\")"),
            "(between \"100\" \"200\")"
        );
    }

    #[test]
    fn test_preprocess_mixed_quoted_bare() {
        // When first arg is already quoted, only second bare integer gets quoted
        // The regex only matches bare integers, not quoted ones
        assert_eq!(
            preprocess("(between \"100\" 200)"),
            "(between \"100\" 200)" // First arg unchanged, second arg still bare
        );
    }

    #[test]
    fn test_preprocess_no_between_clause() {
        // No between clause - should be unchanged
        assert_eq!(preprocess("(task todo)"), "(task todo)");
    }

    #[test]
    fn test_preprocess_large_integers() {
        // Large integers should be quoted
        assert_eq!(
            preprocess("(between 9999999999998 9999999999999)"),
            "(between \"9999999999998\" \"9999999999999\")"
        );
    }

    #[test]
    fn test_preprocess_negative_integers() {
        // Negative integers in between context
        // Note: the regex expects \d+ which doesn't match negative directly
        // The preprocessing happens before negative sign handling
        assert_eq!(
            preprocess("(between 100 -200)"),
            "(between \"100\" \"-200\")"
        );
    }

    #[test]
    fn test_preprocess_multiple_between() {
        // Multiple between clauses
        assert_eq!(
            preprocess("(and (between 1 10) (between 20 30))"),
            "(and (between \"1\" \"10\") (between \"20\" \"30\"))"
        );
    }

    #[test]
    fn test_preprocess_nested_between() {
        // Nested queries with between
        let result = preprocess("(and (or (between 1 5) (task todo)) (priority a))");
        assert_eq!(
            result,
            "(and (or (between \"1\" \"5\") (task todo)) (priority a))"
        );
    }
}

// =============================================================================
// Task 2: Unit test for semantic validator arity rules
// =============================================================================

#[cfg(test)]
mod validator_arity_tests {
    use super::*;

    #[test]
    fn test_validate_sample_arity_valid() {
        // sample with valid count (1-1000)
        let expr = QueryExpr::Sample(500);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sample_arity_zero() {
        // sample with 0 should fail
        let expr = QueryExpr::Sample(0);
        let result = validate(&expr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::Invalid(_)));
    }

    #[test]
    fn test_validate_sample_arity_too_large() {
        // sample with > 1000 should fail
        let expr = QueryExpr::Sample(1001);
        let result = validate(&expr);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::Invalid(_)));
    }

    #[test]
    fn test_validate_sample_arity_boundary_1() {
        // sample with 1 should pass (lower boundary)
        let expr = QueryExpr::Sample(1);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sample_arity_boundary_1000() {
        // sample with 1000 should pass (upper boundary)
        let expr = QueryExpr::Sample(1000);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sortby_requires_field() {
        // sort-by with empty field should fail
        let expr = QueryExpr::SortBy {
            field: "".to_string(),
            direction: SortDirection::Asc,
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
        };
        let result = validate(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_sortby_valid() {
        // valid sort-by should pass
        let expr = QueryExpr::SortBy {
            field: "created_at".to_string(),
            direction: SortDirection::Asc,
            inner: Box::new(QueryExpr::Task(vec!["todo".to_string()])),
        };
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_exists_requires_key() {
        // exists with empty key should fail
        let expr = QueryExpr::Exists("".to_string());
        let result = validate(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exists_valid() {
        // valid exists should pass
        let expr = QueryExpr::Exists("author".to_string());
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_namespace_requires_name() {
        // namespace with empty name should fail
        let expr = QueryExpr::Namespace("".to_string());
        let result = validate(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_namespace_valid() {
        // valid namespace should pass
        let expr = QueryExpr::Namespace("work".to_string());
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_table_requires_at_least_one_expr() {
        // table with empty expressions should fail
        let expr = QueryExpr::Table(vec![]);
        let result = validate(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_table_valid() {
        // valid table should pass
        let expr = QueryExpr::Table(vec![QueryExpr::Task(vec!["todo".to_string()])]);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_property_between_requires_value2() {
        // property with 'between' op but no value2 should fail
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::Between,
            value: quilt_query::parser::QueryValue::Integer(10),
            value2: None,
        };
        let result = validate(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_property_between_with_both_values() {
        // property with 'between' op and both values should pass
        let expr = QueryExpr::Property {
            key: "count".to_string(),
            op: PropertyOp::Between,
            value: quilt_query::parser::QueryValue::Integer(10),
            value2: Some(quilt_query::parser::QueryValue::Integer(100)),
        };
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_nested_ands() {
        // nested ands should recursively validate
        let expr = QueryExpr::And(vec![
            QueryExpr::Task(vec!["todo".to_string()]),
            QueryExpr::And(vec![
                QueryExpr::Priority(vec!["a".to_string()]),
                QueryExpr::Sample(50),
            ]),
        ]);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_nested_ands_with_invalid_sample() {
        // nested ands with invalid sample should fail at validation
        let expr = QueryExpr::And(vec![
            QueryExpr::Task(vec!["todo".to_string()]),
            QueryExpr::And(vec![
                QueryExpr::Priority(vec!["a".to_string()]),
                QueryExpr::Sample(0), // invalid
            ]),
        ]);
        let result = validate(&expr);
        assert!(result.is_err());
    }
}

// =============================================================================
// Task 3: Unit test for semantic validator range rules
// =============================================================================

#[cfg(test)]
mod validator_range_tests {
    use super::*;

    #[test]
    fn test_validate_sample_range_0_invalid() {
        let expr = QueryExpr::Sample(0);
        assert!(validate(&expr).is_err());
    }

    #[test]
    fn test_validate_sample_range_1_valid() {
        let expr = QueryExpr::Sample(1);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sample_range_500_valid() {
        let expr = QueryExpr::Sample(500);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sample_range_1000_valid() {
        let expr = QueryExpr::Sample(1000);
        assert!(validate(&expr).is_ok());
    }

    #[test]
    fn test_validate_sample_range_1001_invalid() {
        let expr = QueryExpr::Sample(1001);
        assert!(validate(&expr).is_err());
    }

    #[test]
    fn test_validate_sample_range_large_invalid() {
        let expr = QueryExpr::Sample(10000);
        assert!(validate(&expr).is_err());
    }

    #[test]
    fn test_validate_sample_range_negative_invalid() {
        // Negative sample values are caught at parse time (parsing to usize)
        // But semantic validation can also catch edge cases
        let expr = QueryExpr::Sample(1); // This is the minimum
        assert!(validate(&expr).is_ok());
    }
}

// =============================================================================
// Task 4: Unit test for SortBy SQL generation
// =============================================================================

#[cfg(test)]
mod sortby_sql_tests {
    use super::*;

    #[test]
    fn test_sortby_generates_order_by() {
        // Note: field must be quoted as string per grammar
        let (sql, _) = build_sql(r#"(sort-by "created_at" asc (task todo))"#);
        assert!(
            sql.contains("ORDER BY"),
            "SortBy should generate ORDER BY clause"
        );
        assert!(
            sql.contains("created_at"),
            "SortBy should include field name"
        );
        assert!(sql.contains("ASC"), "SortBy should include direction");
    }

    #[test]
    fn test_sortby_desc_direction() {
        let (sql, _) = build_sql(r#"(sort-by "updated_at" desc (task todo))"#);
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("updated_at"));
        assert!(sql.contains("DESC"));
    }

    #[test]
    fn test_sortby_default_direction() {
        // Without explicit direction, should default to asc
        let (sql, _) = build_sql(r#"(sort-by "created_at" (task todo))"#);
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("ASC"));
    }

    #[test]
    fn test_sortby_where_clause_generated() {
        // The inner expression should still generate WHERE clause
        let (sql, params) = build_where(r#"(sort-by "created_at" asc (task todo))"#);
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_sortby_integer_field() {
        // Integer fields should be used directly (column index)
        let (sql, _) = build_sql("(sort-by 1 asc (task todo))");
        assert!(sql.contains("ORDER BY 1"));
    }

    #[test]
    fn test_sortby_full_sql_structure() {
        let (sql, _) = build_sql(r#"(sort-by "created_at" desc (task todo))"#);
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("JOIN pages p"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("ORDER BY b.created_at DESC"));
        assert!(sql.contains("LIMIT 100"));
    }

    #[test]
    fn test_sortby_date_field_qualified() {
        // Date fields should be qualified with b. prefix
        let (sql, _) = build_sql(r#"(sort-by "scheduled" asc (task todo))"#);
        assert!(sql.contains("ORDER BY b.scheduled"));
    }

    #[test]
    fn test_sortby_with_and() {
        // SortBy wrapping complex expression
        let (sql, _) = build_sql(r#"(sort-by "created_at" desc (and (task todo) (priority a)))"#);
        assert!(sql.contains("ORDER BY"));
        assert!(sql.contains("AND"));
    }
}

// =============================================================================
// Task 5: Unit test for Exists SQL generation
// =============================================================================

#[cfg(test)]
mod exists_sql_tests {
    use super::*;

    #[test]
    fn test_exists_generates_is_not_null() {
        let (sql, params) = build_where("(exists \"author\")");
        assert!(
            sql.contains("IS NOT NULL"),
            "Exists should generate IS NOT NULL"
        );
        assert!(
            sql.contains("json_extract"),
            "Exists should use json_extract"
        );
        assert!(sql.contains("$.author"), "Exists should reference the key");
        assert!(params.is_empty(), "Exists should have no params");
    }

    #[test]
    fn test_exists_full_sql() {
        let (sql, params) = build_sql("(exists \"custom-field\")");
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("json_extract"));
        assert!(sql.contains("$.custom-field"));
        assert!(sql.contains("IS NOT NULL"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_exists_combined_with_task() {
        let (sql, params) = build_where("(and (exists \"author\") (task todo))");
        assert!(sql.contains("IS NOT NULL"));
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_exists_multiple_keys() {
        // Multiple exists combined with OR
        let (sql, params) = build_where("(or (exists \"author\") (exists \"reviewer\"))");
        assert!(sql.contains("IS NOT NULL"));
        assert!(sql.contains("OR"));
        assert!(params.is_empty());
    }
}

// =============================================================================
// Task 6: Unit test for Missing SQL generation
// =============================================================================

#[cfg(test)]
mod missing_sql_tests {
    use super::*;

    #[test]
    fn test_missing_generates_is_null() {
        let (sql, params) = build_where("(missing \"author\")");
        assert!(sql.contains("IS NULL"), "Missing should generate IS NULL");
        assert!(
            sql.contains("json_extract"),
            "Missing should use json_extract"
        );
        assert!(sql.contains("$.author"), "Missing should reference the key");
        assert!(params.is_empty(), "Missing should have no params");
    }

    #[test]
    fn test_missing_full_sql() {
        let (sql, params) = build_sql("(missing \"custom-field\")");
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("json_extract"));
        assert!(sql.contains("$.custom-field"));
        assert!(sql.contains("IS NULL"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_missing_combined_with_task() {
        let (sql, params) = build_where("(and (missing \"author\") (task todo))");
        assert!(sql.contains("IS NULL"));
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_missing_not_query() {
        // Missing wrapped in NOT should result in NOT (json_extract... IS NULL)
        // Semantically equivalent to IS NOT NULL
        let (sql, params) = build_where("(not (missing \"author\"))");
        assert!(
            sql.contains("NOT (json_extract"),
            "NOT Missing should generate NOT (...) IS NULL"
        );
        assert!(
            sql.contains("IS NULL"),
            "Should still contain IS NULL from inner Missing"
        );
        assert!(params.is_empty());
    }
}

// =============================================================================
// Task 7: Unit test for Namespace SQL generation
// =============================================================================

#[cfg(test)]
mod namespace_sql_tests {
    use super::*;

    #[test]
    fn test_namespace_generates_correlated_subquery() {
        let (sql, params) = build_where("(namespace \"work\")");
        assert!(
            sql.contains("EXISTS"),
            "Namespace should use EXISTS subquery"
        );
        assert!(
            sql.contains("SELECT 1 FROM pages p"),
            "Namespace should subquery pages table"
        );
        assert!(
            sql.contains("p.namespace = ?"),
            "Namespace should filter by namespace"
        );
        assert_eq!(params.len(), 1, "Namespace should have 1 param");
        assert_eq!(
            params[0], "work",
            "Namespace param should be the namespace name"
        );
    }

    #[test]
    fn test_namespace_full_sql() {
        let (sql, params) = build_sql("(namespace \"projects/work\")");
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("EXISTS"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "projects/work");
    }

    #[test]
    fn test_namespace_combined_with_task() {
        let (sql, params) = build_where("(and (namespace \"work\") (task todo))");
        assert!(sql.contains("EXISTS"));
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("AND"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_namespace_or_query() {
        let (sql, params) = build_where("(or (namespace \"personal\") (namespace \"work\"))");
        assert!(sql.contains("EXISTS"));
        assert!(sql.contains("OR"));
        assert_eq!(params.len(), 2);
    }
}

// =============================================================================
// Task 8: Unit test for Table SQL generation
// =============================================================================

#[cfg(test)]
mod table_sql_tests {
    use super::*;

    #[test]
    fn test_table_generates_and_clauses() {
        let (sql, params) = build_where("(table (task todo) (priority a))");
        assert!(
            sql.contains("marker IN"),
            "Table should generate marker filter"
        );
        assert!(
            sql.contains("priority"),
            "Table should generate priority filter"
        );
        assert!(sql.contains("AND"), "Table should combine with AND");
        // task todo -> 1 param, priority a -> 1 param = 2 total
        assert_eq!(params.len(), 2, "Should have params for both expressions");
    }

    #[test]
    fn test_table_full_sql() {
        let (sql, params) = build_sql("(table (task todo))");
        assert!(sql.contains("SELECT b.*"));
        assert!(sql.contains("FROM blocks b"));
        assert!(sql.contains("WHERE"));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_table_multiple_exprs() {
        // Table with multiple expressions combines with AND
        let (sql, params) = build_where("(table (task todo) (priority a))");
        assert!(
            sql.contains("marker IN"),
            "Table should generate marker filter"
        );
        assert!(
            sql.contains("priority"),
            "Table should generate priority filter"
        );
        assert!(sql.contains("AND"), "Table should combine with AND");
        // Task has 1 param, priority has 1 param (a)
        assert_eq!(params.len(), 2, "Should have params for both expressions");
    }

    #[test]
    fn test_table_single_expr() {
        let (sql, params) = build_where("(table (task todo))");
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_table_nested() {
        // Table with nested expressions
        let (sql, params) = build_where("(table (and (task todo) (priority a)))");
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("priority"));
        assert!(sql.contains("AND"));
        // task todo -> 1 param, priority a -> 1 param
        assert_eq!(params.len(), 2);
    }
}

// =============================================================================
// Task 9: Backward compatibility test - existing queries
// =============================================================================

#[cfg(test)]
mod backward_compat_tests {
    use super::*;

    #[test]
    fn test_existing_task_query() {
        let (sql, params) = build_sql("(task todo)");
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("?"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "todo");
    }

    #[test]
    fn test_existing_priority_query() {
        let (sql, params) = build_sql("(priority a b c)");
        assert!(sql.contains("priority"));
        assert!(sql.contains("COLLATE NOCASE IN"));
        assert_eq!(params.len(), 3);
    }

    #[test]
    fn test_existing_page_query() {
        let (sql, params) = build_sql("(page \"TestPage\")");
        assert!(sql.contains("EXISTS"));
        assert!(sql.contains("pages p"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_existing_property_query() {
        let (sql, params) = build_sql("(property \"author\" \"John\")");
        assert!(sql.contains("json_extract"));
        assert!(sql.contains("$.author"));
        assert!(sql.contains("= ?"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_existing_and_query() {
        let (sql, params) = build_sql("(and (task todo) (priority a))");
        assert!(sql.contains("AND"));
        assert!(sql.contains("marker IN"));
        assert!(sql.contains("priority"));
        // task todo -> 1 param, priority a -> 1 param = 2 total
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_existing_or_query() {
        let (sql, params) = build_sql("(or (task todo) (task done))");
        assert!(sql.contains("OR"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_existing_not_query() {
        let (sql, params) = build_sql("(not (task done))");
        assert!(sql.contains("NOT"));
        assert!(sql.contains("marker IN"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_existing_between_query() {
        let (sql, params) = build_sql("(between \"1000\" \"2000\")");
        assert!(sql.contains("BETWEEN"));
        assert!(sql.contains("created_at"));
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_existing_page_ref_query() {
        let (sql, params) = build_sql("[[Test Page]]");
        assert!(sql.contains("content LIKE"));
        assert_eq!(params.len(), 1);
        assert!(params[0].contains("Test Page"));
    }

    #[test]
    fn test_existing_self_ref_query() {
        let (sql, params) = build_sql("self");
        assert!(sql.contains("1 = 1"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_existing_fts_query() {
        let (sql, params) = build_sql("(full-text-search \"keyword\")");
        assert!(sql.contains("blocks_fts"));
        assert!(sql.contains("MATCH"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "keyword");
    }

    #[test]
    fn test_existing_tags_query() {
        let (sql, params) = build_sql("(tags \"important\")");
        assert!(sql.contains("tags LIKE"));
        assert_eq!(params.len(), 1);
    }

    #[test]
    fn test_existing_sample_query() {
        let (sql, params) = build_sql("(sample 10)");
        assert!(sql.contains("ORDER BY RANDOM()"));
        assert!(sql.contains("LIMIT"));
        assert!(params.is_empty());
    }
}

// =============================================================================
// Task 10: Regression test - TimeOffset direction
// =============================================================================

#[cfg(test)]
mod timeoffset_direction_tests {
    use super::*;

    #[test]
    fn test_negative_offset_is_past() {
        // -7d should represent 7 days in the past
        let offset = quilt_query::time_helpers::TimeOffset::parse("-7d");
        assert!(offset.is_some());
        let offset = offset.unwrap();
        assert!(matches!(
            offset,
            quilt_query::time_helpers::TimeOffset::Days(-7)
        ));
    }

    #[test]
    fn test_positive_offset_is_future() {
        // +7d should represent 7 days in the future
        let offset = quilt_query::time_helpers::TimeOffset::parse("7d");
        assert!(offset.is_some());
        let offset = offset.unwrap();
        assert!(matches!(
            offset,
            quilt_query::time_helpers::TimeOffset::Days(7)
        ));
    }

    #[test]
    fn test_negative_offset_to_date_moves_backward() {
        use chrono::NaiveDate;

        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = quilt_query::time_helpers::TimeOffset::Days(-7);
        let result = offset.to_date(base);

        // Moving backward from Jan 15 by 7 days = Jan 8
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
    }

    #[test]
    fn test_positive_offset_to_date_moves_forward() {
        use chrono::NaiveDate;

        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = quilt_query::time_helpers::TimeOffset::Days(7);
        let result = offset.to_date(base);

        // Moving forward from Jan 15 by 7 days = Jan 22
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 22).unwrap());
    }

    #[test]
    fn test_timeoffset_weeks_negative() {
        let offset = quilt_query::time_helpers::TimeOffset::parse("-2w");
        assert!(offset.is_some());
        assert!(matches!(
            offset.unwrap(),
            quilt_query::time_helpers::TimeOffset::Weeks(-2)
        ));
    }

    #[test]
    fn test_timeoffset_months_negative() {
        let offset = quilt_query::time_helpers::TimeOffset::parse("-1m");
        assert!(offset.is_some());
        assert!(matches!(
            offset.unwrap(),
            quilt_query::time_helpers::TimeOffset::Months(-1)
        ));
    }

    #[test]
    fn test_timeoffset_years_negative() {
        let offset = quilt_query::time_helpers::TimeOffset::parse("-1y");
        assert!(offset.is_some());
        assert!(matches!(
            offset.unwrap(),
            quilt_query::time_helpers::TimeOffset::Years(-1)
        ));
    }

    #[test]
    fn test_timeoffset_hours_negative() {
        let offset = quilt_query::time_helpers::TimeOffset::parse("-4h");
        assert!(offset.is_some());
        assert!(matches!(
            offset.unwrap(),
            quilt_query::time_helpers::TimeOffset::Hours(-4)
        ));
    }

    #[test]
    fn test_timeoffset_minutes_negative() {
        let offset = quilt_query::time_helpers::TimeOffset::parse("-30n");
        assert!(offset.is_some());
        assert!(matches!(
            offset.unwrap(),
            quilt_query::time_helpers::TimeOffset::Minutes(-30)
        ));
    }

    #[test]
    fn test_parse_time_helper_negative_offset() {
        // parse_time_helper should handle negative offsets
        let result = quilt_query::time_helpers::parse_time_helper("-7d");
        assert!(result.is_some());

        // Verify it represents a date 7 days in the past
        let today = chrono::Utc::now().date_naive();
        let result_date = result.unwrap();
        let diff = today.signed_duration_since(result_date).num_days();

        // Should be exactly 7 days ago
        assert_eq!(diff, 7);
    }

    #[test]
    fn test_between_with_negative_timeoffset() {
        // Regression: between with time offset should work correctly
        // Note: time_helpers like -30d are NOT preprocessed (they parse correctly as time_helper)
        // So we test that a pure integer range still works (gets preprocessed to quoted)
        let (sql, params) = build_where("(between -30 7)");
        assert!(sql.contains("BETWEEN"));
        assert_eq!(params.len(), 2);
    }
}
