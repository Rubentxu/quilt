//! BlockFormat value object - content format (markdown or org)

use crate::errors::DomainError;
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
    pub fn parse_str(s: &str) -> Result<Self, DomainError> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Ok(BlockFormat::Markdown),
            "org" | "org-mode" => Ok(BlockFormat::Org),
            _ => Err(DomainError::ParseError(format!(
                "Invalid block format value: {}",
                s
            ))),
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
    fn test_default_is_markdown() {
        assert_eq!(BlockFormat::default(), BlockFormat::Markdown);
    }

    #[test]
    fn test_extension() {
        assert_eq!(BlockFormat::Markdown.extension(), "md");
        assert_eq!(BlockFormat::Org.extension(), "org");
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(BlockFormat::Markdown.mime_type(), "text/markdown");
        assert_eq!(BlockFormat::Org.mime_type(), "text/org");
    }

    #[test]
    fn test_parse_str_markdown_variants() {
        assert_eq!(
            BlockFormat::parse_str("markdown"),
            Ok(BlockFormat::Markdown)
        );
        assert_eq!(
            BlockFormat::parse_str("MARKDOWN"),
            Ok(BlockFormat::Markdown)
        );
        assert_eq!(
            BlockFormat::parse_str("Markdown"),
            Ok(BlockFormat::Markdown)
        );
        assert_eq!(BlockFormat::parse_str("md"), Ok(BlockFormat::Markdown));
        assert_eq!(BlockFormat::parse_str("MD"), Ok(BlockFormat::Markdown));
    }

    #[test]
    fn test_parse_str_org_variants() {
        assert_eq!(BlockFormat::parse_str("org"), Ok(BlockFormat::Org));
        assert_eq!(BlockFormat::parse_str("ORG"), Ok(BlockFormat::Org));
        assert_eq!(BlockFormat::parse_str("org-mode"), Ok(BlockFormat::Org));
        assert_eq!(BlockFormat::parse_str("ORG-MODE"), Ok(BlockFormat::Org));
    }

    #[test]
    fn test_parse_str_invalid() {
        assert!(BlockFormat::parse_str("html").is_err());
        assert!(BlockFormat::parse_str("").is_err());
        assert!(BlockFormat::parse_str("text").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", BlockFormat::Markdown), "markdown");
        assert_eq!(format!("{}", BlockFormat::Org), "org");
    }

    #[test]
    fn test_serde_roundtrip() {
        for fmt in &[BlockFormat::Markdown, BlockFormat::Org] {
            let json = serde_json::to_string(fmt).unwrap();
            let restored: BlockFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(*fmt, restored);
        }
    }

    #[test]
    fn test_serde_rejects_unknown() {
        assert!(serde_json::from_str::<BlockFormat>("\"html\"").is_err());
    }
}
