//! JournalDay value object - represents a day in the journal (YYYYMMDD)

use chrono::Datelike;
use std::fmt;
use std::ops::Sub;
use std::str::FromStr;

/// JournalDay represents a calendar day in YYYYMMDD format.
///
/// This is stored as i32 internally for efficient database storage,
/// but provides a type-safe API for parsing and formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JournalDay(i32);

impl JournalDay {
    /// Create a JournalDay from year, month, day components
    pub fn from_ymd(year: u16, month: u8, day: u8) -> Option<Self> {
        if (1..=12).contains(&month) && (1..=31).contains(&day) {
            Some(JournalDay(
                (year as i32) * 10000 + (month as i32) * 100 + (day as i32),
            ))
        } else {
            None
        }
    }

    /// Create a JournalDay from a NaiveDate
    pub fn from_naive_date(date: chrono::NaiveDate) -> Self {
        JournalDay(date.year() * 10000 + date.month() as i32 * 100 + date.day() as i32)
    }

    /// Create a JournalDay from a DateTime<Utc>
    pub fn from_datetime(dt: &chrono::DateTime<chrono::Utc>) -> Self {
        Self::from_naive_date(dt.date_naive())
    }

    /// Create a JournalDay from a raw i32 value (YYYYMMDD) without validation.
    /// Prefer [`from_i32`] which validates the value.
    pub const fn from_i32_unchecked(value: i32) -> Self {
        JournalDay(value)
    }

    /// Get the underlying integer value (YYYYMMDD)
    pub fn as_i32(&self) -> i32 {
        self.0
    }

    /// Get the year component
    pub fn year(&self) -> i32 {
        self.0 / 10000
    }

    /// Get the month component (1-12)
    pub fn month(&self) -> i32 {
        (self.0 % 10000) / 100
    }

    /// Get the day component (1-31)
    pub fn day(&self) -> i32 {
        self.0 % 100
    }

    /// Convert to a NaiveDate
    pub fn to_naive_date(&self) -> Option<chrono::NaiveDate> {
        chrono::NaiveDate::from_ymd_opt(self.year(), self.month() as u32, self.day() as u32)
    }

    /// Convert to a DateTime<Utc> at midnight
    pub fn to_datetime(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.to_naive_date()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, chrono::Utc))
    }

    /// Get today's journal day
    pub fn today() -> Self {
        let now = chrono::Utc::now();
        Self::from_datetime(&now)
    }

    /// Add days to this journal day
    pub fn add_days(self, days: i64) -> Option<Self> {
        self.to_naive_date()
            .and_then(|d| d.checked_add_signed(chrono::Duration::days(days)))
            .map(Self::from_naive_date)
    }

    /// Get yesterday's journal day
    pub fn yesterday(&self) -> Option<Self> {
        self.add_days(-1)
    }

    /// Get tomorrow's journal day
    pub fn tomorrow(&self) -> Option<Self> {
        self.add_days(1)
    }

    /// Calculate the number of days between two journal days
    pub fn days_between(&self, other: &JournalDay) -> i64 {
        let date_a = self.to_naive_date();
        let date_b = other.to_naive_date();

        match (date_a, date_b) {
            (Some(a), Some(b)) => (a - b).num_days(),
            _ => 0,
        }
    }

    /// Format the journal day using a strftime pattern.
    /// Returns the ISO format if the pattern is invalid.
    pub fn format_with(&self, pattern: &str) -> String {
        self.to_naive_date()
            .map(|d| d.format(pattern).to_string())
            .unwrap_or_else(|| self.to_string())
    }
}

impl fmt::Display for JournalDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02}",
            self.year(),
            self.month(),
            self.day()
        )
    }
}

impl FromStr for JournalDay {
    type Err = crate::errors::DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        // Try common date formats in order of specificity
        let formats = [
            "%Y-%m-%d",     // ISO: 2026-05-14
            "%d-%m-%Y",     // European: 14-05-2026
            "%d/%m/%Y",     // European slash: 14/05/2026
            "%m/%d/%Y",     // US: 05/14/2026
            "%Y/%m/%d",     // Asian: 2026/05/14
            "%d.%m.%Y",     // German: 14.05.2026
        ];

        for fmt in &formats {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(s, fmt) {
                return Ok(Self::from_naive_date(d));
            }
        }

        // Try parsing as raw YYYYMMDD integer
        if let Ok(value) = s.parse::<i32>() {
            return Self::from_i32(value)
                .ok_or_else(|| crate::errors::DomainError::InvalidJournalDay(s.to_string()));
        }

        Err(crate::errors::DomainError::InvalidJournalDay(s.to_string()))
    }
}

impl JournalDay {
    /// Create from raw i32 (YYYYMMDD)
    pub fn from_i32(value: i32) -> Option<Self> {
        let _year = value / 10000;
        let month = (value % 10000) / 100;
        let day = value % 100;

        // Basic validation
        if (1..=12).contains(&month) && (1..=31).contains(&day) {
            Some(JournalDay(value))
        } else {
            None
        }
    }
}

impl Sub for JournalDay {
    type Output = i64;

    fn sub(self, rhs: JournalDay) -> Self::Output {
        self.days_between(&rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ymd() {
        let day = JournalDay::from_ymd(2026, 5, 2).unwrap();
        assert_eq!(day.as_i32(), 20260502);
        assert_eq!(day.year(), 2026);
        assert_eq!(day.month(), 5);
        assert_eq!(day.day(), 2);
    }

    #[test]
    fn test_display() {
        let day = JournalDay::from_ymd(2026, 5, 2).unwrap();
        assert_eq!(day.to_string(), "2026-05-02");
    }

    #[test]
    fn test_from_str() {
        let day: JournalDay = "2026-05-02".parse().unwrap();
        assert_eq!(day.as_i32(), 20260502);
    }

    #[test]
    fn test_today() {
        let today = JournalDay::today();
        let expected = JournalDay::from_datetime(&chrono::Utc::now());
        assert_eq!(today, expected);
    }

    #[test]
    fn test_add_days() {
        let day = JournalDay::from_ymd(2026, 5, 2).unwrap();
        let next_week = day.add_days(7).unwrap();
        assert_eq!(next_week.as_i32(), 20260509);
    }

    #[test]
    fn test_days_between() {
        let day1 = JournalDay::from_ymd(2026, 5, 1).unwrap();
        let day2 = JournalDay::from_ymd(2026, 5, 10).unwrap();
        assert_eq!(day2 - day1, 9);
    }

    #[test]
    fn test_format_with_iso() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        assert_eq!(day.format_with("%Y-%m-%d"), "2026-05-14");
    }

    #[test]
    fn test_format_with_european() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        assert_eq!(day.format_with("%d-%m-%Y"), "14-05-2026");
    }

    #[test]
    fn test_format_with_full_month() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        assert_eq!(day.format_with("%B %d, %Y"), "May 14, 2026");
    }

    #[test]
    fn test_from_str_iso() {
        let day: JournalDay = "2026-05-14".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }

    #[test]
    fn test_from_str_european_dash() {
        let day: JournalDay = "14-05-2026".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }

    #[test]
    fn test_from_str_european_slash() {
        let day: JournalDay = "14/05/2026".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }

    #[test]
    fn test_from_str_us_slash() {
        let day: JournalDay = "05/14/2026".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }

    #[test]
    fn test_from_str_german_dots() {
        let day: JournalDay = "14.05.2026".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }

    #[test]
    fn test_from_str_raw_integer() {
        let day: JournalDay = "20260514".parse().unwrap();
        assert_eq!(day.as_i32(), 20260514);
    }
}
