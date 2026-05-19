//! Timestamps value object - represents physical creation and update timestamps
//!
//! This value object encapsulates the database-level timestamps for a block:
//! when it was physically created and when it was last updated.
//!
//! # Design Rationale
//!
//! Timestamps tracks when data was physically stored in the database.
//! This is different from JournalEntry which tracks when content "belongs"
//! to a journal day in the user's timezone.
//!
//! Example: A block created at 11:59 PM in America/Mexico_City (UTC-6)
//! would have:
//! - `created_at` = 2026-05-16T05:59:00Z (UTC timestamp)
//! - `journal_day` = 20260515 (because in CDMX it's still May 15)
//!
//! Use `Timestamps` for:
//! - Database operations (optimistic locking, caching)
#![allow(dead_code)]
//! - Audit trails
//!
//! Use `JournalEntry` for:
//! - Activity streams ("updated today")
//! - Journal queries

use chrono::{DateTime, Utc};

/// Timestamps represents the physical storage timestamps of a block.
///
/// It encapsulates:
/// - `created_at`: When the block was first created in the database
/// - `updated_at`: When the block was last modified in the database
///
/// # Invariants
///
/// - `updated_at` should always be >= `created_at`
/// - Both fields are always Some in normal operation
#[derive(Debug, Clone, PartialEq)]
pub struct Timestamps {
    /// Creation timestamp in UTC
    pub created_at: DateTime<Utc>,
    /// Last update timestamp in UTC
    pub updated_at: DateTime<Utc>,
}

impl Timestamps {
    /// Create new timestamps with the same value for both fields.
    ///
    /// Used when creating a new entity.
    pub fn new(now: DateTime<Utc>) -> Self {
        Self {
            created_at: now,
            updated_at: now,
        }
    }

    /// Create timestamps with different created_at and updated_at.
    ///
    /// Use with caution - normally both should be the same at creation.
    pub fn with_update_time(created_at: DateTime<Utc>, updated_at: DateTime<Utc>) -> Self {
        Self {
            created_at,
            updated_at,
        }
    }

    /// Update the updated_at timestamp to the current time.
    ///
    /// Call this when the entity is modified.
    pub fn touch(&mut self, now: DateTime<Utc>) {
        self.updated_at = now;
    }

    /// Check if this entity has been updated since creation.
    pub fn was_ever_updated(&self) -> bool {
        self.updated_at > self.created_at
    }

    /// Get the time elapsed since creation.
    pub fn age(&self) -> chrono::Duration {
        self.updated_at - self.created_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamps_new() {
        let now = Utc::now();
        let ts = Timestamps::new(now);
        
        assert_eq!(ts.created_at, now);
        assert_eq!(ts.updated_at, now);
    }

    #[test]
    fn test_timestamps_touch() {
        let now = Utc::now();
        let later = now + chrono::Duration::seconds(10);
        
        let mut ts = Timestamps::new(now);
        ts.touch(later);
        
        assert_eq!(ts.created_at, now);
        assert_eq!(ts.updated_at, later);
    }

    #[test]
    fn test_was_ever_updated() {
        let now = Utc::now();
        let ts = Timestamps::new(now);
        
        assert!(!ts.was_ever_updated());
        
        let mut ts = Timestamps::new(now);
        ts.touch(now + chrono::Duration::seconds(1));
        
        assert!(ts.was_ever_updated());
    }

    #[test]
    fn test_age() {
        let now = Utc::now();
        let ts = Timestamps::new(now);
        let age = ts.age();
        
        assert_eq!(age, chrono::Duration::zero());
    }
}
