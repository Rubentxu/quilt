//! Taxonomy value object - represents tagging and references for a block
//!
//! This value object encapsulates the taxonomy-related fields:
/// tags and references (refs) to other blocks/pages.

use crate::value_objects::Uuid;

/// Taxonomy represents the tagging and reference state of a block.
///
/// It encapsulates:
/// - `tags`: Associated tags (e.g., #tag, #project)
/// - `refs`: References to other blocks or pages (e.g., [[page]] links)
///
/// # Invariants
///
/// - Tags and refs are stored as vectors for efficient queries
/// - Tags are kept in insertion order
/// - Refs are kept in insertion order and deduplicated
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taxonomy {
    /// Tags associated with this block
    pub tags: Vec<String>,
    /// References to other blocks/pages
    pub refs: Vec<Uuid>,
}

impl Taxonomy {
    /// Create an empty taxonomy (no tags or refs).
    pub fn none() -> Self {
        Self {
            tags: Vec::new(),
            refs: Vec::new(),
        }
    }

    /// Create a taxonomy with tags only.
    pub fn with_tags(tags: Vec<String>) -> Self {
        Self { tags, refs: Vec::new() }
    }

    /// Create a taxonomy with refs only.
    pub fn with_refs(refs: Vec<Uuid>) -> Self {
        Self {
            tags: Vec::new(),
            refs,
        }
    }

    /// Check if taxonomy is empty (no tags and no refs).
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.refs.is_empty()
    }

    /// Check if this block has any tags.
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    /// Check if this block has any refs.
    pub fn has_refs(&self) -> bool {
        !self.refs.is_empty()
    }

    /// Add a tag (deduplicates).
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag.
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }

    /// Add a reference (deduplicates).
    pub fn add_ref(&mut self, id: Uuid) {
        if !self.refs.contains(&id) {
            self.refs.push(id);
        }
    }

    /// Remove a reference.
    pub fn remove_ref(&mut self, id: Uuid) {
        self.refs.retain(|r| *r != id);
    }

    /// Check if this taxonomy contains a specific tag.
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Check if this taxonomy contains a specific ref.
    pub fn contains_ref(&self, id: Uuid) -> bool {
        self.refs.contains(&id)
    }

    /// Get the number of tags.
    pub fn tag_count(&self) -> usize {
        self.tags.len()
    }

    /// Get the number of refs.
    pub fn ref_count(&self) -> usize {
        self.refs.len()
    }
}

impl Default for Taxonomy {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taxonomy_none() {
        let taxonomy = Taxonomy::none();
        assert!(taxonomy.is_empty());
        assert!(!taxonomy.has_tags());
        assert!(!taxonomy.has_refs());
    }

    #[test]
    fn test_add_tag_deduplicates() {
        let mut taxonomy = Taxonomy::none();
        taxonomy.add_tag("important");
        taxonomy.add_tag("important");
        taxonomy.add_tag("urgent");
        
        assert_eq!(taxonomy.tag_count(), 2);
    }

    #[test]
    fn test_add_ref_deduplicates() {
        let mut taxonomy = Taxonomy::none();
        let id1 = Uuid::new_v4();
        taxonomy.add_ref(id1);
        taxonomy.add_ref(id1);
        
        assert_eq!(taxonomy.ref_count(), 1);
    }

    #[test]
    fn test_remove_tag() {
        let mut taxonomy = Taxonomy::with_tags(vec!["important".to_string()]);
        taxonomy.remove_tag("important");
        assert!(!taxonomy.has_tags());
    }

    #[test]
    fn test_remove_ref() {
        let id = Uuid::new_v4();
        let mut taxonomy = Taxonomy::with_refs(vec![id]);
        taxonomy.remove_ref(id);
        assert!(!taxonomy.has_refs());
    }

    #[test]
    fn test_contains_tag() {
        let taxonomy = Taxonomy::with_tags(vec!["important".to_string()]);
        assert!(taxonomy.contains_tag("important"));
        assert!(!taxonomy.contains_tag("urgent"));
    }
}
