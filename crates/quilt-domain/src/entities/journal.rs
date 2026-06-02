//! Journal entity - a specialized page for daily notes

use crate::entities::Page;
use crate::value_objects::JournalDay;

/// Journal is a specialized Page variant for daily notes.
///
/// Journals are automatically created for each day and have a specific
/// naming convention (YYYY-MM-DD format).
#[derive(Debug, Clone, PartialEq)]
pub struct Journal {
    /// The underlying page
    page: Page,
    /// The journal day (YYYYMMDD as integer)
    journal_day: JournalDay,
}

impl Journal {
    /// Create a new journal for the given day
    pub fn new(
        day: JournalDay,
        format: crate::value_objects::BlockFormat,
        journal_format: &str,
    ) -> Result<Self, crate::errors::DomainError> {
        let page = Page::new_journal(day, format, journal_format)?;
        Ok(Self {
            page,
            journal_day: day,
        })
    }

    /// Get the underlying page
    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Get the journal day
    pub fn day(&self) -> JournalDay {
        self.journal_day
    }

    /// Get the journal name (e.g., "2026-05-02")
    pub fn name(&self) -> &str {
        &self.page.name
    }
}

impl std::ops::Deref for Journal {
    type Target = Page;

    fn deref(&self) -> &Self::Target {
        &self.page
    }
}

impl From<Journal> for Page {
    fn from(journal: Journal) -> Self {
        journal.page
    }
}

impl TryFrom<Page> for Journal {
    type Error = crate::errors::DomainError;

    fn try_from(page: Page) -> Result<Self, Self::Error> {
        let journal_day = page.journal_day.ok_or_else(|| {
            crate::errors::DomainError::InvalidPageType("Page is not a journal".to_string())
        })?;

        Ok(Self { page, journal_day })
    }
}
