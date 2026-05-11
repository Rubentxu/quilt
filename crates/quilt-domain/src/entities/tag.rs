//! Tag entity - a label for categorizing pages and blocks

use crate::value_objects::Uuid;

/// Tag represents a tag/label in the knowledge graph.
///
/// Tags are pages with the # prefix in Logseq syntax.
/// They can be hierarchical (e.g., #rust/async).
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    /// The page ID this tag is associated with
    page_id: Uuid,
    /// The tag name (without the # prefix)
    name: String,
    /// Creation timestamp
    created_at: chrono::DateTime<chrono::Utc>,
}

impl Tag {
    /// Create a new tag
    pub fn new(page_id: Uuid, name: impl Into<String>) -> Self {
        Self {
            page_id,
            name: name.into(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Get the page ID
    pub fn page_id(&self) -> Uuid {
        self.page_id
    }

    /// Get the tag name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the full tag with # prefix (as used in Logseq)
    pub fn full_name(&self) -> String {
        format!("#{}", self.name)
    }

    /// Check if this tag is a child of another tag (hierarchical)
    pub fn is_child_of(&self, parent_name: &str) -> bool {
        self.name.starts_with(&format!("{}/", parent_name))
    }

    /// Get the parent tag name (if this is a child tag)
    pub fn parent_name(&self) -> Option<&str> {
        self.name.split('/').next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_creation() {
        let page_id = Uuid::new_v4();
        let tag = Tag::new(page_id, "rust");

        assert_eq!(tag.name, "rust");
        assert_eq!(tag.full_name(), "#rust");
    }

    #[test]
    fn test_hierarchical_tags() {
        let page_id = Uuid::new_v4();
        let tag = Tag::new(page_id, "rust/async");

        assert!(tag.is_child_of("rust"));
        assert!(!tag.is_child_of("python"));
        assert_eq!(tag.parent_name(), Some("rust"));
    }
}
