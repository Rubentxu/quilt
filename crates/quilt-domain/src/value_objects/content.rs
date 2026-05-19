//! Content value object - represents block content
//!
//! This value object encapsulates the content of a block: its text
/// and formatting.

use crate::value_objects::BlockFormat;

/// Content represents the actual content of a block.
///
/// It consists of:
/// - `content`: The actual content text
/// - `format`: The content format (Markdown or Org mode)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    /// The actual content text
    pub content: String,
    /// Content format
    pub format: BlockFormat,
}

impl Content {
    /// Create new content with default Markdown format.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            format: BlockFormat::Markdown,
        }
    }

    /// Create new content with specified format.
    pub fn with_format(content: impl Into<String>, format: BlockFormat) -> Self {
        Self {
            content: content.into(),
            format,
        }
    }

    /// Create empty content with Markdown format.
    pub fn empty() -> Self {
        Self {
            content: String::new(),
            format: BlockFormat::Markdown,
        }
    }

    /// Check if content is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Get the content length.
    pub fn len(&self) -> usize {
        self.content.len()
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::empty()
    }
}

impl AsRef<str> for Content {
    fn as_ref(&self) -> &str {
        &self.content
    }
}

impl std::ops::Deref for Content {
    type Target = str;

    fn deref(&self) -> &str {
        &self.content
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_new() {
        let content = Content::new("Hello, world!");
        assert_eq!(content.content, "Hello, world!");
        assert_eq!(content.format, BlockFormat::Markdown);
    }

    #[test]
    fn test_content_with_format() {
        let content = Content::with_format("*bold*", BlockFormat::Org);
        assert_eq!(content.content, "*bold*");
        assert_eq!(content.format, BlockFormat::Org);
    }

    #[test]
    fn test_content_empty() {
        let content = Content::empty();
        assert!(content.is_empty());
        assert_eq!(content.len(), 0);
    }

    #[test]
    fn test_content_from_string() {
        let content: Content = "test".to_string().into();
        assert_eq!(content.content, "test");
    }
}
