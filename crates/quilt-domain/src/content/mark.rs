//! Mark - text formatting within a text segment

use serde::{Deserialize, Serialize};

/// Mark represents inline formatting applied to a portion of text.
///
/// Marks can be:
/// - Bold, Italic, Strikethrough, Code (inline formatting)
/// - Highlight with a color
/// - Link to a URL
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Mark {
    /// Bold text (**bold**)
    Bold,
    /// Italic text (*italic*)
    Italic,
    /// Strikethrough text (~~strikethrough~~)
    Strikethrough,
    /// Inline code (`code`)
    Code,
    /// Highlighted text with a color
    Highlight {
        /// The highlight color (e.g., "#ffff00" for yellow)
        color: String,
    },
    /// A hyperlink
    Link {
        /// The URL
        url: String,
        /// Optional display label
        label: Option<String>,
    },
}

impl Mark {
    /// Check if this mark represents a formatting change only (no data).
    /// Used for simplified equality checks.
    pub fn is_formatting_only(&self) -> bool {
        match self {
            Mark::Bold
            | Mark::Italic
            | Mark::Strikethrough
            | Mark::Code => true,
            Mark::Highlight { .. } | Mark::Link { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_is_formatting_only() {
        assert!(Mark::Bold.is_formatting_only());
        assert!(Mark::Italic.is_formatting_only());
        assert!(Mark::Strikethrough.is_formatting_only());
        assert!(Mark::Code.is_formatting_only());
        assert!(!Mark::Highlight { color: "#ffff00".to_string() }.is_formatting_only());
        assert!(!Mark::Link { url: "http://example.com".to_string(), label: None }.is_formatting_only());
    }
}
