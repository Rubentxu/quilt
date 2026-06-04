//! Integration tests for BlockFormat — extension, mime_type,
//! parse_str edge cases, Display, Default, and serde roundtrip.

use quilt_domain::value_objects::BlockFormat;

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
        Some(BlockFormat::Markdown)
    );
    assert_eq!(
        BlockFormat::parse_str("MARKDOWN"),
        Some(BlockFormat::Markdown)
    );
    assert_eq!(
        BlockFormat::parse_str("Markdown"),
        Some(BlockFormat::Markdown)
    );
    assert_eq!(BlockFormat::parse_str("md"), Some(BlockFormat::Markdown));
    assert_eq!(BlockFormat::parse_str("MD"), Some(BlockFormat::Markdown));
}

#[test]
fn test_parse_str_org_variants() {
    assert_eq!(BlockFormat::parse_str("org"), Some(BlockFormat::Org));
    assert_eq!(BlockFormat::parse_str("ORG"), Some(BlockFormat::Org));
    assert_eq!(BlockFormat::parse_str("org-mode"), Some(BlockFormat::Org));
    assert_eq!(BlockFormat::parse_str("ORG-MODE"), Some(BlockFormat::Org));
}

#[test]
fn test_parse_str_invalid() {
    assert_eq!(BlockFormat::parse_str("html"), None);
    assert_eq!(BlockFormat::parse_str(""), None);
    assert_eq!(BlockFormat::parse_str("text"), None);
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
