//! Schedule value object - represents scheduling information for a block
//!
//! This value object encapsulates the scheduling-related fields of a block:
/// scheduled date/time, deadline, start time, and repeat configuration.

use chrono::{DateTime, Utc};

/// Schedule represents the scheduling state of a block.
///
/// It encapsulates:
/// - `scheduled`: Scheduled date/time for the task
/// - `deadline`: Deadline date/time
/// - `start_time`: Start time for duration tracking
/// - `repeated`: Repeated task configuration (next occurrence)
///
/// # Invariants
///
/// - All fields are optional
/// - `repeated` indicates this is a recurring task
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// Scheduled date/time
    pub scheduled: Option<DateTime<Utc>>,
    /// Deadline date/time
    pub deadline: Option<DateTime<Utc>>,
    /// Start time for duration tracking
    pub start_time: Option<DateTime<Utc>>,
    /// Repeated task configuration (next occurrence)
    pub repeated: Option<DateTime<Utc>>,
}

impl Schedule {
    /// Create an empty schedule (no scheduling info).
    pub fn none() -> Self {
        Self {
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
        }
    }

    /// Create a schedule with only a scheduled date.
    pub fn scheduled(dt: DateTime<Utc>) -> Self {
        Self {
            scheduled: Some(dt),
            deadline: None,
            start_time: None,
            repeated: None,
        }
    }

    /// Create a schedule with scheduled and deadline.
    pub fn with_deadline(scheduled: DateTime<Utc>, deadline: DateTime<Utc>) -> Self {
        Self {
            scheduled: Some(scheduled),
            deadline: Some(deadline),
            start_time: None,
            repeated: None,
        }
    }

    /// Create a schedule with a start time.
    pub fn with_start(start_time: DateTime<Utc>) -> Self {
        Self {
            scheduled: None,
            deadline: None,
            start_time: Some(start_time),
            repeated: None,
        }
    }

    /// Check if this schedule is empty (no scheduling info).
    pub fn is_empty(&self) -> bool {
        self.scheduled.is_none()
            && self.deadline.is_none()
            && self.start_time.is_none()
            && self.repeated.is_none()
    }

    /// Check if this is a repeated/recurring task.
    pub fn is_repeated(&self) -> bool {
        self.repeated.is_some()
    }

    /// Set the scheduled time.
    pub fn set_scheduled(&mut self, dt: Option<DateTime<Utc>>) {
        self.scheduled = dt;
    }

    /// Set the deadline.
    pub fn set_deadline(&mut self, dt: Option<DateTime<Utc>>) {
        self.deadline = dt;
    }

    /// Set the start time.
    pub fn set_start_time(&mut self, dt: Option<DateTime<Utc>>) {
        self.start_time = dt;
    }

    /// Set the repeated configuration.
    pub fn set_repeated(&mut self, dt: Option<DateTime<Utc>>) {
        self.repeated = dt;
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_none() {
        let schedule = Schedule::none();
        assert!(schedule.is_empty());
        assert!(!schedule.is_repeated());
    }

    #[test]
    fn test_schedule_scheduled() {
        let dt = Utc::now();
        let schedule = Schedule::scheduled(dt);
        assert!(!schedule.is_empty());
        assert_eq!(schedule.scheduled, Some(dt));
    }

    #[test]
    fn test_schedule_with_deadline() {
        let scheduled = Utc::now();
        let deadline = scheduled + chrono::Duration::days(7);
        let schedule = Schedule::with_deadline(scheduled, deadline);
        
        assert_eq!(schedule.scheduled, Some(scheduled));
        assert_eq!(schedule.deadline, Some(deadline));
    }

    #[test]
    fn test_schedule_repeated() {
        let dt = Utc::now();
        let mut schedule = Schedule::scheduled(dt);
        schedule.set_repeated(Some(dt + chrono::Duration::days(1)));
        
        assert!(schedule.is_repeated());
    }
}
