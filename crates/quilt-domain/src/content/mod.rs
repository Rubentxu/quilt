//! Content module - represents block content as typed segments
//!
//! This module implements the BlockSegment content model from ADR 0005.
//! Content is represented as a sequence of typed segments, not a plain string.
//! This enables mixed content (text + images + code) within a single block.

mod block_segment;
mod mark;

pub use block_segment::BlockSegment;
pub use mark::Mark;

use serde::{Deserialize, Serialize};

/// A block's content as a sequence of typed segments.
///
/// BlockContent is the content model for blocks. It replaces the previous
/// plain String content with a structured sequence of segments that can be:
/// - Plain text with optional formatting marks
/// - Page references [[Page]]
/// - Block references ((Block))
/// - Images
/// - Code blocks
/// - Tables
/// - Dates
/// - Tags
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct BlockContent {
    /// The sequence of segments that make up this content
    pub segments: Vec<BlockSegment>,
}

impl BlockContent {
    /// Create a new BlockContent with a single text segment.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            segments: vec![BlockSegment::Text {
                content: text.into(),
                marks: Vec::new(),
            }],
        }
    }

    /// Create an empty BlockContent.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if content is empty.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
            || self.segments.iter().all(|s| {
                matches!(
                    s,
                    BlockSegment::Text { content, marks }
                        if content.is_empty() && marks.is_empty()
                )
            })
    }

    /// Get the number of segments.
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    /// Get plain text content by concatenating all text segments.
    /// Non-text segments (refs, images, etc.) are excluded.
    pub fn as_plain_text(&self) -> String {
        let mut result = String::new();
        for segment in &self.segments {
            if let BlockSegment::Text { content, .. } = segment {
                if !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(content);
            }
        }
        result
    }

    /// Get plain text as lowercase.
    ///
    /// Convenience method equivalent to `self.as_plain_text().to_lowercase()`.
    pub fn to_lowercase(&self) -> String {
        self.as_plain_text().to_lowercase()
    }

    /// Check if plain text content contains a substring (case-sensitive).
    pub fn contains(&self, needle: &str) -> bool {
        self.as_plain_text().contains(needle)
    }

    /// Extract all page references from this content.
    pub fn page_refs(&self) -> Vec<crate::value_objects::Uuid> {
        self.segments
            .iter()
            .filter_map(|s| {
                if let BlockSegment::PageRef { target, .. } = s {
                    Some(*target)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Extract all block references from this content.
    pub fn block_refs(&self) -> Vec<crate::value_objects::Uuid> {
        self.segments
            .iter()
            .filter_map(|s| {
                if let BlockSegment::BlockRef { target } = s {
                    Some(*target)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Split this content at the given character position.
    ///
    /// Returns two BlockContents:
    /// - First: content from start up to (but not including) the char at `position`
    /// - Second: content from `position` to end
    ///
    /// Non-text segments (refs, images, etc.) are assigned to the block that
    /// contains the split point, or to the first block if the split falls
    /// between segments.
    ///
    /// # Arguments
    /// * `position` - Character index (in plain text) where to split
    ///
    /// # Returns
    /// `(left, right)` BlockContents. If position is 0, left is empty.
    /// If position >= total_chars, right is empty.
    pub fn split_at(&self, position: usize) -> (Self, Self) {
        let total_chars: usize = self.segments.iter().map(|s| s.text_len()).sum();

        // Handle edge cases
        if position == 0 {
            return (BlockContent::empty(), self.clone());
        }
        if position >= total_chars {
            return (self.clone(), BlockContent::empty());
        }

        let mut left_segments = Vec::new();
        let mut right_segments = Vec::new();
        let mut current_pos = 0;
        let mut split_done = false;

        for segment in &self.segments {
            let seg_len = segment.text_len();

            if split_done {
                // We're past the split point
                right_segments.push(segment.clone());
            } else if current_pos + seg_len <= position {
                // This segment is entirely before the split
                left_segments.push(segment.clone());
                current_pos += seg_len;
            } else {
                // This segment contains the split point
                match segment {
                    BlockSegment::Text { content, marks } => {
                        let local_pos = position - current_pos;
                        let (left_text, right_text) = content.split_at(local_pos);

                        // Left text segment (if non-empty)
                        if !left_text.is_empty() {
                            left_segments.push(BlockSegment::Text {
                                content: left_text.to_string(),
                                marks: marks.clone(),
                            });
                        }

                        // Right text segment (if non-empty)
                        if !right_text.is_empty() {
                            right_segments.push(BlockSegment::Text {
                                content: right_text.to_string(),
                                marks: marks.clone(),
                            });
                        }
                    }
                    _ => {
                        // Non-text segment: put in left block if we're at the split,
                        // otherwise continue to left
                        left_segments.push(segment.clone());
                    }
                }
                split_done = true;
                current_pos += seg_len;
            }
        }

        (
            BlockContent {
                segments: left_segments,
            },
            BlockContent {
                segments: right_segments,
            },
        )
    }
}

impl From<String> for BlockContent {
    fn from(s: String) -> Self {
        Self::from_text(s)
    }
}

impl From<&str> for BlockContent {
    fn from(s: &str) -> Self {
        Self::from_text(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_content_from_text() {
        let content = BlockContent::from_text("Hello, world!");
        assert_eq!(content.segments.len(), 1);
        match &content.segments[0] {
            BlockSegment::Text { content: c, marks } => {
                assert_eq!(c, "Hello, world!");
                assert!(marks.is_empty());
            }
            _ => panic!("Expected Text segment"),
        }
    }

    #[test]
    fn test_block_content_empty() {
        let content = BlockContent::empty();
        assert!(content.is_empty());
        assert_eq!(content.len(), 0);
    }

    #[test]
    fn test_block_content_plain_text() {
        let content = BlockContent::from_text("Hello");
        assert_eq!(content.as_plain_text(), "Hello");

        let content = BlockContent {
            segments: vec![
                BlockSegment::Text {
                    content: "Hello".to_string(),
                    marks: Vec::new(),
                },
                BlockSegment::PageRef {
                    target: crate::value_objects::Uuid::new_v4(),
                    label: None,
                },
                BlockSegment::Text {
                    content: "World".to_string(),
                    marks: Vec::new(),
                },
            ],
        };
        assert_eq!(content.as_plain_text(), "Hello World");
    }

    #[test]
    fn test_block_content_refs() {
        let page_id = crate::value_objects::Uuid::new_v4();
        let block_id = crate::value_objects::Uuid::new_v4();

        let content = BlockContent {
            segments: vec![
                BlockSegment::Text {
                    content: "Check".to_string(),
                    marks: Vec::new(),
                },
                BlockSegment::PageRef {
                    target: page_id,
                    label: Some("My Page".to_string()),
                },
                BlockSegment::BlockRef { target: block_id },
                BlockSegment::Text {
                    content: "done".to_string(),
                    marks: Vec::new(),
                },
            ],
        };

        assert_eq!(content.page_refs(), vec![page_id]);
        assert_eq!(content.block_refs(), vec![block_id]);
    }

    #[test]
    fn test_split_at_middle() {
        let content = BlockContent::from_text("Hello World");
        // Position 5 is between "Hello" and " World" (after 'o', before space)
        let (left, right) = content.split_at(5);

        assert_eq!(left.as_plain_text(), "Hello");
        assert_eq!(right.as_plain_text(), " World");
    }

    #[test]
    fn test_split_at_start() {
        let content = BlockContent::from_text("Hello World");
        let (left, right) = content.split_at(0);

        assert!(left.is_empty());
        assert_eq!(right.as_plain_text(), "Hello World");
    }

    #[test]
    fn test_split_at_end() {
        let content = BlockContent::from_text("Hello World");
        // Position 11 is after last char
        let (left, right) = content.split_at(11);

        assert_eq!(left.as_plain_text(), "Hello World");
        assert!(right.is_empty());
    }

    #[test]
    fn test_split_at_beyond_end() {
        let content = BlockContent::from_text("Hello");
        let (left, right) = content.split_at(100);

        assert_eq!(left.as_plain_text(), "Hello");
        assert!(right.is_empty());
    }

    #[test]
    fn test_split_with_refs() {
        let page_id = crate::value_objects::Uuid::new_v4();
        let content = BlockContent {
            segments: vec![
                BlockSegment::Text {
                    content: "See ".to_string(),
                    marks: Vec::new(),
                },
                BlockSegment::PageRef {
                    target: page_id,
                    label: None,
                },
                BlockSegment::Text {
                    content: " for details".to_string(),
                    marks: Vec::new(),
                },
            ],
        };

        // Split at position 8: after "See [[page]]" but before "or"
        let (left, right) = content.split_at(8);
        assert_eq!(left.as_plain_text(), "See   for");
        assert_eq!(right.as_plain_text(), " details");
    }
}
