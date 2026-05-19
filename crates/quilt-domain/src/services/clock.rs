//! Clock trait and implementations for time access in domain layer
//!
//! This module provides the `Clock` trait that abstracts time operations,
//! allowing the domain layer to be independent of concrete time implementations.
//! This is essential for testing and for maintaining timezone-aware behavior
//! without depending on concrete services.
//!
//! # Design Rationale
//!
//! The domain layer should NOT depend on `TimezoneService` directly because:
//! - It creates a tight coupling between domain logic and infrastructure
//! - It makes testing harder (need to mock the full service)
//! - It violates dependency inversion principle
//!
//! Instead, domain receives a `&dyn Clock` which provides just the methods
//! it needs: `now()` and `today_journal_day()`.

use crate::errors::DomainError;
use crate::value_objects::JournalDay;
use chrono::{DateTime, Utc};

/// Trait for abstracting time operations in the domain layer.
///
/// This trait allows domain entities and services to get current time
/// and journal day without depending on concrete implementations.
/// It is the primary mechanism for time access in the domain layer.
pub trait Clock: Send + Sync {
    /// Get the current datetime in UTC.
    fn now(&self) -> DateTime<Utc>;

    /// Get today's journal day in the user's timezone.
    ///
    /// This is the CORRECT method for "today" in domain logic.
    /// Uses local time, not UTC.
    ///
    /// # Examples
    /// ```ignore
    /// // If in CDMX at 1:00 AM May 15, returns JournalDay for May 15
    /// ```
    fn today_journal_day(&self) -> JournalDay;
}

/// System clock implementation that delegates to TimezoneService.
///
/// This is the production implementation used in the application.
#[derive(Debug, Clone)]
pub struct SystemClock {
    timezone: chrono_tz::Tz,
}

impl SystemClock {
    /// Create a SystemClock from a timezone string.
    ///
    /// # Arguments
    /// * `tz` - IANA timezone identifier (e.g., "America/Mexico_City", "Europe/Madrid")
    ///
    /// # Errors
    /// Returns `DomainError::InvalidTimezone` if the timezone string is not valid.
    pub fn from_tz_string(tz: &str) -> Result<Self, DomainError> {
        tz.parse::<chrono_tz::Tz>()
            .map(|timezone| Self { timezone })
            .map_err(|_| DomainError::InvalidTimezone(tz.to_string()))
    }

    /// Create a SystemClock from a chrono_tz::Tz.
    pub fn from_tz(timezone: chrono_tz::Tz) -> Self {
        Self { timezone }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        // Get current time in the user's timezone, then convert to UTC
        // This ensures we get the correct "local" time
        let local_now = chrono::Utc::now().with_timezone(&self.timezone);
        local_now.with_timezone(&chrono::Utc)
    }

    fn today_journal_day(&self) -> JournalDay {
        let local_now = self.now().with_timezone(&self.timezone);
        let utc_with_tz = local_now.with_timezone(&Utc);
        JournalDay::from_datetime(&utc_with_tz)
    }
}

/// Mock clock implementation for testing.
///
/// This implementation allows precise control over the returned
/// time values, making it ideal for unit tests.
#[derive(Debug, Clone)]
pub struct MockClock {
    now: DateTime<Utc>,
    journal_day: JournalDay,
}

impl MockClock {
    /// Create a new mock clock with the given time values.
    pub fn new(now: DateTime<Utc>, journal_day: JournalDay) -> Self {
        Self { now, journal_day }
    }

    /// Create a mock clock that always returns the same time.
    pub fn at(journal_day: JournalDay) -> Self {
        let datetime = journal_day.to_datetime().unwrap_or_else(Utc::now);
        Self {
            now: datetime,
            journal_day,
        }
    }

    /// Create a mock clock frozen at the current UTC time.
    pub fn frozen() -> Self {
        let now = Utc::now();
        let journal_day = JournalDay::from_datetime(&now);
        Self { now, journal_day }
    }
}

impl Clock for MockClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }

    fn today_journal_day(&self) -> JournalDay {
        self.journal_day
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value_objects::JournalDay;

    #[test]
    fn test_system_clock_utc() {
        let clock = SystemClock::from_tz_string("UTC").unwrap();
        let now = clock.now();
        let journal_day = clock.today_journal_day();
        
        // In UTC, now() should equal Utc::now() and journal_day should match
        assert_eq!(now.offset(), &Utc);
        assert_eq!(journal_day, JournalDay::today());
    }

    #[test]
    fn test_system_clock_invalid_timezone() {
        let result = SystemClock::from_tz_string("Invalid/Timezone");
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_clock_frozen() {
        let clock = MockClock::frozen();
        let now = clock.now();
        let journal_day = clock.today_journal_day();
        
        assert_eq!(journal_day, JournalDay::from_datetime(&now));
    }

    #[test]
    fn test_mock_clock_custom() {
        let journal_day = JournalDay::from_ymd(2026, 5, 15).unwrap();
        let clock = MockClock::at(journal_day);
        
        assert_eq!(clock.today_journal_day(), journal_day);
    }
}
