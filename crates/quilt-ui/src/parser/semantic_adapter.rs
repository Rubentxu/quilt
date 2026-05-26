//! Semantic adapter for safe UI consumption of parsed content.
//!
//! Provides a lightweight bridge between the parser and UI components
//! without coupling them directly. This module handles:
//! - Deriving `SemanticData` from block content
//! - Producing safe display values (no panics, no I/O)
//! - Extension points for future autocomplete/decorations
//!
//! This is the **only** module that the UI layer should import from the parser.
//! Components should NOT import `InlineParser` directly.

use crate::parser::inline::InlineParser;
use crate::parser::SemanticData;

/// Compute semantic data for a block content string.
///
/// This is the primary entry point for UI components that need
/// to know about tags, refs, and properties in a block without
/// dealing with parser internals.
///
/// # Returns
/// Always returns a `SemanticData` value, never panics.
/// If content is empty or unparseable, returns default (empty) data.
pub fn compute_semantic_data(content: &str) -> SemanticData {
    if content.is_empty() {
        return SemanticData::default();
    }
    let parser = InlineParser::default();
    let parsed = parser.parse(content);
    parsed.semantic_data()
}

/// Check if a block has any tags (from `#tag` or `tags::`).
pub fn has_tags(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    let parser = InlineParser::default();
    let parsed = parser.parse(content);
    let data = parsed.semantic_data();
    !data.tags.is_empty()
}

/// Get tags for display, limited to max_tags items.
/// Tags are returned in normalized lowercase form.
pub fn display_tags(content: &str, max_tags: usize) -> Vec<String> {
    let data = compute_semantic_data(content);
    data.tags.into_iter().take(max_tags).collect()
}

/// Format a block's semantic data for debug display.
/// Useful for development overlays and inspector panels.
#[cfg(debug_assertions)]
pub fn debug_semantic_summary(content: &str) -> String {
    let data = compute_semantic_data(content);
    let parts: Vec<String> = Vec::new();
    let mut parts = parts;
    if !data.tags.is_empty() {
        parts.push(format!("tags: [{}]", data.tags.join(", ")));
    }
    if !data.page_refs.is_empty() {
        parts.push(format!("pages: {}", data.page_refs.len()));
    }
    if !data.block_refs.is_empty() {
        parts.push(format!("blocks: {}", data.block_refs.len()));
    }
    if data.properties_count > 0 {
        parts.push(format!("props: {}", data.properties_count));
    }
    if parts.is_empty() {
        String::new()
    } else {
        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_empty() {
        let data = compute_semantic_data("");
        assert!(data.tags.is_empty());
        assert!(data.page_refs.is_empty());
        assert_eq!(data.properties_count, 0);
    }

    #[test]
    fn test_compute_tags_only() {
        let data = compute_semantic_data("#bug #critical");
        assert_eq!(data.tags.len(), 2);
        assert!(data.tags.contains(&"bug".to_string()));
        assert!(data.tags.contains(&"critical".to_string()));
    }

    #[test]
    fn test_has_tags_detection() {
        assert!(has_tags("#urgent meeting"));
        assert!(has_tags("tags:: work, personal"));
        assert!(!has_tags("plain text with no tags"));
        assert!(!has_tags(""));
    }

    #[test]
    fn test_display_tags_limited() {
        let tags = display_tags("#a #b #c #d", 2);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_compute_mixed_content() {
        let data = compute_semantic_data("#work Meeting [[Project Alpha]] status:: active");
        assert!(data.tags.contains(&"work".to_string()));
        assert!(data.page_refs.contains(&"Project Alpha".to_string()));
        assert_eq!(data.properties.get("status"), Some(&"active".to_string()));
    }
}
