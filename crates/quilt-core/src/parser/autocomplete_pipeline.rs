//! Autocomplete pipeline — trigger detection → suggestion → insertion.
//!
//! Provides pure functions that compose the autocomplete data flow:
//! 1. `detect_trigger` from parser/autocomplete
//! 2. `AutocompleteService.suggest`
//! 3. `compute_insertion` — given a trigger and selected item, compute
//!    the new text and cursor position after insertion.
//!
//! These functions are synchronous and stateless, making them testable
//! without a WASM environment.

use crate::parser::autocomplete::{
    detect_trigger, AutocompleteItem, AutocompleteResult, AutocompleteTrigger,
};
use crate::parser::providers::create_default_service;

/// Result of computing an autocomplete insertion.
#[derive(Debug, Clone, PartialEq)]
pub struct InsertionResult {
    /// The full text to replace the current block content with.
    pub new_content: String,
    /// Where to place the cursor after insertion.
    pub cursor_offset: usize,
}

/// Compute the insertion result for a trigger and selected item.
///
/// Given the original content, the detected trigger, and the selected
/// autocomplete item, this function produces the updated content and
/// cursor position.
///
/// For page refs: `[[` + page_name + `]]` with cursor after the closing brackets.
/// For tags: `#` + tag_name (no closing needed) with cursor after the tag word.
/// For property values: `key:: ` + value with cursor after the value.
///
/// # Returns
/// `None` if the trigger position cannot be determined from the content
/// (e.g., the trigger has been removed since detection).
pub fn compute_insertion(
    content: &str,
    trigger: &AutocompleteTrigger,
    item: &AutocompleteItem,
) -> Option<InsertionResult> {
    match trigger {
        AutocompleteTrigger::PageRef { prefix: _ } => {
            // Find the [[ that started this trigger
            let before_cursor = content.len();
            let insertion_start = find_pattern_start(content, "[[", before_cursor)?;
            // Check no ]] between [[ and end
            let after_brackets = &content[insertion_start + 2..];
            if after_brackets.contains("]]") {
                return None; // Already closed
            }

            let page_name = &item.insert_text;
            let replacement = format!("[[{}]]", page_name);
            let new_content = format!("{}{}", &content[..insertion_start], &replacement);
            let cursor_offset = insertion_start + replacement.len();
            Some(InsertionResult {
                new_content,
                cursor_offset,
            })
        }
        AutocompleteTrigger::Tag { prefix: _ } => {
            let before_cursor = content.len();
            let hash_pos = find_pattern_start(content, "#", before_cursor)?;

            // Check word boundary before #
            if hash_pos > 0 && !content.as_bytes()[hash_pos - 1].is_ascii_whitespace() {
                return None;
            }

            let tag_name = &item.insert_text;
            let replacement = format!("#{}", tag_name);
            let new_content = format!("{}{}", &content[..hash_pos], &replacement);
            let cursor_offset = hash_pos + replacement.len();
            Some(InsertionResult {
                new_content,
                cursor_offset,
            })
        }
        AutocompleteTrigger::PropertyValue { key, prefix: _ } => {
            // Find the key:: that started this trigger
            let before_cursor = content.len();
            let pattern = format!("{}::", key);
            let colon_pos = find_pattern_start(content, &pattern, before_cursor)?;

            // Everything after key:: and before cursor is the prefix we typed
            let value = &item.insert_text;
            let replacement = format!("{}:: {}", key, value);
            let new_content = format!("{}{}", &content[..colon_pos], &replacement);
            let cursor_offset = colon_pos + replacement.len();
            Some(InsertionResult {
                new_content,
                cursor_offset,
            })
        }
        AutocompleteTrigger::BlockRef { .. } => {
            // BlockRef insertion is not yet implemented
            None
        }
    }
}

/// Find the last occurrence of a pattern before a position.
/// Like rfind, but returns position relative to original string.
fn find_pattern_start(content: &str, pattern: &str, before: usize) -> Option<usize> {
    let search_boundary = if before > content.len() {
        content.len()
    } else {
        before
    };
    content[..search_boundary].rfind(pattern)
}

/// Run the full autocomplete pipeline: detect → suggest.
///
/// This is the primary integration point for the editor.
/// Given current content and cursor position, it:
/// 1. Detects the trigger
/// 2. Runs all registered providers
/// 3. Returns the result
///
/// If no trigger is found, returns an empty result.
pub fn autocomplete_at_cursor(
    content: &str,
    cursor_pos: usize,
) -> (Option<AutocompleteTrigger>, AutocompleteResult) {
    let trigger = detect_trigger(content, cursor_pos);
    match trigger {
        Some(t) => {
            let service = create_default_service(vec![]); // No pages loaded by default
            let result = service.suggest(&t);
            (Some(t), result)
        }
        None => (None, AutocompleteResult::default()),
    }
}

/// Run pipeline with a pre-built service for richer suggestions.
pub fn autocomplete_at_cursor_with_service(
    content: &str,
    cursor_pos: usize,
    service: &crate::parser::autocomplete::AutocompleteService,
) -> (Option<AutocompleteTrigger>, AutocompleteResult) {
    let trigger = detect_trigger(content, cursor_pos);
    match trigger {
        Some(t) => {
            let result = service.suggest(&t);
            (Some(t), result)
        }
        None => (None, AutocompleteResult::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::autocomplete::AutocompleteService;

    // ── compute_insertion tests ──

    #[test]
    fn test_insert_page_ref_simple() {
        let content = "see [[proj";
        let trigger = AutocompleteTrigger::PageRef {
            prefix: "proj".into(),
        };
        let item = AutocompleteItem {
            label: "Project Alpha".into(),
            insert_text: "Project Alpha".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Page,
        };

        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");

        assert_eq!(result.new_content, "see [[Project Alpha]]");
        assert_eq!(result.cursor_offset, 21); // after [[Project Alpha]]
    }

    #[test]
    fn test_insert_page_ref_already_closed_returns_none() {
        let content = "[[Done]] more text";
        let trigger = AutocompleteTrigger::PageRef { prefix: "".into() };
        let item = AutocompleteItem {
            label: "Done".into(),
            insert_text: "Done".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Page,
        };

        let result = compute_insertion(content, &trigger, &item);
        assert!(result.is_none(), "Already closed ref should not insert");
    }

    #[test]
    fn test_insert_tag_simple() {
        let content = "meeting #ur";
        let trigger = AutocompleteTrigger::Tag {
            prefix: "ur".into(),
        };
        let item = AutocompleteItem {
            label: "#urgent".into(),
            insert_text: "urgent".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Tag,
        };

        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");

        assert_eq!(result.new_content, "meeting #urgent");
        assert_eq!(result.cursor_offset, 15); // after #urgent
    }

    #[test]
    fn test_insert_tag_at_start() {
        let content = "#pro";
        let trigger = AutocompleteTrigger::Tag {
            prefix: "pro".into(),
        };
        let item = AutocompleteItem {
            label: "#project".into(),
            insert_text: "project".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Tag,
        };

        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");

        assert_eq!(result.new_content, "#project");
    }

    #[test]
    fn test_insert_property_value() {
        let content = "status:: ";
        let trigger = AutocompleteTrigger::PropertyValue {
            key: "status".into(),
            prefix: "".into(),
        };
        let item = AutocompleteItem {
            label: "status:: TODO".into(),
            insert_text: "TODO".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::PropertyValue,
        };

        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");

        assert_eq!(result.new_content, "status:: TODO");
        assert_eq!(result.cursor_offset, 13); // after status:: TODO
    }

    #[test]
    fn test_insert_block_ref_not_implemented() {
        let content = "see ((550e";
        let trigger = AutocompleteTrigger::BlockRef {
            prefix: "550e".into(),
        };
        let item = AutocompleteItem {
            label: "block ref".into(),
            insert_text: "550e...".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Block,
        };

        let result = compute_insertion(content, &trigger, &item);
        assert!(result.is_none(), "BlockRef insertion not yet implemented");
    }

    // ── autocomplete_at_cursor tests ──

    #[test]
    fn test_autocomplete_at_cursor_no_trigger() {
        let (trigger, result) = autocomplete_at_cursor("plain text", 10);
        assert!(trigger.is_none());
        assert!(result.is_empty());
    }

    #[test]
    fn test_autocomplete_at_cursor_empty() {
        let (trigger, result) = autocomplete_at_cursor("", 0);
        assert!(trigger.is_none());
        assert!(result.is_empty());
    }

    #[test]
    fn test_autocomplete_at_cursor_tag_trigger() {
        let (trigger, result) = autocomplete_at_cursor("text #ur", 8);
        assert!(trigger.is_some());
        if let Some(AutocompleteTrigger::Tag { prefix }) = &trigger {
            assert_eq!(prefix, "ur");
        } else {
            panic!("Expected Tag trigger");
        }
        // No page names loaded, but tag provider uses defaults
        assert!(!result.is_empty(), "Tag provider should return defaults");
    }

    #[test]
    fn test_autocomplete_at_cursor_page_ref_empty_service() {
        let (trigger, result) = autocomplete_at_cursor("[[Proj", 6);
        assert!(trigger.is_some());
        // Empty page names list → no results
        assert!(result.is_empty(), "No pages loaded → no suggestions");
    }

    #[test]
    fn test_autocomplete_at_cursor_with_service() {
        let mut service = AutocompleteService::new();
        service.register(Box::new(crate::parser::providers::PageRefProvider::new(
            vec!["Project Alpha".into(), "Project Beta".into()],
        )));

        let (trigger, result) = autocomplete_at_cursor_with_service("[[Proj", 6, &service);
        assert!(trigger.is_some());
        assert_eq!(result.items.len(), 2);
    }

    // ── Triangulation: edge cases ──

    #[test]
    fn test_insert_page_ref_with_existing_content_after() {
        let content = "[[proj is cool";
        let trigger = AutocompleteTrigger::PageRef {
            prefix: "proj".into(),
        };
        let item = AutocompleteItem {
            label: "Project".into(),
            insert_text: "Project".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Page,
        };

        // The insertion replaces from [[ onward
        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");
        assert_eq!(result.new_content, "[[Project]]");
        // Note: content after the trigger is lost — acceptable for v1
    }

    #[test]
    fn test_autocomplete_at_cursor_property_trigger() {
        let (trigger, result) = autocomplete_at_cursor("status:: ", 9);
        assert!(trigger.is_some());
        if let Some(AutocompleteTrigger::PropertyValue { key, .. }) = &trigger {
            assert_eq!(key, "status");
        } else {
            panic!("Expected PropertyValue trigger");
        }
        assert!(
            !result.is_empty(),
            "Property provider should return status values"
        );
    }

    #[test]
    fn test_compute_insertion_with_trigger_has_no_prefix_trailing() {
        // Tag at start of content
        let content = "#urg";
        let trigger = AutocompleteTrigger::Tag {
            prefix: "urg".into(),
        };
        let item = AutocompleteItem {
            label: "#urgent".into(),
            insert_text: "urgent".into(),
            description: None,
            category: crate::parser::autocomplete::AutocompleteCategory::Tag,
        };

        let result = compute_insertion(content, &trigger, &item).expect("Should compute insertion");
        assert_eq!(result.new_content, "#urgent");
        assert_eq!(result.cursor_offset, 7);
    }
}
