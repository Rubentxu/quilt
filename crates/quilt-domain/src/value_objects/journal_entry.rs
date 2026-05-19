//! JournalEntry value object - represents journal day tracking for a block
//!
//! This value object encapsulates the journal day information for a block.
//! Unlike a simple i32, this is a proper domain concept representing
//! when the block was created and last updated in the journal.
//!
//! # Design Rationale
//!
//! JournalEntry is a domain concept, like an email's "inbox timestamp".
//! It's NOT the same as `Timestamps` which tracks when data was physically
//! created/updated in the database. JournalEntry tracks when the content
//! "belongs" to a particular journal day in the user's timezone.
//!
//! This distinction is important because:
//! - A block created at 11:59 PM in America/Mexico_City belongs to that day
//!   even if the UTC timestamp is technically the next calendar day
//! - Journal queries rely on journal_day, not just created_at

use super::JournalDay;

/// JournalEntry represents journal day tracking for a block.
///
/// It captures:
/// - `journal_day`: The journal day when this block was created (YYYYMMDD)
/// - `updated_journal_day`: The journal day when this block was last updated (YYYYMMDD)
///
/// # Invariants
///
/// - Both fields are Option<i32> (None for pre-migration blocks or orphans)
/// - `updated_journal_day` should always be >= `journal_day` when both are Some
///
/// # Journal Day Semantics
///
/// - Journal day is based on user's timezone, not UTC
/// - A block's journal_day is set at creation and only changes when
///   content实质性 changes (not just metadata like collapsed state)
#[derive(Debug, Clone, PartialEq)]
pub struct JournalEntry {
    /// Journal day when this block was created (YYYYMMDD format).
    ///
    /// This is a denormalized field for efficient queries.
    /// Set automatically when the block is created based on user's timezone.
    ///
    /// None means the block was created before this feature
    /// was implemented (migration case) or is an orphan block.
    pub journal_day: Option<JournalDay>,
    /// Journal day when this block was last updated (YYYYMMDD format).
    ///
    /// Updated on every content change, move, or property change.
    /// Used for the "updated today" activity stream.
    ///
    /// None means the block has never been updated since migration.
    pub updated_journal_day: Option<JournalDay>,
}

impl JournalEntry {
    /// Create a new journal entry at the current journal day.
    ///
    /// Both journal_day and updated_journal_day are set to the same value.
    pub fn new(journal_day: JournalDay) -> Self {
        Self {
            journal_day: Some(journal_day),
            updated_journal_day: Some(journal_day),
        }
    }

    /// Create an empty journal entry (for migrated blocks).
    ///
    /// This indicates the block predates the journal day feature.
    pub fn none() -> Self {
        Self {
            journal_day: None,
            updated_journal_day: None,
        }
    }

    /// Update the updated_journal_day.
    ///
    /// Call this when the block content changes.
    pub fn touch(&mut self, journal_day: JournalDay) {
        self.updated_journal_day = Some(journal_day);
    }

    /// Check if this entry has any journal day info.
    pub fn has_journal_day(&self) -> bool {
        self.journal_day.is_some()
    }

    /// Check if this entry was migrated (has no journal day info).
    pub fn is_migrated(&self) -> bool {
        self.journal_day.is_none()
    }

    /// Check if the block was updated today (in user's timezone).
    pub fn was_updated_today(&self, today: JournalDay) -> bool {
        self.updated_journal_day == Some(today)
    }

    /// Check if the block was created today (in user's timezone).
    pub fn was_created_today(&self, today: JournalDay) -> bool {
        self.journal_day == Some(today)
    }

    /// Get the age of this entry in days.
    ///
    /// Returns None if either journal_day is None.
    pub fn age_days(&self) -> Option<i64> {
        match (self.journal_day, self.updated_journal_day) {
            (Some(created), Some(updated)) => Some(updated - created),
            _ => None,
        }
    }
}

impl Default for JournalEntry {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_entry_new() {
        let day = JournalDay::from_ymd(2026, 5, 15).unwrap();
        let entry = JournalEntry::new(day);
        
        assert_eq!(entry.journal_day, Some(day));
        assert_eq!(entry.updated_journal_day, Some(day));
    }

    #[test]
    fn test_journal_entry_touch() {
        let day1 = JournalDay::from_ymd(2026, 5, 15).unwrap();
        let day2 = JournalDay::from_ymd(2026, 5, 16).unwrap();
        
        let mut entry = JournalEntry::new(day1);
        entry.touch(day2);
        
        assert_eq!(entry.journal_day, Some(day1));
        assert_eq!(entry.updated_journal_day, Some(day2));
    }

    #[test]
    fn test_journal_entry_none() {
        let entry = JournalEntry::none();
        assert!(entry.is_migrated());
        assert!(!entry.has_journal_day());
    }

    #[test]
    fn test_was_updated_today() {
        let day = JournalDay::today();
        let entry = JournalEntry::new(day);
        
        assert!(entry.was_created_today(day));
    }

    #[test]
    fn test_age_days() {
        let day1 = JournalDay::from_ymd(2026, 5, 10).unwrap();
        let day2 = JournalDay::from_ymd(2026, 5, 15).unwrap();
        
        let entry = JournalEntry {
            journal_day: Some(day1),
            updated_journal_day: Some(day2),
        };
        
        assert_eq!(entry.age_days(), Some(5));
    }
}
