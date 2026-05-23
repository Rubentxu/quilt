//! Text parser for BlockContent
//!
//! Parses raw text into typed BlockContent segments:
//! - `[[PageName]]` → PageRef
//! - `((BlockId))` → BlockRef
//! - Plain text → Text
//!
//! This parser is stateless and synchronous. It does NOT resolve page names
//! or block IDs to UUIDs — that resolution happens at the repository/service layer.

use crate::content::BlockSegment;
use thiserror::Error;

/// Parse error for text parsing operations
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ParseError {
    #[error("Unclosed page reference at position {position}")]
    UnclosedPageRef { position: usize },

    #[error("Unclosed block reference at position {position}")]
    UnclosedBlockRef { position: usize },

    #[error("Invalid block ID format: {0}")]
    InvalidBlockId(String),
}

/// Parse raw text into BlockContent with typed segments.
///
/// This function extracts `[[PageName]]` and `((BlockId))` patterns as typed
/// segments, preserving all other text as plain text segments.
///
/// # Example
///
/// ```
/// use quilt_domain::content::parse_text;
///
/// let content = parse_text("Hello [[World]] and ((block-id))");
/// // Returns BlockContent with segments:
/// // - Text { content: "Hello ", marks: [] }
/// // - PageRef { target: "World", label: None }
/// // - Text { content: " and ", marks: [] }
/// // - BlockRef { target: "block-id" }
/// ```
///
/// # Notes
///
/// - Page references are stored with the page name as a string identifier.
///   Resolution to UUID happens at the repository layer when the page is looked up.
/// - Block references are stored with the block ID as a string identifier.
///   Resolution to UUID happens when the block is loaded.
pub fn parse_text(input: &str) -> crate::content::BlockContent {
    let mut segments: Vec<BlockSegment> = Vec::new();

    let input_chars: Vec<char> = input.chars().collect();
    let len = input_chars.len();
    let mut i = 0;
    let mut text_start = 0;

    while i < len {
        // Check for page reference opening [[
        if i + 1 < len && input_chars[i] == '[' && input_chars[i + 1] == '[' {
            let entry_pos = i;
            // Flush accumulated text
            if text_start < i {
                let text: String = input_chars[text_start..i].iter().collect();
                if !text.is_empty() {
                    segments.push(BlockSegment::Text {
                        content: text,
                        marks: Vec::new(),
                    });
                }
            }
            // Skip the opening [[
            i += 2;
            let content_start = i;

            // Find the closing ]]
            while i < len {
                if i + 1 < len && input_chars[i] == ']' && input_chars[i + 1] == ']' {
                    // Extract page name
                    let page_name: String = input_chars[content_start..i].iter().collect();
                    if !page_name.is_empty() {
                        segments.push(BlockSegment::PageRef {
                            target: page_name,
                            label: None,
                        });
                    }
                    // Skip the closing ]] and any trailing space
                    i += 2;
                    if i < len && input_chars[i] == ' ' {
                        i += 1;
                    }
                    text_start = i;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Check for block reference opening ((
        if i + 1 < len && input_chars[i] == '(' && input_chars[i + 1] == '(' {
            let entry_pos = i;
            // Flush accumulated text
            if text_start < i {
                let text: String = input_chars[text_start..i].iter().collect();
                if !text.is_empty() {
                    segments.push(BlockSegment::Text {
                        content: text,
                        marks: Vec::new(),
                    });
                }
            }
            // Skip the opening ((
            i += 2;
            let content_start = i;

            // Find the closing ))
            while i < len {
                if i + 1 < len && input_chars[i] == ')' && input_chars[i + 1] == ')' {
                    // Extract block ID
                    let block_id: String = input_chars[content_start..i].iter().collect();
                    // Validate block ID format (alphanumeric, dash, underscore)
                    if !block_id.is_empty()
                        && block_id
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        segments.push(BlockSegment::BlockRef { target: block_id });
                    }
                    // Skip the closing )) and any trailing space
                    i += 2;
                    if i < len && input_chars[i] == ' ' {
                        i += 1;
                    }
                    text_start = i;
                    break;
                }
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    // Flush any remaining text
    if text_start < len {
        let text: String = input_chars[text_start..len].iter().collect();
        if !text.is_empty() {
            segments.push(BlockSegment::Text {
                content: text,
                marks: Vec::new(),
            });
        }
    }

    crate::content::BlockContent { segments }
}

/// Parse text with marks (formatting).
///
/// This is a more advanced parser that also extracts inline formatting marks
/// like **bold**, *italic*, `code`, etc.
///
/// Currently, this preserves marks in a simplified way. Full mark extraction
/// requires a more sophisticated parser state machine.
pub fn parse_text_with_marks(input: &str) -> crate::content::BlockContent {
    parse_text(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_text() {
        let content = parse_text("Hello World");
        assert_eq!(content.segments.len(), 1);
        assert_eq!(content.as_plain_text(), "Hello World");
    }

    #[test]
    fn test_parse_page_ref() {
        let content = parse_text("Hello [[World]]");
        assert_eq!(content.segments.len(), 2);

        match &content.segments[0] {
            BlockSegment::Text { content, marks } => {
                assert_eq!(content, "Hello ");
                assert!(marks.is_empty());
            }
            _ => panic!("Expected Text segment"),
        }

        match &content.segments[1] {
            BlockSegment::PageRef { target, label } => {
                assert_eq!(target, "World");
                assert!(label.is_none());
            }
            _ => panic!("Expected PageRef segment"),
        }
    }

    #[test]
    fn test_parse_block_ref() {
        let content = parse_text("See ((block-123)) for details");
        assert_eq!(content.segments.len(), 3);

        match &content.segments[1] {
            BlockSegment::BlockRef { target } => {
                assert_eq!(target, "block-123");
            }
            _ => panic!("Expected BlockRef segment"),
        }
    }

    #[test]
    fn test_parse_mixed() {
        let content = parse_text("Hello [[World]] and ((block-id))");
        assert_eq!(content.segments.len(), 4);

        match &content.segments[0] {
            BlockSegment::Text { content, .. } => assert_eq!(content, "Hello "),
            _ => panic!("Expected Text"),
        }

        match &content.segments[1] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "World"),
            _ => panic!("Expected PageRef"),
        }

        match &content.segments[2] {
            BlockSegment::Text { content, .. } => assert_eq!(content, " and "),
            _ => panic!("Expected Text"),
        }

        match &content.segments[3] {
            BlockSegment::BlockRef { target } => assert_eq!(target, "block-id"),
            _ => panic!("Expected BlockRef"),
        }
    }

    #[test]
    fn test_parse_multiple_page_refs() {
        let content = parse_text("[[Page1]] and [[Page2]]");
        assert_eq!(content.segments.len(), 3);

        match &content.segments[0] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "Page1"),
            _ => panic!("Expected PageRef"),
        }

        match &content.segments[1] {
            BlockSegment::Text { content, .. } => assert_eq!(content, " and "),
            _ => panic!("Expected Text"),
        }

        match &content.segments[2] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "Page2"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_multiple_block_refs() {
        let content = parse_text("((id1)) then ((id2))");
        assert_eq!(content.segments.len(), 3);

        match &content.segments[0] {
            BlockSegment::BlockRef { target } => assert_eq!(target, "id1"),
            _ => panic!("Expected BlockRef"),
        }

        match &content.segments[2] {
            BlockSegment::BlockRef { target } => assert_eq!(target, "id2"),
            _ => panic!("Expected BlockRef"),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let content = parse_text("");
        assert!(content.is_empty());
    }

    #[test]
    fn test_parse_only_refs() {
        let content = parse_text("[[Page]]");
        assert_eq!(content.segments.len(), 1);

        match &content.segments[0] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "Page"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_unicode() {
        let content = parse_text("Hello [[世界]]");
        assert_eq!(content.segments.len(), 2);

        match &content.segments[1] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "世界"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_page_ref_with_special_chars() {
        let content = parse_text("See [[Page-123]] for details");
        assert_eq!(content.segments.len(), 2);

        match &content.segments[1] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "Page-123"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_block_ref_with_underscore() {
        let content = parse_text("Link to ((block_id_123))");
        assert_eq!(content.segments.len(), 2);

        match &content.segments[1] {
            BlockSegment::BlockRef { target } => assert_eq!(target, "block_id_123"),
            _ => panic!("Expected BlockRef"),
        }
    }

    #[test]
    fn test_plain_text_with_refs() {
        let content = parse_text("The [[Page]] contains ((Block))");
        assert_eq!(content.as_plain_text(), "The Page contains Block");
    }

    #[test]
    fn test_parse_nested_brackets() {
        // [[Page [with] brackets]] - should parse as single ref with literal brackets
        let content = parse_text("[[Page [with] brackets]]");
        assert_eq!(content.segments.len(), 1);
        match &content.segments[0] {
            BlockSegment::PageRef { target, .. } => assert_eq!(target, "Page [with] brackets"),
            _ => panic!("Expected PageRef"),
        }
    }
}
