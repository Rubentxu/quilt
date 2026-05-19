//! Time abstractions for the domain layer
//!
//! This module provides:
//! - [`Timestamp`]: A value object wrapping `DateTime<Utc>` for single timestamp values
//! - [`Clock`]: A trait for abstracting time operations, enabling testability
//!
//! # Design Rationale
//!
//! Using `DateTime<Utc>` directly in repository signatures creates time coupling:
//! - Testing requires mocking chrono
//! - Time semantics are scattered across the codebase
//!
//! This module centralizes time access through the `Clock` trait and `Timestamp` value object.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Timestamp is a value object that wraps a `DateTime<Utc>`.
///
/// It provides a stable interface for representing instant moments in time,
/// hiding the underlying chrono implementation from callers.
///
/// Unlike `Timestamps` (which tracks created_at + updated_at for entities),
/// `Timestamp` is a single instant suitable for method parameters and return values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a new Timestamp from a DateTime<Utc>.
    ///
    /// Note: Prefer using `Clock::now()` in application code rather than
    /// directly constructing timestamps.
    #[inline]
    pub fn from_datetime(dt: &DateTime<Utc>) -> Self {
        Self(*dt)
    }

    /// Get the inner `DateTime<Utc>` value.
    #[inline]
    pub fn as_datetime(&self) -> DateTime<Utc> {
        self.0
    }

    /// Create a Timestamp for the current moment (UTC).
    ///
    /// This should only be used in infrastructure/testing code.
    /// Application code should receive timestamps via `Clock::now()`.
    #[inline]
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}

impl From<Timestamp> for DateTime<Utc> {
    fn from(ts: Timestamp) -> Self {
        ts.0
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Trait for abstracting time operations in the domain layer.
///
/// This trait allows domain code to get current time without depending
/// on concrete implementations. Implementations can provide:
/// - `SystemClock`: Production time (UTC with timezone awareness)
/// - `MockClock`: Deterministic time for testing
///
/// # Example
///
/// ```ignore
/// struct MyService<C: Clock> {
///     clock: C,
/// }
///
/// impl<C: Clock> MyService<C> {
///     fn do_something(&self) -> Timestamp {
///         self.clock.now()
///     }
/// }
/// ```
pub trait Clock: Send + Sync {
    /// Get the current timestamp in UTC.
    fn now(&self) -> Timestamp;
}

/// Mock clock implementation for testing.
///
/// This provides deterministic timestamps, making tests repeatable.
#[derive(Debug, Clone)]
pub struct MockClock {
    now: Timestamp,
}

impl MockClock {
    /// Create a mock clock frozen at a specific timestamp.
    pub fn at(timestamp: Timestamp) -> Self {
        Self { now: timestamp }
    }

    /// Create a mock clock frozen at the current moment.
    pub fn frozen() -> Self {
        Self { now: Timestamp::now() }
    }
}

impl Clock for MockClock {
    fn now(&self) -> Timestamp {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_now() {
        let ts = Timestamp::now();
        assert_eq!(ts.as_datetime().offset(), &Utc);
    }

    #[test]
    fn test_timestamp_from_datetime() {
        let dt = Utc::now();
        let ts = Timestamp::from_datetime(&dt);
        assert_eq!(ts.as_datetime(), dt);
    }

    #[test]
    fn test_timestamp_conversion() {
        let dt = Utc::now();
        let ts: Timestamp = dt.into();
        let back: DateTime<Utc> = ts.into();
        assert_eq!(dt, back);
    }

    #[test]
    fn test_mock_clock_frozen() {
        let clock = MockClock::frozen();
        let ts = clock.now();
        assert_eq!(ts.as_datetime().offset(), &Utc);
    }

    #[test]
    fn test_mock_clock_at() {
        let ts = Timestamp::now();
        let clock = MockClock::at(ts);
        assert_eq!(clock.now(), ts);
    }
}