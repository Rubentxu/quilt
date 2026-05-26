//! Time helpers for queries
//!
//! This module provides utilities for parsing and working with relative
//! time expressions like "-7d" (7 days ago) or "2w" (2 weeks from now).

use chrono::{NaiveDate, Utc};

/// Represents a relative time offset.
///
/// Used to express dates relative to a base date, such as "7 days ago"
/// or "2 weeks from now".
#[derive(Debug, Clone, PartialEq)]
pub enum TimeOffset {
    /// Days offset (positive = future, negative = past)
    Days(i64),
    /// Weeks offset
    Weeks(i64),
    /// Months offset (approximated as 30 days)
    Months(i64),
    /// Years offset (approximated as 365 days)
    Years(i64),
    /// Hours offset
    Hours(i64),
    /// Minutes offset
    Minutes(i64),
}

impl TimeOffset {
    /// Parses a time offset string into a [`TimeOffset`].
    ///
    /// # Format
    ///
    /// - `Nd` → N days (e.g., `7d` = 7 days)
    /// - `Nw` → N weeks (e.g., `2w` = 2 weeks)
    /// - `Nm` → N months (e.g., `3m` = 3 months)
    /// - `Ny` → N years (e.g., `1y` = 1 year)
    /// - `Nh` → N hours (e.g., `-4h` = 4 hours ago)
    /// - `Nn` → N minutes (e.g., `30n` = 30 minutes)
    ///
    /// Prefix with `-` for past dates (e.g., `-7d` = 7 days ago).
    ///
    /// # Arguments
    ///
    /// * `s` - The time offset string to parse
    ///
    /// # Returns
    ///
    /// Returns `Some(TimeOffset)` if parsing succeeds, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_query::time_helpers::TimeOffset;
    ///
    /// assert_eq!(TimeOffset::parse("7d"), Some(TimeOffset::Days(7)));
    /// assert_eq!(TimeOffset::parse("-3d"), Some(TimeOffset::Days(-3)));
    /// assert_eq!(TimeOffset::parse("2w"), Some(TimeOffset::Weeks(2)));
    /// assert_eq!(TimeOffset::parse("foo"), None);
    /// ```
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        let (sign, rest) = if s.starts_with('-') {
            (-1, &s[1..])
        } else {
            (1, s)
        };

        if rest.ends_with('d') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Days(n * sign))
        } else if rest.ends_with('w') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Weeks(n * sign))
        } else if rest.ends_with('m') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Months(n * sign))
        } else if rest.ends_with('y') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Years(n * sign))
        } else if rest.ends_with('h') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Hours(n * sign))
        } else if rest.ends_with('n') {
            let n: i64 = rest[..rest.len() - 1].parse().ok()?;
            Some(TimeOffset::Minutes(n * sign))
        } else {
            None
        }
    }

    /// Converts this time offset to an absolute date relative to `base`.
    ///
    /// # Arguments
    ///
    /// * `base` - The base date to calculate from
    ///
    /// # Returns
    ///
    /// The resulting date after applying the offset.
    ///
    /// # Example
    ///
    /// ```
    /// use chrono::NaiveDate;
    /// use quilt_query::time_helpers::TimeOffset;
    ///
    /// let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
    /// let offset = TimeOffset::Days(7);
    /// let result = offset.to_date(base);
    /// assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 22).unwrap());
    /// ```
    pub fn to_date(&self, base: NaiveDate) -> NaiveDate {
        use chrono::Duration;

        match self {
            TimeOffset::Days(n) => base + Duration::days(*n),
            TimeOffset::Weeks(n) => base + Duration::weeks(*n),
            TimeOffset::Months(n) => base + Duration::days(n * 30),
            TimeOffset::Years(n) => base + Duration::days(n * 365),
            TimeOffset::Hours(n) => base + Duration::hours(*n),
            TimeOffset::Minutes(n) => base + Duration::minutes(*n),
        }
    }
}

/// Parses a time helper string into a date.
///
/// Recognizes the following special values:
/// - `today` → current date
/// - `yesterday` → one day before current date
/// - `tomorrow` → one day after current date
///
/// Otherwise, treats the input as a [`TimeOffset`] string.
///
/// # Arguments
///
/// * `s` - The time helper string to parse
///
/// # Returns
///
/// Returns `Some(NaiveDate)` if parsing succeeds, `None` otherwise.
///
/// # Example
///
/// ```
/// use quilt_query::time_helpers::parse_time_helper;
///
/// assert!(parse_time_helper("today").is_some());
/// assert!(parse_time_helper("yesterday").is_some());
/// assert!(parse_time_helper("-7d").is_some());
/// assert!(parse_time_helper("nope").is_none());
/// ```
pub fn parse_time_helper(s: &str) -> Option<NaiveDate> {
    match s.to_lowercase().as_str() {
        "today" => Some(Utc::now().date_naive()),
        "yesterday" => Some(Utc::now().date_naive() - chrono::Duration::days(1)),
        "tomorrow" => Some(Utc::now().date_naive() + chrono::Duration::days(1)),
        s => {
            let offset = TimeOffset::parse(s)?;
            Some(offset.to_date(Utc::now().date_naive()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_days_positive() {
        let result = TimeOffset::parse("7d");
        assert_eq!(result, Some(TimeOffset::Days(7)));
    }

    #[test]
    fn test_parse_days_negative() {
        let result = TimeOffset::parse("-3d");
        assert_eq!(result, Some(TimeOffset::Days(-3)));
    }

    #[test]
    fn test_parse_weeks() {
        let result = TimeOffset::parse("2w");
        assert_eq!(result, Some(TimeOffset::Weeks(2)));
    }

    #[test]
    fn test_parse_months() {
        let result = TimeOffset::parse("1m");
        assert_eq!(result, Some(TimeOffset::Months(1)));
    }

    #[test]
    fn test_parse_years() {
        let result = TimeOffset::parse("1y");
        assert_eq!(result, Some(TimeOffset::Years(1)));
    }

    #[test]
    fn test_parse_hours() {
        let result = TimeOffset::parse("-4h");
        assert_eq!(result, Some(TimeOffset::Hours(-4)));
    }

    #[test]
    fn test_parse_minutes() {
        let result = TimeOffset::parse("30n");
        assert_eq!(result, Some(TimeOffset::Minutes(30)));
    }

    #[test]
    fn test_parse_invalid() {
        let result = TimeOffset::parse("foo");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_empty() {
        let result = TimeOffset::parse("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_to_date_days() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = TimeOffset::Days(1);
        let result = offset.to_date(base);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 16).unwrap());
    }

    #[test]
    fn test_to_date_weeks() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = TimeOffset::Weeks(1);
        let result = offset.to_date(base);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 22).unwrap());
    }

    #[test]
    fn test_to_date_months() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let offset = TimeOffset::Months(1);
        let result = offset.to_date(base);
        // Months are approximated as 30 days, so March 15 + 30 days = April 14
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 4, 14).unwrap());
    }

    #[test]
    fn test_to_date_years() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let offset = TimeOffset::Years(1);
        let result = offset.to_date(base);
        // Years are approximated as 365 days. Jan 15, 2025 + 365 days = Jan 15, 2026
        assert_eq!(result, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    }

    #[test]
    fn test_to_date_hours() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = TimeOffset::Hours(24);
        let result = offset.to_date(base);
        // Hours ARE added and 24 hours crosses a day boundary
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 16).unwrap());
    }

    #[test]
    fn test_parse_time_helper_today() {
        let result = parse_time_helper("today");
        assert!(result.is_some());
        let expected = Utc::now().date_naive();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_time_helper_yesterday() {
        let result = parse_time_helper("yesterday");
        assert!(result.is_some());
        let expected = Utc::now().date_naive() - chrono::Duration::days(1);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_time_helper_tomorrow() {
        let result = parse_time_helper("tomorrow");
        assert!(result.is_some());
        let expected = Utc::now().date_naive() + chrono::Duration::days(1);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_time_helper_offset() {
        let result = parse_time_helper("-7d");
        assert!(result.is_some());
        // -7d means 7 days in the past (consistent with "yesterday")
        let today = Utc::now().date_naive();
        let diff = today.signed_duration_since(result.unwrap()).num_days();
        // -7d gives 7 days in the PAST, so diff should be positive 7
        assert_eq!(diff, 7);
    }

    #[test]
    fn test_parse_time_helper_invalid() {
        let result = parse_time_helper("nope");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_time_helper_uppercase() {
        // parse_time_helper lowercases the input
        let result = parse_time_helper("TODAY");
        assert!(result.is_some());
        let expected = Utc::now().date_naive();
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn test_parse_positive_weeks() {
        let result = TimeOffset::parse("3w");
        assert_eq!(result, Some(TimeOffset::Weeks(3)));
    }

    #[test]
    fn test_parse_positive_months() {
        let result = TimeOffset::parse("6m");
        assert_eq!(result, Some(TimeOffset::Months(6)));
    }

    #[test]
    fn test_parse_positive_years() {
        let result = TimeOffset::parse("2y");
        assert_eq!(result, Some(TimeOffset::Years(2)));
    }

    #[test]
    fn test_parse_positive_hours() {
        let result = TimeOffset::parse("5h");
        assert_eq!(result, Some(TimeOffset::Hours(5)));
    }

    #[test]
    fn test_parse_positive_minutes() {
        let result = TimeOffset::parse("15n");
        assert_eq!(result, Some(TimeOffset::Minutes(15)));
    }

    #[test]
    fn test_to_date_negative_days() {
        use chrono::NaiveDate;
        let base = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let offset = TimeOffset::Days(-7);
        let result = offset.to_date(base);
        assert_eq!(result, NaiveDate::from_ymd_opt(2024, 1, 8).unwrap());
    }
}
