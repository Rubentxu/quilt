//! JournalRepository trait - abstraction for journal-specific queries

use crate::entities::{Block, Page};
use crate::errors::DomainError;
use crate::types::DailySummary;
use crate::value_objects::JournalDay;
use async_trait::async_trait;

/// Repository for journal-specific queries.
///
/// This trait provides efficient access to journal-day-based
/// queries without requiring JOINs with the pages table.
#[async_trait]
pub trait JournalRepository: Send + Sync {
    /// Get or create a journal page for a specific day.
    ///
    /// If the journal page doesn't exist, it will be created
    /// with the default format and today's timestamp.
    async fn get_or_create(&self, day: JournalDay) -> Result<Page, DomainError>;

    /// Get all blocks created on a specific journal day.
    async fn get_blocks_created(&self, day: JournalDay) -> Result<Vec<Block>, DomainError>;

    /// Get all blocks updated on a specific journal day.
    /// This includes blocks created on other days but modified on this day.
    async fn get_blocks_updated(&self, day: JournalDay) -> Result<Vec<Block>, DomainError>;

    /// Get all blocks that exist on a journal day (created OR updated).
    async fn get_blocks_for_day(&self, day: JournalDay) -> Result<Vec<Block>, DomainError> {
        let created = self.get_blocks_created(day).await?;
        let updated = self.get_blocks_updated(day).await?;

        // Deduplicate - a block created today also appears in updated
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for block in created.into_iter().chain(updated) {
            if seen.insert(block.id) {
                result.push(block);
            }
        }

        Ok(result)
    }

    /// Get a summary of all activity on a journal day.
    async fn get_daily_summary(&self, day: JournalDay) -> Result<DailySummary, DomainError>;

    /// Get all journal days that have activity in a date range.
    async fn get_active_days(
        &self,
        start: JournalDay,
        end: JournalDay,
    ) -> Result<Vec<JournalDay>, DomainError>;

    /// Get orphan blocks (blocks with no journal_day).
    ///
    /// These are blocks created before the migration or blocks
    /// on non-journal pages.
    async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError>;
}
