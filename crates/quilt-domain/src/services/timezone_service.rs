//! TimezoneService - Domain service for timezone-aware date/time operations
//!
//! This service centralizes all timezone-aware date calculations to ensure
//! consistent handling across the application.

use crate::errors::DomainError;
use crate::value_objects::JournalDay;
use chrono::DateTime;
use chrono_tz::Tz;

/// Service for handling timezone-aware date/time operations.
///
/// All journal-related date calculations MUST go through this service
/// to ensure consistent timezone handling across the application.
#[derive(Debug, Clone)]
pub struct TimezoneService {
    timezone: Tz,
}

impl TimezoneService {
    /// Create a TimezoneService from a timezone string.
    ///
    /// # Arguments
    /// * `tz` - IANA timezone identifier (e.g., "America/Mexico_City", "Europe/Madrid")
    ///
    /// # Errors
    /// Returns `DomainError::InvalidTimezone` if the timezone string is not valid.
    ///
    /// # Examples
    /// ```ignore
    /// let tz = crate::TimezoneService::from_tz_string("America/Mexico_City").unwrap();
    /// let today = tz.today_journal_day();
    /// ```
    pub fn from_tz_string(tz: &str) -> Result<Self, DomainError> {
        tz.parse::<Tz>()
            .map(|tz| Self { timezone: tz })
            .map_err(|_| DomainError::InvalidTimezone(tz.to_string()))
    }

    /// Create a TimezoneService from a chrono_tz::Tz.
    ///
    /// Use this when you already have a parsed timezone.
    pub fn from_tz(timezone: Tz) -> Self {
        Self { timezone }
    }

    /// Get current datetime in the user's timezone.
    ///
    /// This is the primary method for getting "now" in the user's local time.
    /// All UI displays should use this method, not `Utc::now()`.
    pub fn now(&self) -> DateTime<Tz> {
        chrono::Utc::now().with_timezone(&self.timezone)
    }

    /// Get today's JournalDay in the user's timezone.
    ///
    /// This is the CORRECT implementation for "today".
    /// Uses local time, not UTC.
    ///
    /// # Examples
    /// ```ignore
    /// // If in CDMX at 1:00 AM May 15, returns JournalDay for May 15
    /// ```
    pub fn today_journal_day(&self) -> JournalDay {
        let local_now = self.now();
        // Convert to UTC for JournalDay (which stores UTC internally)
        let utc_with_tz = local_now.with_timezone(&chrono::Utc);
        JournalDay::from_datetime(&utc_with_tz)
    }

    /// Convert a UTC DateTime to local time in user's timezone.
    pub fn utc_to_local(&self, utc: DateTime<chrono::Utc>) -> DateTime<Tz> {
        utc.with_timezone(&self.timezone)
    }

    /// Convert a local DateTime to UTC.
    pub fn local_to_utc(&self, local: DateTime<Tz>) -> DateTime<chrono::Utc> {
        local.with_timezone(&chrono::Utc)
    }

    /// Get the timezone identifier string.
    pub fn timezone_id(&self) -> &'static str {
        self.timezone.name()
    }

    /// Get yesterday's JournalDay in user's timezone.
    pub fn yesterday_journal_day(&self) -> Option<JournalDay> {
        self.today_journal_day().yesterday()
    }

    /// Get tomorrow's JournalDay in user's timezone.
    pub fn tomorrow_journal_day(&self) -> Option<JournalDay> {
        self.today_journal_day().tomorrow()
    }

    /// Get a JournalDay offset from today.
    ///
    /// # Arguments
    /// * `days` - Number of days to add (positive) or subtract (negative)
    pub fn offset_journal_day(&self, days: i64) -> Option<JournalDay> {
        self.today_journal_day().add_days(days)
    }

    /// Check if a given UTC datetime falls on a different day in the user's timezone.
    ///
    /// This is useful for determining if a UTC timestamp "belongs" to today or tomorrow/yesterday
    /// in the user's local timezone (e.g., for 11:59 PM UTC check-ins).
    pub fn utc_is_same_local_day(&self, utc: DateTime<chrono::Utc>, day: JournalDay) -> bool {
        let local = self.utc_to_local(utc);
        let local_day = JournalDay::from_naive_date(local.date_naive());
        local_day == day
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utc_timezone() {
        // UTC timezone should behave like using Utc::now()
        let tz = TimezoneService::from_tz_string("UTC").unwrap();
        let today = tz.today_journal_day();
        let expected = JournalDay::today();
        assert_eq!(today, expected);
    }

    #[test]
    fn test_invalid_timezone() {
        let result = TimezoneService::from_tz_string("Invalid/Timezone");
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidTimezone(tz) => assert_eq!(tz, "Invalid/Timezone"),
            _ => panic!("Expected InvalidTimezone error"),
        }
    }

    #[test]
    fn test_timezone_id() {
        let tz = TimezoneService::from_tz_string("America/Mexico_City").unwrap();
        assert_eq!(tz.timezone_id(), "America/Mexico_City");
    }

    #[test]
    fn test_offset_journal_day() {
        let tz = TimezoneService::from_tz_string("UTC").unwrap();
        let today = tz.today_journal_day();
        let tomorrow = tz.tomorrow_journal_day().unwrap();
        let yesterday = tz.yesterday_journal_day().unwrap();

        assert_eq!(tomorrow, today.add_days(1).unwrap());
        assert_eq!(yesterday, today.add_days(-1).unwrap());
    }
}
