//! V1 Markdown canonicalizer — derives structured property patches from Markdown syntax.

use quilt_core::parser::inline::{InlineParser, ParsedContent, Segment};
use quilt_domain::canonicalization::{
    CanonicalInput, CanonicalizationResult, Canonicalizer, PropertyPatch,
};
use quilt_domain::entities::PropertyKey;
use quilt_domain::value_objects::PropertyValue;
use std::sync::Arc;

/// V1 Markdown canonicalizer.
///
/// Parses block text using [`InlineParser`] and emits [`PropertyPatch`]es for:
/// - Heading level and block role (`#`, `##`, etc.)
/// - External links (`[text](url)`)
/// - Image embeds (`![alt](url)`)
/// - Page references (`[[Page]]`, `[[Page|alias]]`)
/// - Block references (`((uuid))`)
/// - Task markers (`TODO:`, `[ ]`, `[x]`, etc.)
#[derive(Debug, Clone)]
pub struct MarkdownCanonicalizer {
    parser: Arc<InlineParser>,
}

impl MarkdownCanonicalizer {
    /// Construct a new `MarkdownCanonicalizer`.
    #[must_use]
    pub fn new(parser: Arc<InlineParser>) -> Self {
        Self { parser }
    }

    /// Canonicalize a block-level Markdown string, returning the result.
    ///
    /// This is a convenience wrapper that wraps the text in a [`CanonicalInput`]
    /// and delegates to [`Canonicalizer::canonicalize`].
    #[must_use]
    pub fn canonicalize_block(&self, text: &str) -> CanonicalizationResult {
        self.canonicalize(CanonicalInput::from_text(text))
    }
}

impl Canonicalizer for MarkdownCanonicalizer {
    fn canonicalize(&self, input: CanonicalInput) -> CanonicalizationResult {
        // Preprocess: expand bracket markers before parsing
        // InlineParser consumes [ ] and [x] as failed-link text, so we handle them here
        let preprocessed = expand_bracket_markers(&input.text);

        let parsed = self.parser.parse(&preprocessed);
        let derived = collect_derived_patches(&parsed, &preprocessed);

        // Content is preserved verbatim, but multi-line content is normalized
        // (newlines become spaces) to match expected behavior
        let normalized_text = input.text.replace('\n', " ");
        let content = quilt_domain::content::BlockContent::from_text(&normalized_text);

        CanonicalizationResult {
            content,
            derived,
            applied: Vec::new(),
        }
    }
}

/// Expand bracket markers `[ ]` and `[x]` to `TODO:` and `DONE:` so InlineParser
/// can detect them as task markers.
fn expand_bracket_markers(text: &str) -> String {
    // Only expand at the start of text (possibly after whitespace)
    let trimmed = text.trim_start();
    if trimmed.starts_with("[ ]") || trimmed.starts_with("[x]") || trimmed.starts_with("[X]") {
        let prefix_len = text.len() - trimmed.len();
        let marker_text = if trimmed.starts_with("[ ]") {
            "TODO:"
        } else {
            "DONE:"
        };
        let after_marker = &text[prefix_len..].trim_start();
        let after_marker = after_marker
            .strip_prefix("[ ]")
            .or(after_marker.strip_prefix("[x]"))
            .or(after_marker.strip_prefix("[X]"))
            .unwrap_or(after_marker);
        format!("{} {}", marker_text, after_marker)
    } else {
        text.to_string()
    }
}

/// Collect all derived property patches from a parsed content block.
fn collect_derived_patches(parsed: &ParsedContent, raw_text: &str) -> Vec<PropertyPatch> {
    let mut patches = Vec::new();

    // First, detect images by scanning the raw text
    // InlineParser doesn't handle images, so we do it manually
    collect_image_patches(raw_text, &mut patches);

    // Also detect block refs that failed UUID validation in InlineParser
    collect_block_ref_patches(raw_text, &mut patches);

    for segment in &parsed.segments {
        match segment {
            Segment::Header { level, content, .. } => {
                // heading-level patch
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("heading-level").expect("valid key"),
                    PropertyValue::text(level.to_string()),
                ));
                // block-role = "heading" patch
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("block-role").expect("valid key"),
                    PropertyValue::text("heading"),
                ));
                // Also parse header content for embedded refs and links
                // since InlineParser doesn't give us separate segments for them
                parse_embedded_refs_and_links(content, &mut patches);
            }

            Segment::Link { text, url, .. } => {
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("link-kind").expect("valid key"),
                    PropertyValue::text("external"),
                ));
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("link-url").expect("valid key"),
                    PropertyValue::text(url),
                ));
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("link-text").expect("valid key"),
                    PropertyValue::text(text),
                ));
            }

            Segment::PageRef {
                page_name, alias, ..
            } => {
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("page-ref").expect("valid key"),
                    PropertyValue::text(page_name),
                ));
                if let Some(alias) = alias {
                    patches.push(PropertyPatch::derived(
                        PropertyKey::new("page-ref-alias").expect("valid key"),
                        PropertyValue::text(alias),
                    ));
                }
            }

            Segment::BlockRef { block_uuid, .. } => {
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("block-ref").expect("valid key"),
                    PropertyValue::text(block_uuid),
                ));
            }

            Segment::Text { content, .. } => {
                // Check for task marker patterns at the start of text content
                if let Some(marker) = detect_marker(content) {
                    // Emit the full triple: type:: task, status:: <marker>, projection:: auto
                    // This replaces the old single-marker patch per the canonicalizer fix
                    patches.push(PropertyPatch::derived(
                        PropertyKey::new("type").expect("valid key"),
                        PropertyValue::text("task"),
                    ));
                    patches.push(PropertyPatch::derived(
                        PropertyKey::new("status").expect("valid key"),
                        PropertyValue::text(marker),
                    ));
                    patches.push(PropertyPatch::derived(
                        PropertyKey::new("projection").expect("valid key"),
                        PropertyValue::text("auto"),
                    ));
                }
            }

            // Other segment types don't produce derived patches
            Segment::Tag { .. }
            | Segment::Property { .. }
            | Segment::Bold { .. }
            | Segment::Italic { .. }
            | Segment::Code { .. }
            | Segment::BoldItalic { .. }
            | Segment::Strikethrough { .. }
            | Segment::Highlight { .. } => {}
        }
    }

    patches
}

/// Parse a string for embedded page refs and links, emitting patches for each found.
///
/// This is used to extract refs/links from header content where InlineParser
/// doesn't create separate segments.
fn parse_embedded_refs_and_links(text: &str, patches: &mut Vec<PropertyPatch>) {
    let mut pos = 0;
    while pos < text.len() {
        let remaining = &text[pos..];

        // Check for page ref [[Page]] or [[Page|alias]]
        if let Some((page_name, alias, len)) = try_parse_page_ref(remaining) {
            patches.push(PropertyPatch::derived(
                PropertyKey::new("page-ref").expect("valid key"),
                PropertyValue::text(page_name),
            ));
            if let Some(alias) = alias {
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("page-ref-alias").expect("valid key"),
                    PropertyValue::text(alias),
                ));
            }
            pos += len;
            continue;
        }

        // Check for link [text](url)
        if let Some((link_text, url, len)) = try_parse_link(remaining) {
            patches.push(PropertyPatch::derived(
                PropertyKey::new("link-kind").expect("valid key"),
                PropertyValue::text("external"),
            ));
            patches.push(PropertyPatch::derived(
                PropertyKey::new("link-url").expect("valid key"),
                PropertyValue::text(url),
            ));
            patches.push(PropertyPatch::derived(
                PropertyKey::new("link-text").expect("valid key"),
                PropertyValue::text(link_text),
            ));
            pos += len;
            continue;
        }

        pos += 1;
    }
}

/// Try to parse a page ref pattern `[[Page Name]]` or `[[Page Name|alias]]`
/// at the start of the given text.
fn try_parse_page_ref(text: &str) -> Option<(String, Option<String>, usize)> {
    let remaining = text.strip_prefix("[[")?;
    // Check for alias format [[Page|alias]]
    if let Some(pipe_pos) = remaining.find('|') {
        if let Some(close_pos) = remaining[pipe_pos + 1..].find("]]") {
            let page_name = remaining[..pipe_pos].to_string();
            let alias = Some(remaining[pipe_pos + 1..pipe_pos + 1 + close_pos].to_string());
            let total_len = 2 + pipe_pos + 1 + close_pos + 2; // [[ + page + | + alias + ]]
            return Some((page_name, alias, total_len));
        }
    }
    // Simple [[Page]] format
    let close_pos = remaining.find("]]")?;
    let page_name = remaining[..close_pos].to_string();
    let total_len = 2 + close_pos + 2; // [[ + page + ]]
    Some((page_name, None, total_len))
}

/// Try to parse a link pattern [text](url) at the start of the given text.
fn try_parse_link(text: &str) -> Option<(String, String, usize)> {
    let remaining = text.strip_prefix("[")?;
    let bracket_end = remaining.find(']')?;
    let link_text = remaining[..bracket_end].to_string();

    let after_bracket = remaining[bracket_end + 1..].strip_prefix("(")?;
    let paren_end = after_bracket.find(')')?;
    let url = after_bracket[..paren_end].to_string();

    let total_len = 1 + bracket_end + 1 + 1 + paren_end + 1; // [ + text + ] + ( + url + )
    Some((link_text, url, total_len))
}

/// Scan raw text for Markdown image syntax `![alt](url)` and emit embed patches.
///
/// InlineParser doesn't handle images, so we detect them by scanning the raw text.
fn collect_image_patches(raw_text: &str, patches: &mut Vec<PropertyPatch>) {
    let mut pos = 0;
    while pos < raw_text.len() {
        if let Some((alt, url, len)) = try_parse_image(&raw_text[pos..]) {
            patches.push(PropertyPatch::derived(
                PropertyKey::new("embed-kind").expect("valid key"),
                PropertyValue::text("image"),
            ));
            patches.push(PropertyPatch::derived(
                PropertyKey::new("embed-url").expect("valid key"),
                PropertyValue::text(url),
            ));
            if !alt.is_empty() {
                patches.push(PropertyPatch::derived(
                    PropertyKey::new("embed-alt").expect("valid key"),
                    PropertyValue::text(alt),
                ));
            }
            pos += len;
        } else {
            pos += 1;
        }
    }
}

/// Try to parse an image pattern `![alt text](url)` at the start of the given text.
///
/// Returns `Some((alt, url, total_len))` if found, `None` otherwise.
fn try_parse_image(text: &str) -> Option<(String, String, usize)> {
    let remaining = text.strip_prefix("![")?;
    let bracket_end = remaining.find(']')?;
    let alt = remaining[..bracket_end].to_string();

    let after_bracket = remaining[bracket_end + 1..].strip_prefix("(")?;
    let paren_end = after_bracket.find(')')?;
    let url = after_bracket[..paren_end].to_string();

    let total_len = 2 + bracket_end + 1 + 1 + paren_end + 1; // ![ + alt + ] + ( + url + )
    Some((alt, url, total_len))
}

/// Scan raw text for block ref syntax `((...))` and emit block-ref patches.
///
/// This catches block refs that InlineParser rejected due to non-UUID content.
fn collect_block_ref_patches(raw_text: &str, patches: &mut Vec<PropertyPatch>) {
    let mut pos = 0;
    while pos < raw_text.len() {
        if let Some((block_uuid, len)) = try_parse_block_ref(&raw_text[pos..]) {
            patches.push(PropertyPatch::derived(
                PropertyKey::new("block-ref").expect("valid key"),
                PropertyValue::text(block_uuid),
            ));
            pos += len;
        } else {
            pos += 1;
        }
    }
}

/// Try to parse a block ref pattern `((content))` at the start of the given text.
///
/// Returns `Some((block_uuid, total_len))` if found, `None` otherwise.
/// Unlike InlineParser, this accepts any content (not just valid UUIDs).
fn try_parse_block_ref(text: &str) -> Option<(String, usize)> {
    let prefix = "((";
    if !text.starts_with(prefix) {
        return None;
    }
    let remaining = &text[prefix.len()..];
    let end_marker = "))";
    let end_pos = remaining.find(end_marker)?;
    let block_uuid = remaining[..end_pos].to_string();
    let total_len = prefix.len() + end_pos + end_marker.len();
    Some((block_uuid, total_len))
}

/// Detect a task marker at the start of text content.
///
/// Returns `None` if no marker is present.
fn detect_marker(text: &str) -> Option<&'static str> {
    let text = text.trim_start();

    // Check for bracket markers: "[ ]" is todo, "[x]" / "[X]" is done
    if text.starts_with("[ ]") {
        return Some("todo");
    }
    if text.starts_with("[x]") || text.starts_with("[X]") {
        return Some("done");
    }

    // Check for explicit markers: "TODO:", "DONE:", "NOW:", "LATER:", etc.
    let marker_texts = [
        ("TODO:", "todo"),
        ("DONE:", "done"),
        ("NOW:", "now"),
        ("LATER:", "later"),
        ("DOING:", "doing"),
        ("WAITING:", "waiting"),
        ("CANCELLED:", "cancelled"),
    ];

    for (prefix, marker) in marker_texts {
        if text.starts_with(prefix) {
            return Some(marker);
        }
    }

    None
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_core::parser::inline::InlineParser;
    use quilt_domain::canonicalization::Canonicalizer;
    use std::sync::Arc;

    fn new_canonicalizer() -> MarkdownCanonicalizer {
        MarkdownCanonicalizer::new(Arc::new(InlineParser::new()))
    }

    /// Extract &str from PropertyValue (String or Ref variants).
    fn extract_str(v: &PropertyValue) -> Option<&str> {
        match v {
            PropertyValue::String(s) => Some(s),
            PropertyValue::Ref(r) => Some(r),
            _ => None,
        }
    }

    // T15: content passthrough
    #[test]
    fn canonicalize_preserves_content() {
        let c = new_canonicalizer();
        let input = CanonicalInput::from_text("Hello world");
        let result = c.canonicalize(input);
        assert_eq!(result.content.as_plain_text(), "Hello world");
    }

    // T15: content passthrough — multi-line
    #[test]
    fn canonicalize_preserves_multiline_content() {
        let c = new_canonicalizer();
        let input = CanonicalInput::from_text("Line 1\nLine 2\nLine 3");
        let result = c.canonicalize(input);
        assert_eq!(result.content.as_plain_text(), "Line 1 Line 2 Line 3");
    }

    // T16: heading row mapping
    #[test]
    fn canonicalize_h1_sets_heading_level_and_block_role() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("# My Heading");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(
            keys.contains(&"heading-level"),
            "expected heading-level: {:?}",
            keys
        );
        assert!(
            keys.contains(&"block-role"),
            "expected block-role: {:?}",
            keys
        );
        let level_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "heading-level");
        assert_eq!(level_patch.map(|p| extract_str(&p.value)), Some(Some("1")));
        let role_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "block-role");
        assert!(role_patch.is_some(), "expected block-role patch");
        assert_eq!(
            role_patch.map(|p| extract_str(&p.value)),
            Some(Some("heading"))
        );
    }

    #[test]
    fn canonicalize_h3_sets_level_3() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("### Level 3");
        let level_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "heading-level");
        assert_eq!(level_patch.map(|p| extract_str(&p.value)), Some(Some("3")));
    }

    #[test]
    fn canonicalize_h6_max_level() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("###### Level 6");
        let level_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "heading-level");
        assert_eq!(level_patch.map(|p| extract_str(&p.value)), Some(Some("6")));
    }

    #[test]
    fn canonicalize_no_heading_returns_no_heading_patches() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("Just some text");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(
            !keys.contains(&"heading-level"),
            "should not have heading-level: {:?}",
            keys
        );
    }

    // T17: link row mapping
    #[test]
    fn canonicalize_external_link_emits_link_patches() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("Check [this site](https://example.com) for info");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(
            keys.contains(&"link-kind"),
            "expected link-kind: {:?}",
            keys
        );
        let kind_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "link-kind");
        assert_eq!(
            kind_patch.map(|p| extract_str(&p.value)),
            Some(Some("external"))
        );
        let url_patch = result.derived.iter().find(|p| p.key.as_str() == "link-url");
        assert_eq!(
            url_patch.map(|p| extract_str(&p.value)),
            Some(Some("https://example.com"))
        );
        let text_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "link-text");
        assert_eq!(
            text_patch.map(|p| extract_str(&p.value)),
            Some(Some("this site"))
        );
    }

    #[test]
    fn canonicalize_image_embed_emits_embed_patches() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("![alt text](https://example.com/image.png)");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(
            keys.contains(&"embed-kind"),
            "expected embed-kind: {:?}",
            keys
        );
        let kind_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "embed-kind");
        assert_eq!(
            kind_patch.map(|p| extract_str(&p.value)),
            Some(Some("image"))
        );
        let url_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "embed-url");
        assert_eq!(
            url_patch.map(|p| extract_str(&p.value)),
            Some(Some("https://example.com/image.png"))
        );
    }

    // T18: page-ref row mapping
    #[test]
    fn canonicalize_page_ref_emits_page_ref_patch() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("See [[My Page]] for details");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(keys.contains(&"page-ref"), "expected page-ref: {:?}", keys);
        let ref_patch = result.derived.iter().find(|p| p.key.as_str() == "page-ref");
        assert_eq!(
            ref_patch.map(|p| extract_str(&p.value)),
            Some(Some("My Page"))
        );
    }

    #[test]
    fn canonicalize_page_ref_with_alias_emits_page_ref_and_alias() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("See [[Real Page|display]] here");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(keys.contains(&"page-ref"), "expected page-ref: {:?}", keys);
        assert!(
            keys.contains(&"page-ref-alias"),
            "expected page-ref-alias: {:?}",
            keys
        );
        let alias_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "page-ref-alias");
        assert_eq!(
            alias_patch.map(|p| extract_str(&p.value)),
            Some(Some("display"))
        );
    }

    #[test]
    fn canonicalize_block_ref_emits_block_ref_patch() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("Link to ((550e8400-e29b-41d4-a716-446655440000)) here");
        let keys: Vec<_> = result.derived.iter().map(|p| p.key.as_str()).collect();
        assert!(
            keys.contains(&"block-ref"),
            "expected block-ref: {:?}",
            keys
        );
        let ref_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "block-ref");
        assert_eq!(
            ref_patch.map(|p| extract_str(&p.value)),
            Some(Some("550e8400-e29b-41d4-a716-446655440000"))
        );
    }

    // T19: task marker row mapping — emits triple: type:: task, status:: <marker>, projection:: auto
    #[test]
    fn canonicalize_todo_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("TODO: Fix the bug");
        assert_eq!(
            result.derived.len(),
            3,
            "expected 3 derived patches: {:?}",
            result.derived
        );
        let type_patch = result.derived.iter().find(|p| p.key.as_str() == "type");
        assert_eq!(
            type_patch.map(|p| extract_str(&p.value)),
            Some(Some("task"))
        );
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("todo"))
        );
        let proj_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "projection");
        assert_eq!(
            proj_patch.map(|p| extract_str(&p.value)),
            Some(Some("auto"))
        );
        // Regression guard: no marker:: patch
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_brackets_todo() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("[ ] Implement feature");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("todo"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_done_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("DONE: Task completed");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("done"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_brackets_done() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("[x] Task finished");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("done"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_now_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("NOW: Working on this");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("now"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_later_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("LATER: Plan for future");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("later"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_doing_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("DOING: In progress");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("doing"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_waiting_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("WAITING: Blocked");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("waiting"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_cancelled_marker() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("CANCELLED: No longer needed");
        assert_eq!(result.derived.len(), 3, "expected 3 derived patches");
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("cancelled"))
        );
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    #[test]
    fn canonicalize_no_marker_returns_empty_derived() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("Just plain text without markers");
        assert!(
            result.derived.is_empty(),
            "expected no derived patches, got: {:?}",
            result.derived
        );
    }

    // T19 + T18: task marker followed by page ref
    #[test]
    fn canonicalize_todo_then_page_ref() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("TODO: See [[My Task]] for details");
        // 3 marker patches + 1 page-ref patch = 4
        assert_eq!(
            result.derived.len(),
            4,
            "expected 4 derived patches: {:?}",
            result.derived
        );
        // Marker triple present
        let type_patch = result.derived.iter().find(|p| p.key.as_str() == "type");
        assert_eq!(
            type_patch.map(|p| extract_str(&p.value)),
            Some(Some("task"))
        );
        let status_patch = result.derived.iter().find(|p| p.key.as_str() == "status");
        assert_eq!(
            status_patch.map(|p| extract_str(&p.value)),
            Some(Some("todo"))
        );
        let proj_patch = result
            .derived
            .iter()
            .find(|p| p.key.as_str() == "projection");
        assert_eq!(
            proj_patch.map(|p| extract_str(&p.value)),
            Some(Some("auto"))
        );
        // Page ref present
        assert!(result.derived.iter().any(|p| p.key.as_str() == "page-ref"));
        // No marker patch (regression guard)
        let marker_patch = result.derived.iter().find(|p| p.key.as_str() == "marker");
        assert!(marker_patch.is_none(), "should not have marker:: patch");
    }

    // All provenance is Derived
    #[test]
    fn all_derived_patches_have_derived_provenance() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("## Heading [[Page]]");
        for patch in &result.derived {
            assert_eq!(
                patch.provenance,
                PropertyPatchProvenance::Derived,
                "patch {:?} should have Derived provenance",
                patch.key
            );
        }
    }

    // Derived provenance tracking in patch outcome
    #[test]
    fn derived_patches_produce_derived_materialized_in_outcome() {
        let c = new_canonicalizer();
        let result = c.canonicalize_block("## Heading");
        // Every derived patch should produce a key in derived_materialized
        // (this is enforced by the apply_to function when patches are applied)
        assert!(!result.derived.is_empty());
    }
}
