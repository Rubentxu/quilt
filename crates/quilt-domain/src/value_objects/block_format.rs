//! BlockFormat value object - content format (markdown or org)

use std::fmt;

/// BlockFormat represents the content format of a block.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum BlockFormat {
    /// Markdown format
    #[default]
    Markdown,
    /// Emacs Org mode format
    Org,
}

impl BlockFormat {
    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            BlockFormat::Markdown => "md",
            BlockFormat::Org => "org",
        }
    }

    /// Get the mime type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            BlockFormat::Markdown => "text/markdown",
            BlockFormat::Org => "text/org",
        }
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Some(BlockFormat::Markdown),
            "org" | "org-mode" => Some(BlockFormat::Org),
            _ => None,
        }
    }
}

impl fmt::Display for BlockFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockFormat::Markdown => write!(f, "markdown"),
            BlockFormat::Org => write!(f, "org"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        assert_eq!(BlockFormat::Markdown.extension(), "md");
        assert_eq!(BlockFormat::Org.extension(), "org");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            BlockFormat::parse_str("markdown"),
            Some(BlockFormat::Markdown)
        );
        assert_eq!(BlockFormat::parse_str("md"), Some(BlockFormat::Markdown));
        assert_eq!(BlockFormat::parse_str("org"), Some(BlockFormat::Org));
        assert_eq!(BlockFormat::parse_str("unknown"), None);
    }
}
