//! BlockSegment - typed content segments for block content
//!
//! Each block's content is a sequence of segments. Segments can be:
//! - Text with optional formatting marks
//! - Page references [[Page]]
//! - Block references ((Block))
//! - Images
//! - Code blocks
//! - Tables
//! - Dates
//! - Tags

use crate::Mark;
use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A segment in a block's content.
///
/// BlockSegment represents one unit of content within a block. Multiple segments
/// combine to form the complete content of a block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockSegment {
    /// Plain text with optional formatting marks.
    ///
    /// Example: "Hello **world**" might be two segments:
    /// - Text { content: "Hello ", marks: [] }
    /// - Text { content: "world", marks: [Bold] }
    Text {
        /// The text content
        content: String,
        /// Inline formatting marks applied to this text
        marks: Vec<super::Mark>,
    },

    /// A reference to another page: [[Page Name]]
    PageRef {
        /// The target page's UUID
        target: Uuid,
        /// Optional display label (defaults to page name)
        label: Option<String>,
    },

    /// A reference to another block: ((Block UUID))
    ///
    /// When rendered, this shows a transclusion of the referenced block.
    BlockRef {
        /// The target block's UUID
        target: Uuid,
    },

    /// An embedded image.
    Image {
        /// The image URL or path
        url: String,
        /// Optional alt text for accessibility
        alt: Option<String>,
    },

    /// A code block with syntax highlighting.
    Code {
        /// The programming language (e.g., "rust", "python")
        language: String,
        /// The source code
        source: String,
    },

    /// A table with headers and rows.
    Table {
        /// Column headers
        headers: Vec<String>,
        /// Data rows (each row has the same number of cells as headers)
        rows: Vec<Vec<String>>,
    },

    /// A date value.
    Date {
        /// The date value (stored as YYYYMMDD integer)
        value: i32,
    },

    /// A tag value (e.g., #tag)
    Tag {
        /// The tag value (without the #)
        value: String,
    },
}

impl BlockSegment {
    /// Check if this segment represents editable text content.
    pub fn is_text(&self) -> bool {
        matches!(self, BlockSegment::Text { .. })
    }

    /// Check if this segment is a reference (page or block).
    pub fn is_ref(&self) -> bool {
        matches!(
            self,
            BlockSegment::PageRef { .. } | BlockSegment::BlockRef { .. }
        )
    }

    /// Check if this segment is an embed (image, code, table).
    pub fn is_embed(&self) -> bool {
        matches!(
            self,
            BlockSegment::Image { .. } | BlockSegment::Code { .. } | BlockSegment::Table { .. }
        )
    }

    /// Get plain text from a text segment, or None if not text.
    pub fn as_text(&self) -> Option<&str> {
        if let BlockSegment::Text { content, .. } = self {
            Some(content)
        } else {
            None
        }
    }

    /// Get the target UUID from a reference segment, or None if not a ref.
    pub fn as_ref_target(&self) -> Option<Uuid> {
        match self {
            BlockSegment::PageRef { target, .. } => Some(*target),
            BlockSegment::BlockRef { target } => Some(*target),
            _ => None,
        }
    }

    /// Get the length of text content in this segment.
    ///
    /// For Text segments, returns the content length.
    /// For non-text segments (refs, images, etc.), returns 0.
    pub fn text_len(&self) -> usize {
        match self {
            BlockSegment::Text { content, .. } => content.chars().count(),
            _ => 0,
        }
    }

    /// Convert this segment to a Markdown string.
    pub fn to_markdown(&self) -> String {
        match self {
            BlockSegment::Text { content, marks } => {
                let mut result = content.clone();
                // Apply marks in reverse order so nested marks work correctly
                // Order: Link, Highlight, Code, Strikethrough, Italic, Bold
                for mark in marks {
                    match mark {
                        Mark::Bold => result = format!("**{}**", result),
                        Mark::Italic => result = format!("*{}*", result),
                        Mark::Strikethrough => result = format!("~~{}~~", result),
                        Mark::Code => result = format!("`{}`", result),
                        Mark::Highlight { color: _ } => {
                            // TODO: Quilt uses colored highlights, standard Markdown doesn't
                            // For now, render without the highlight marker
                        }
                        Mark::Link { url, label } => {
                            if let Some(label) = label {
                                result = format!("[{}]({})", label, url);
                            } else {
                                result = format!("[{}]({})", result, url);
                            }
                        }
                    }
                }
                result
            }
            BlockSegment::PageRef { target: _, label } => {
                if let Some(label) = label {
                    format!("[[{}]]", label)
                } else {
                    // Cannot render UUID-only reference meaningfully in Markdown
                    String::new()
                }
            }
            BlockSegment::BlockRef { target } => {
                format!("(({}))", target)
            }
            BlockSegment::Image { url, alt } => {
                if let Some(alt) = alt {
                    format!("![{}]({})", alt, url)
                } else {
                    format!("![]({})", url)
                }
            }
            BlockSegment::Code { language, source } => {
                format!("```{}\n{}\n```", language, source)
            }
            BlockSegment::Table { headers, rows } => {
                // Simple Markdown table rendering
                let mut result = String::new();
                // Header row
                result.push_str("| ");
                result.push_str(&headers.join(" | "));
                result.push_str(" |\n");
                // Separator row
                result.push_str("| ");
                result.push_str(
                    &headers
                        .iter()
                        .map(|_| "---".to_string())
                        .collect::<Vec<_>>()
                        .join(" | "),
                );
                result.push_str(" |\n");
                // Data rows
                for row in rows {
                    result.push_str("| ");
                    result.push_str(&row.join(" | "));
                    result.push_str(" |\n");
                }
                result
            }
            BlockSegment::Date { value } => {
                // Format date as YYYY-MM-DD
                let date_str = value.to_string();
                if date_str.len() == 8 {
                    format!(
                        "{}-{}-{}",
                        &date_str[0..4],
                        &date_str[4..6],
                        &date_str[6..8]
                    )
                } else {
                    date_str
                }
            }
            BlockSegment::Tag { value } => {
                format!("#{}", value)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_segment_is_text() {
        let text_seg = BlockSegment::Text {
            content: "Hello".to_string(),
            marks: Vec::new(),
        };
        assert!(text_seg.is_text());
        assert!(!text_seg.is_ref());
        assert!(!text_seg.is_embed());

        let page_ref = BlockSegment::PageRef {
            target: Uuid::new_v4(),
            label: None,
        };
        assert!(!page_ref.is_text());
        assert!(page_ref.is_ref());
        assert!(!page_ref.is_embed());
    }

    #[test]
    fn test_block_segment_as_text() {
        let text_seg = BlockSegment::Text {
            content: "Hello".to_string(),
            marks: Vec::new(),
        };
        assert_eq!(text_seg.as_text(), Some("Hello"));

        let page_ref = BlockSegment::PageRef {
            target: Uuid::new_v4(),
            label: None,
        };
        assert_eq!(page_ref.as_text(), None);
    }

    #[test]
    fn test_block_segment_as_ref_target() {
        let id = Uuid::new_v4();
        let page_ref = BlockSegment::PageRef {
            target: id,
            label: None,
        };
        assert_eq!(page_ref.as_ref_target(), Some(id));

        let text_seg = BlockSegment::Text {
            content: "Hello".to_string(),
            marks: Vec::new(),
        };
        assert_eq!(text_seg.as_ref_target(), None);
    }

    #[test]
    fn test_block_segment_text_len() {
        let text_seg = BlockSegment::Text {
            content: "Hello".to_string(),
            marks: Vec::new(),
        };
        assert_eq!(text_seg.text_len(), 5);

        let page_ref = BlockSegment::PageRef {
            target: Uuid::new_v4(),
            label: None,
        };
        assert_eq!(page_ref.text_len(), 0);

        let block_ref = BlockSegment::BlockRef {
            target: Uuid::new_v4(),
        };
        assert_eq!(block_ref.text_len(), 0);

        let image = BlockSegment::Image {
            url: "http://example.com/img.png".to_string(),
            alt: None,
        };
        assert_eq!(image.text_len(), 0);

        // Test unicode
        let unicode_seg = BlockSegment::Text {
            content: "你好".to_string(),
            marks: Vec::new(),
        };
        assert_eq!(unicode_seg.text_len(), 2);
    }
}
