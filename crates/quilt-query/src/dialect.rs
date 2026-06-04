//! SQL dialect abstraction for query generation.
//!
//! This module provides a pluggable SQL dialect trait that allows
//! [`QueryExecutor`](super::executor::QueryExecutor) to generate SQL for different
//! database backends (SQLite, PostgreSQL, MySQL) without changing the executor logic.

use crate::ast::TemporalRange;
use crate::parser::{AggregateFn, PropertyOp, StatsFn};

/// Kinds of window functions for statistical queries.
///
/// Used internally by [`SqlDialect::window_fn`] to generate
/// SQLite-compatible percentile/median calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowFnKind {
    /// Median (50th percentile)
    Median,
    /// Arbitrary percentile (0-100)
    Percentile(u8),
}

/// SQL dialect trait for database-specific query generation.
///
/// Implement this trait to support different database backends.
/// The default implementation is [`SqliteDialect`] for SQLite.
pub trait SqlDialect: Send + Sync + std::fmt::Debug {
    /// Generates a property path expression for JSON property access.
    ///
    /// # Arguments
    ///
    /// * `property` - The property key to access
    ///
    /// # Returns
    ///
    /// SQL expression that extracts the property from the `properties` JSON column.
    ///
    /// # Example
    ///
    /// ```
    /// # use quilt_query::dialect::{SqlDialect, SqliteDialect};
    /// let dialect = SqliteDialect;
    /// assert_eq!(dialect.property_path("author"), "json_extract(properties, '$.author')");
    /// ```
    fn property_path(&self, property: &str) -> String {
        format!("json_extract(properties, '$.{}')", property)
    }

    /// Generates an aggregate function call SQL expression.
    ///
    /// # Arguments
    ///
    /// * `aggregate_fn` - The aggregate function variant
    /// * `prop_path` - The property path expression (from [`SqlDialect::property_path`])
    ///
    /// # Returns
    ///
    /// SQL expression for the aggregate function applied to the property.
    fn aggregate_fn(&self, aggregate_fn: AggregateFn, prop_path: &str) -> String {
        match aggregate_fn {
            AggregateFn::Count => "COUNT(*)".to_string(),
            AggregateFn::Avg => format!("AVG(CAST({} AS REAL))", prop_path),
            AggregateFn::Sum => format!("SUM(CAST({} AS REAL))", prop_path),
            AggregateFn::Min => format!("MIN(CAST({} AS REAL))", prop_path),
            AggregateFn::Max => format!("MAX(CAST({} AS REAL))", prop_path),
        }
    }

    /// Generates a statistical function call SQL expression.
    ///
    /// # Arguments
    ///
    /// * `compute` - The statistical function variant
    /// * `prop_path` - The property path expression (from [`SqlDialect::property_path`])
    ///
    /// # Returns
    ///
    /// SQL expression for the statistical function applied to the property.
    fn stats_fn(&self, compute: StatsFn, prop_path: &str) -> String {
        match compute {
            StatsFn::Stddev => format!("STDDEV_POP({})", prop_path),
            StatsFn::Variance => format!("VAR_POP({})", prop_path),
            StatsFn::Median => self.window_fn(WindowFnKind::Median, prop_path),
            StatsFn::Percentile(p) => self.window_fn(WindowFnKind::Percentile(p), prop_path),
        }
    }

    /// Generates a window function for percentile/median calculations.
    ///
    /// This is used by [`SqlDialect::stats_fn`] for Median and Percentile variants.
    /// The default implementation uses SQLite's `ROW_NUMBER() OVER ()` pattern.
    ///
    /// # Arguments
    ///
    /// * `kind` - The window function kind (Median or Percentile with threshold)
    /// * `prop_path` - The property path expression
    ///
    /// # Returns
    ///
    /// SQL subquery expression that computes the median/percentile value.
    fn window_fn(&self, kind: WindowFnKind, prop_path: &str) -> String {
        match kind {
            WindowFnKind::Median => {
                // SQLite-compatible: subquery with ROW_NUMBER to find median
                format!(
                    "(SELECT val FROM (SELECT {} as val, \
                     ROW_NUMBER() OVER (ORDER BY {}) as rn, \
                     COUNT(*) OVER () as total FROM blocks b \
                     WHERE {} IS NOT NULL) \
                     WHERE rn = CAST(total * 0.5 AS INTEGER))",
                    prop_path, prop_path, prop_path
                )
            }
            WindowFnKind::Percentile(p) => {
                let frac = p as f64 / 100.0;
                // SQLite-compatible: subquery with ROW_NUMBER for percentile
                format!(
                    "(SELECT val FROM (SELECT {} as val, \
                     ROW_NUMBER() OVER (ORDER BY {}) as rn, \
                     COUNT(*) OVER () as total FROM blocks b \
                     WHERE {} IS NOT NULL) \
                     WHERE rn = CAST(total * {} AS INTEGER))",
                    prop_path, prop_path, prop_path, frac
                )
            }
        }
    }

    /// Generates a CAST expression to convert a value to REAL (double precision).
    ///
    /// # Arguments
    ///
    /// * `expr` - The SQL expression to cast
    ///
    /// # Returns
    ///
    /// SQL expression that casts the input to REAL type.
    fn cast_to_real(&self, expr: &str) -> String {
        format!("CAST({} AS REAL)", expr)
    }

    /// F3 — generates the SQL fragment for a property operator comparison.
    /// `Contains` is bound with `LIKE`; caller wraps value as `%v%`.
    fn property_op_sql(&self, op: PropertyOp, prop_path: &str) -> String {
        match op {
            PropertyOp::Equals => format!("{} = ?", prop_path),
            PropertyOp::NotEquals => format!("{} != ?", prop_path),
            PropertyOp::Contains => format!("{} LIKE ?", prop_path),
            PropertyOp::GreaterThan => format!("{} > ?", prop_path),
            PropertyOp::LessThan => format!("{} < ?", prop_path),
            PropertyOp::GreaterThanOrEqual => format!("{} >= ?", prop_path),
            PropertyOp::LessThanOrEqual => format!("{} <= ?", prop_path),
            PropertyOp::Between => format!("{} BETWEEN ? AND ?", prop_path),
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // G3: Temporal range SQL generation
    // ─────────────────────────────────────────────────────────────────────────

    /// Generates SQL WHERE conditions for a temporal range filter.
    ///
    /// # Week Convention
    ///
    /// **Weeks start on Monday** (ISO 8601 standard). This convention is
    /// shared between `compile_temporal` and `temporal_range_sql`. Documented
    /// here to prevent hidden meaning connascence (~0.8 bits per spec discovery).
    ///
    /// # Arguments
    ///
    /// * `range` - The temporal range to convert to SQL
    ///
    /// # Returns
    ///
    /// A tuple of (sql_fragment, params) suitable for embedding in a WHERE clause.
    fn temporal_range_sql(&self, range: &TemporalRange)
    -> (String, Vec<crate::executor::SqlParam>);
}

/// SQLite-specific SQL dialect implementation.
///
/// This is the default dialect used by [`QueryExecutor`](super::executor::QueryExecutor).
#[derive(Debug, Clone, Copy, Default)]
pub struct SqliteDialect;

impl SqlDialect for SqliteDialect {
    /// Generates SQL WHERE conditions for a temporal range.
    ///
    /// Uses SQLite's `julianday()` function for date arithmetic.
    /// Weeks start on Monday (ISO 8601 convention).
    fn temporal_range_sql(
        &self,
        range: &TemporalRange,
    ) -> (String, Vec<crate::executor::SqlParam>) {
        use crate::executor::SqlParam;
        use crate::time_helpers::TimeOffset;
        use chrono::{Datelike, Local, NaiveDate};

        let today = Local::now().date_naive();

        match range {
            TemporalRange::Today => {
                let start = today.format("%Y-%m-%d").to_string();
                let sql =
                    "date(b.created_at / 1000, 'unixepoch', 'localtime') = date(?,'localtime')"
                        .to_string();
                (sql, vec![SqlParam::String(start)])
            }
            TemporalRange::Yesterday => {
                let yesterday = today - chrono::Duration::days(1);
                let start = yesterday.format("%Y-%m-%d").to_string();
                let sql =
                    "date(b.created_at / 1000, 'unixepoch', 'localtime') = date(?,'localtime')"
                        .to_string();
                (sql, vec![SqlParam::String(start)])
            }
            TemporalRange::ThisWeek => {
                // ISO week starts on Monday
                let weekday = today.weekday().num_days_from_monday();
                let monday = today - chrono::Duration::days(weekday as i64);
                let start = monday.format("%Y-%m-%d").to_string();
                let sql =
                    "date(b.created_at / 1000, 'unixepoch', 'localtime') >= date(?,'localtime')"
                        .to_string();
                (sql, vec![SqlParam::String(start)])
            }
            TemporalRange::LastWeek => {
                // Previous week: Monday to Sunday of last week
                let weekday = today.weekday().num_days_from_monday();
                let this_monday = today - chrono::Duration::days(weekday as i64);
                let last_monday = this_monday - chrono::Duration::days(7);
                let last_sunday = this_monday - chrono::Duration::days(1);
                let start = last_monday.format("%Y-%m-%d").to_string();
                let end = last_sunday.format("%Y-%m-%d").to_string();
                let sql = "date(b.created_at / 1000, 'unixepoch', 'localtime') BETWEEN date(?,'localtime') AND date(?,'localtime')".to_string();
                (sql, vec![SqlParam::String(start), SqlParam::String(end)])
            }
            TemporalRange::ThisMonth => {
                let start = format!("{}-01", today.format("%Y-%m"));
                let sql =
                    "date(b.created_at / 1000, 'unixepoch', 'localtime') >= date(?,'localtime')"
                        .to_string();
                (sql, vec![SqlParam::String(start)])
            }
            TemporalRange::LastMonth => {
                let first_of_this_month =
                    NaiveDate::from_ymd_opt(today.year(), today.month(), 1).expect("valid date");
                let last_month = if today.month() == 1 {
                    NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).expect("valid date")
                } else {
                    NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).expect("valid date")
                };
                // Last day of last month is day before first of this month
                let end = first_of_this_month - chrono::Duration::days(1);
                let start = last_month.format("%Y-%m-%d").to_string();
                let end_str = end.format("%Y-%m-%d").to_string();
                let sql = "date(b.created_at / 1000, 'unixepoch', 'localtime') BETWEEN date(?,'localtime') AND date(?,'localtime')".to_string();
                (
                    sql,
                    vec![SqlParam::String(start), SqlParam::String(end_str)],
                )
            }
            TemporalRange::Custom { start, end } => {
                let sql = "date(b.created_at / 1000, 'unixepoch', 'localtime') BETWEEN date(?,'localtime') AND date(?,'localtime')".to_string();
                (
                    sql,
                    vec![
                        SqlParam::String(start.clone()),
                        SqlParam::String(end.clone()),
                    ],
                )
            }
            TemporalRange::Relative(offset) => {
                let base_date = match offset {
                    TimeOffset::Days(n) => today - chrono::Duration::days(*n),
                    TimeOffset::Weeks(n) => today - chrono::Duration::weeks(*n),
                    TimeOffset::Months(n) => today - chrono::Duration::days(n * 30),
                    TimeOffset::Years(n) => today - chrono::Duration::days(n * 365),
                    TimeOffset::Hours(n) => today - chrono::Duration::hours(*n),
                    TimeOffset::Minutes(n) => today - chrono::Duration::minutes(*n),
                };
                let date_str = base_date.format("%Y-%m-%d").to_string();
                let sql =
                    "date(b.created_at / 1000, 'unixepoch', 'localtime') >= date(?,'localtime')"
                        .to_string();
                (sql, vec![SqlParam::String(date_str)])
            }
        }
    }
}
