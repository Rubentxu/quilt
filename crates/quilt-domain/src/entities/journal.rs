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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::PageCreate;
    use crate::errors::DomainError;
    use crate::value_objects::BlockFormat;

    // ── Helpers ──────────────────────────────────────────────────

    fn sample_journal_day() -> JournalDay {
        JournalDay::from_ymd(2026, 6, 2).unwrap()
    }

    fn sample_format() -> BlockFormat {
        BlockFormat::Markdown
    }

    fn sample_journal_format() -> String {
        "yyyy-MM-dd".to_string()
    }

    // ── new (construction) ───────────────────────────────────────

    #[test]
    fn test_new_journal_succeeds() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format())
            .expect("should create journal");
        assert_eq!(journal.day(), day);
    }

    #[test]
    fn test_new_journal_has_correct_name() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        // The name should be the formatted journal day
        assert_eq!(journal.name(), "2026-06-02");
    }

    // ── page() access ────────────────────────────────────────────

    #[test]
    fn test_page_returns_inner_page() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        let page = journal.page();
        assert_eq!(page.name, "2026-06-02");
    }

    // ── day() access ─────────────────────────────────────────────

    #[test]
    fn test_day_returns_journal_day() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        assert_eq!(journal.day(), day);
    }

    // ── name() ───────────────────────────────────────────────────

    #[test]
    fn test_name_is_same_as_page_name() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        assert_eq!(journal.name(), journal.page().name.as_str());
    }

    // ── Deref to Page ────────────────────────────────────────────

    #[test]
    fn test_deref_gives_page_behavior() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        // Through Deref we can access Page fields directly
        assert_eq!(journal.name, "2026-06-02");
        assert_eq!(journal.format, sample_format());
    }

    // ── From<Journal> for Page ───────────────────────────────────

    #[test]
    fn test_from_journal_into_page() {
        let day = sample_journal_day();
        let journal = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        let original_name = journal.name().to_string();
        let page: Page = journal.into();
        assert_eq!(page.name, original_name);
    }

    // ── TryFrom<Page> for Journal ────────────────────────────────

    #[test]
    fn test_try_from_page_valid_journal() {
        let day = sample_journal_day();
        // Create a journal page via Page::new_journal
        let page = Page::new_journal(day, sample_format(), &sample_journal_format()).unwrap();
        let journal = Journal::try_from(page).expect("page should be convertible to journal");
        assert_eq!(journal.day(), day);
    }

    #[test]
    fn test_try_from_page_not_a_journal() {
        // Create a normal page (not a journal)
        let page = Page::new(PageCreate {
            name: "Regular Page".to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
            source_path: None,
            source_mtime: None,
        })
        .unwrap();
        let result = Journal::try_from(page);
        assert!(result.is_err());
        match result.unwrap_err() {
            DomainError::InvalidPageType(msg) => {
                assert!(msg.contains("not a journal"));
            }
            other => panic!("expected InvalidPageType, got {:?}", other),
        }
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn test_journals_with_same_day_are_independent() {
        let day = sample_journal_day();
        let j1 = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        let j2 = Journal::new(day, sample_format(), &sample_journal_format()).unwrap();
        // They have different pages (different UUIDs)
        assert_ne!(j1.page().id, j2.page().id);
        assert_eq!(j1.day(), j2.day());
    }

    #[test]
    fn test_different_journal_formats_produce_different_titles() {
        // The name (used for stable lookups) is always canonical ISO (YYYY-MM-DD).
        // The title varies based on journal_format (e.g., MM-dd-yyyy vs yyyy-MM-dd).
        let day = sample_journal_day();
        let j1 = Journal::new(day, sample_format(), &"MM-dd-yyyy".to_string()).unwrap();
        let j2 = Journal::new(day, sample_format(), &"yyyy/MM/dd".to_string()).unwrap();
        // Names are the same (canonical ISO for stable lookups)
        assert_eq!(j1.name(), j2.name());
        // But titles differ (the format controls the display title)
        assert_ne!(j1.page().title, j2.page().title);
    }
}
