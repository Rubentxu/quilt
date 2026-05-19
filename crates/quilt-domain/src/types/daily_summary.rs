//! DailySummary type - aggregation of all activity on a journal day
//!
//! This is the primary data structure for the journal dashboard.

use crate::entities::{Block, Page};
use crate::value_objects::{JournalDay, Uuid};

/// Summary of all activity on a journal day.
///
/// This is the primary data structure for the journal dashboard.
#[derive(Debug, Clone)]
pub struct DailySummary {
    /// The journal day this summary represents
    pub day: JournalDay,

    /// Pages created on this day
    pub pages_created: Vec<Page>,

    /// Pages updated on this day (content or properties changed)
    pub pages_updated: Vec<Page>,

    /// Blocks created on this day
    pub blocks_created: Vec<Block>,

    /// Blocks updated on this day (modified, not including created)
    pub blocks_updated: Vec<Block>,

    /// IDs of blocks deleted on this day (for audit trail)
    pub blocks_deleted: Vec<Uuid>,

    /// Total count of pages touched
    pub total_pages: usize,

    /// Total count of blocks touched
    pub total_blocks: usize,
}

impl DailySummary {
    /// Create a new daily summary.
    pub fn new(day: JournalDay) -> Self {
        Self {
            day,
            pages_created: Vec::new(),
            pages_updated: Vec::new(),
            blocks_created: Vec::new(),
            blocks_updated: Vec::new(),
            blocks_deleted: Vec::new(),
            total_pages: 0,
            total_blocks: 0,
        }
    }

    /// Calculate totals from child collections.
    pub fn compute_totals(&mut self) {
        self.total_pages = self.pages_created.len() + self.pages_updated.len();
        self.total_blocks = self.blocks_created.len() + self.blocks_updated.len();
    }

    /// Check if there is any activity on this day.
    pub fn is_empty(&self) -> bool {
        self.pages_created.is_empty()
            && self.pages_updated.is_empty()
            && self.blocks_created.is_empty()
            && self.blocks_updated.is_empty()
            && self.blocks_deleted.is_empty()
    }

    /// Get the count of new blocks (净新增)
    pub fn net_new_blocks(&self) -> isize {
        self.blocks_created.len() as isize - self.blocks_deleted.len() as isize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_summary() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        let summary = DailySummary::new(day);
        assert_eq!(summary.day, day);
        assert!(summary.is_empty());
        assert_eq!(summary.total_pages, 0);
        assert_eq!(summary.total_blocks, 0);
    }

    #[test]
    fn test_compute_totals() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        let mut summary = DailySummary::new(day);
        summary.compute_totals();
        assert_eq!(summary.total_pages, 0);
        assert_eq!(summary.total_blocks, 0);
    }

    #[test]
    fn test_net_new_blocks() {
        let day = JournalDay::from_ymd(2026, 5, 14).unwrap();
        let summary = DailySummary::new(day);
        // With no blocks, net is 0
        assert_eq!(summary.net_new_blocks(), 0);
    }
}
