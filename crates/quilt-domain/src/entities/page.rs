//! Page entity - represents a page in the knowledge graph

use crate::errors::DomainError;
use crate::value_objects::{BlockFormat, JournalDay, Uuid};

/// Page represents a page in Logseq.
///
/// Pages can be:
/// - Regular pages with a name
/// - Journal pages (daily notes) with a date
/// - Namespace pages (hierarchical organization)
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    /// Unique identifier
    pub id: Uuid,
    /// Canonical name (lowercase, unique)
    pub name: String,
    /// Optional title (for display purposes)
    pub title: Option<String>,
    /// Parent namespace (for hierarchical pages)
    pub namespace_id: Option<Uuid>,
    /// Journal day if this is a journal page (YYYYMMDD)
    pub journal_day: Option<JournalDay>,
    /// Content format (markdown or org)
    pub format: BlockFormat,
    /// Associated file ID (for file-based pages)
    pub file_id: Option<Uuid>,
    /// Original name before rename
    pub original_name: Option<String>,
    /// Whether this is a journal page
    pub journal: bool,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Data required to create a new page
#[derive(Debug, Clone)]
pub struct PageCreate {
    pub name: String,
    pub title: Option<String>,
    pub namespace_id: Option<Uuid>,
    pub journal_day: Option<JournalDay>,
    pub format: BlockFormat,
    pub file_id: Option<Uuid>,
}

impl Page {
    /// Create a new regular page
    pub fn new(create: PageCreate) -> Result<Self, DomainError> {
        let now = chrono::Utc::now();
        let name = Self::normalize_name(&create.name)?;

        // Validate it's not a journal if journal_day is not set
        if create.journal_day.is_some() && create.name.is_empty() {
            return Err(DomainError::InvalidPageName(
                "Journal pages must have a name".to_string(),
            ));
        }

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            title: create.title,
            namespace_id: create.namespace_id,
            journal_day: create.journal_day,
            format: create.format,
            file_id: create.file_id,
            original_name: None,
            journal: create.journal_day.is_some(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Create a new journal page for a specific day
    pub fn new_journal(day: JournalDay, format: BlockFormat) -> Result<Self, DomainError> {
        let now = chrono::Utc::now();
        let name = day.to_string();

        Ok(Self {
            id: Uuid::new_v4(),
            name: name.clone(),
            title: Some(day.to_string()),
            namespace_id: None,
            journal_day: Some(day),
            format,
            file_id: None,
            original_name: None,
            journal: true,
            created_at: now,
            updated_at: now,
        })
    }

    /// Normalize a page name according to Logseq rules:
    /// - Lowercase
    /// - No special characters: / # ? : | < > * " \
    pub fn normalize_name(name: &str) -> Result<String, DomainError> {
        let normalized = name.trim().to_lowercase();

        // Check for invalid characters
        let invalid_chars = ['/', '#', '?', ':', '|', '<', '>', '*', '"', '\\'];
        for c in invalid_chars.iter() {
            if normalized.contains(*c) {
                return Err(DomainError::InvalidPageName(format!(
                    "Page name cannot contain '{}'",
                    c
                )));
            }
        }

        // Cannot be empty or just numbers
        if normalized.is_empty() {
            return Err(DomainError::InvalidPageName(
                "Page name cannot be empty".to_string(),
            ));
        }

        if normalized.chars().all(|c| c.is_ascii_digit()) {
            return Err(DomainError::InvalidPageName(
                "Page name cannot be only numbers".to_string(),
            ));
        }

        Ok(normalized)
    }

    /// Rename this page
    pub fn rename(&mut self, new_name: &str) -> Result<(), DomainError> {
        let normalized = Self::normalize_name(new_name)?;
        self.original_name = Some(self.name.clone());
        self.name = normalized;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Check if this is a journal page
    pub fn is_journal(&self) -> bool {
        self.journal
    }

    /// Check if this is a namespace page
    pub fn is_namespace(&self) -> bool {
        self.namespace_id.is_none() && !self.journal && self.file_id.is_none()
    }

    /// Get the full path including namespace
    pub fn full_name(&self) -> String {
        if self.namespace_id.is_some() {
            // In a full implementation, we'd look up the namespace path
            format!("../{}", self.name)
        } else {
            self.name.clone()
        }
    }

    /// Check if a page name is valid
    pub fn is_valid_name(name: &str) -> bool {
        Self::normalize_name(name).is_ok()
    }
}

impl Default for PageCreate {
    fn default() -> Self {
        Self {
            name: String::new(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_creation() {
        let create = PageCreate {
            name: "My Test Page".to_string(),
            title: Some("My Test Page".to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        };

        let page = Page::new(create).unwrap();
        assert_eq!(page.name, "my test page");
        assert!(!page.journal);
    }

    #[test]
    fn test_page_name_normalization() {
        let create = PageCreate {
            name: "UPPERCASE".to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
        };

        let page = Page::new(create).unwrap();
        assert_eq!(page.name, "uppercase");
    }

    #[test]
    fn test_invalid_page_names() {
        let invalid_names = vec!["", "/slash", "#hash", "12345"];

        for name in invalid_names {
            let create = PageCreate {
                name: name.to_string(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            };

            assert!(Page::new(create).is_err());
        }
    }

    #[test]
    fn test_journal_page() {
        let day = JournalDay::from_ymd(2026, 5, 2).unwrap();
        let page = Page::new_journal(day, BlockFormat::Markdown).unwrap();

        assert!(page.journal);
        assert!(page.journal_day.is_some());
        assert_eq!(page.name, "2026-05-02");
    }
}
