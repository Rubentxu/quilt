//! Decoration management for semantic inline content.
//!
//! Converts parsed segments (tags, refs, properties) into visual
//! decoration descriptors that can be used by UI components.
//!
//! This module is the **only** bridge between the parser layer and
//! the rendering layer — components should NOT apply decorations
//! based on raw `Segment` matching.

use crate::parser::inline::{ParsedContent, Range, Segment};

/// Visual decoration kind for a text range.
#[derive(Debug, Clone, PartialEq)]
pub enum DecorationKind {
    /// Page link `[[Page Name]]`
    PageLink { page_name: String },
    /// Block link `((uuid))`
    BlockLink { block_uuid: String },
    /// Tag `#tagname`
    Tag { tag_name: String },
    /// Property key `property::`
    Property { key: String },
    /// Search match in content
    SearchMatch { query: String },
    /// Active autocomplete region (used by editor internally)
    AutocompleteActive { index: usize },
}

/// A visual decoration over a range of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoration {
    pub range: Range,
    pub kind: DecorationKind,
}

/// A rendered text segment with optional decoration.
///
/// This is the display-oriented output that UI components can render
/// directly without interpreting Segment or Decoration internals.
#[derive(Debug, Clone, PartialEq)]
pub struct DecoratedTextSegment {
    /// Plain text content
    pub text: String,
    /// CSS class to apply (empty string = no decoration)
    pub css_class: &'static str,
    /// Human-readable label for the decorated element (empty if plain text)
    pub label: String,
}

/// Converts parsed block content into visual decorations.
///
/// This is a pure transformation — no I/O, no mutation, no side effects.
/// It can be called on every keystroke without performance concerns since
/// it only transforms already-parsed structures.
#[derive(Debug, Clone, Default)]
pub struct DecorationManager;

impl DecorationManager {
    /// Build decorations from parsed content.
    ///
    /// Each `Segment` in the parsed content produces a `Decoration`
    /// with the appropriate `DecorationKind` and the same character range.
    pub fn build_decorations(parsed: &ParsedContent) -> Vec<Decoration> {
        let mut decorations = Vec::new();
        for segment in &parsed.segments {
            match segment {
                Segment::PageRef {
                    page_name, range, ..
                } => {
                    decorations.push(Decoration {
                        range: range.clone(),
                        kind: DecorationKind::PageLink {
                            page_name: page_name.clone(),
                        },
                    });
                }
                Segment::BlockRef {
                    block_uuid, range, ..
                } => {
                    decorations.push(Decoration {
                        range: range.clone(),
                        kind: DecorationKind::BlockLink {
                            block_uuid: block_uuid.clone(),
                        },
                    });
                }
                Segment::Tag { name, range, .. } => {
                    decorations.push(Decoration {
                        range: range.clone(),
                        kind: DecorationKind::Tag {
                            tag_name: name.clone(),
                        },
                    });
                }
                Segment::Property { key, range, .. } => {
                    decorations.push(Decoration {
                        range: range.clone(),
                        kind: DecorationKind::Property { key: key.clone() },
                    });
                }
                Segment::Text { .. } => {} // No decoration for plain text
            }
        }
        decorations
    }

    /// Produce display-oriented segments from raw text and decorations.
    ///
    /// The output preserves text ordering; callers can iterate and render
    /// each segment with the provided `css_class` and `label`.
    ///
    /// This is the primary rendering entry point for UI components.
    pub fn decorated_segments(content: &str, parsed: &ParsedContent) -> Vec<DecoratedTextSegment> {
        let decorations = Self::build_decorations(parsed);
        Self::apply_decorations_to_text(content, &decorations)
    }

    /// Apply decorations to text, producing decorated segments.
    ///
    /// Internal logic:
    /// 1. Sort decorations by range start.
    /// 2. Walk the text, emitting plain segments before each decoration.
    /// 3. Emit the decorated segment with its CSS class.
    fn apply_decorations_to_text(
        content: &str,
        decorations: &[Decoration],
    ) -> Vec<DecoratedTextSegment> {
        if content.is_empty() {
            return vec![];
        }

        let mut sorted: Vec<&Decoration> = decorations.iter().collect();
        sorted.sort_by_key(|d| d.range.start);

        let mut segments = Vec::new();
        let mut pos = 0;

        for deco in sorted {
            // Emit plain text before this decoration
            if deco.range.start > pos {
                let text = &content[pos..deco.range.start];
                segments.push(DecoratedTextSegment {
                    text: text.to_string(),
                    css_class: "",
                    label: String::new(),
                });
            }

            // Emit decorated segment
            let text = &content[deco.range.start..deco.range.end];
            let (css_class, label) = match &deco.kind {
                DecorationKind::PageLink { page_name } => {
                    ("decoration-page-ref", page_name.clone())
                }
                DecorationKind::BlockLink { block_uuid } => {
                    ("decoration-block-ref", block_uuid.clone())
                }
                DecorationKind::Tag { tag_name } => ("decoration-tag", format!("#{}", tag_name)),
                DecorationKind::Property { key } => ("decoration-property", key.clone()),
                DecorationKind::SearchMatch { query } => ("decoration-search", query.clone()),
                DecorationKind::AutocompleteActive { .. } => {
                    ("decoration-autocomplete", String::new())
                }
            };

            segments.push(DecoratedTextSegment {
                text: text.to_string(),
                css_class,
                label,
            });

            pos = deco.range.end;
        }

        // Emit remaining plain text after last decoration
        if pos < content.len() {
            let text = &content[pos..];
            segments.push(DecoratedTextSegment {
                text: text.to_string(),
                css_class: "",
                label: String::new(),
            });
        }

        segments
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::inline::InlineParser;

    // ── DecorationManager::build_decorations ──

    #[test]
    fn test_empty_content_no_decorations() {
        let parser = InlineParser::default();
        let parsed = parser.parse("");
        let decos = DecorationManager::build_decorations(&parsed);
        assert!(
            decos.is_empty(),
            "Empty content should produce no decorations"
        );
    }

    #[test]
    fn test_tag_decoration() {
        let parser = InlineParser::default();
        let parsed = parser.parse("#urgent");
        let decos = DecorationManager::build_decorations(&parsed);

        assert_eq!(decos.len(), 1, "#urgent should produce one Tag decoration");
        match &decos[0].kind {
            DecorationKind::Tag { tag_name } => {
                assert_eq!(tag_name, "urgent");
            }
            other => panic!("Expected Tag decoration, got {:?}", other),
        }
    }

    #[test]
    fn test_page_ref_decoration() {
        let parser = InlineParser::default();
        let parsed = parser.parse("[[Project Alpha]]");
        let decos = DecorationManager::build_decorations(&parsed);

        assert_eq!(
            decos.len(),
            1,
            "[[Project Alpha]] should produce one PageLink decoration"
        );
        match &decos[0].kind {
            DecorationKind::PageLink { page_name } => {
                assert_eq!(page_name, "Project Alpha");
            }
            other => panic!("Expected PageLink decoration, got {:?}", other),
        }
    }

    #[test]
    fn test_block_ref_decoration() {
        let parser = InlineParser::default();
        let parsed = parser.parse("((550e8400-e29b-41d4-a716-446655440000))");
        let decos = DecorationManager::build_decorations(&parsed);

        assert_eq!(decos.len(), 1);
        match &decos[0].kind {
            DecorationKind::BlockLink { block_uuid } => {
                assert_eq!(block_uuid, "550e8400-e29b-41d4-a716-446655440000");
            }
            other => panic!("Expected BlockLink decoration, got {:?}", other),
        }
    }

    #[test]
    fn test_property_decoration() {
        let parser = InlineParser::default();
        let parsed = parser.parse("status:: active");
        let decos = DecorationManager::build_decorations(&parsed);

        assert!(
            !decos.is_empty(),
            "status:: active should produce decorations"
        );
        // The property decoration should be for "status::"
        let has_property = decos
            .iter()
            .any(|d| matches!(&d.kind, DecorationKind::Property { key } if key == "status"));
        assert!(
            has_property,
            "Should have a Property decoration for 'status'"
        );
    }

    #[test]
    fn test_mixed_decorations() {
        let parser = InlineParser::default();
        let content = "Meeting #standup about [[Project]] status:: active";
        let parsed = parser.parse(content);
        let decos = DecorationManager::build_decorations(&parsed);

        let kinds: Vec<&str> = decos
            .iter()
            .map(|d| match &d.kind {
                DecorationKind::PageLink { .. } => "page",
                DecorationKind::BlockLink { .. } => "block",
                DecorationKind::Tag { .. } => "tag",
                DecorationKind::Property { .. } => "property",
                DecorationKind::SearchMatch { .. } => "search",
                DecorationKind::AutocompleteActive { .. } => "autocomplete",
            })
            .collect();

        assert!(
            kinds.contains(&"tag"),
            "Should have a tag decoration, got {:?}",
            kinds
        );
        assert!(
            kinds.contains(&"page"),
            "Should have a page decoration, got {:?}",
            kinds
        );
        assert!(
            kinds.contains(&"property"),
            "Should have a property decoration, got {:?}",
            kinds
        );
    }

    #[test]
    fn test_decoration_range_preserved() {
        let parser = InlineParser::default();
        let parsed = parser.parse("a[[bc]]d");
        let decos = DecorationManager::build_decorations(&parsed);

        let page_deco = decos
            .iter()
            .find(|d| matches!(&d.kind, DecorationKind::PageLink { .. }))
            .expect("Should have a PageLink decoration");

        assert_eq!(page_deco.range.start, 1);
        assert_eq!(page_deco.range.end, 7); // "[[bc]]" = 6 chars
    }

    // ── DecoratedTextSegments ──

    #[test]
    fn test_decorated_segments_plain_text() {
        let parser = InlineParser::default();
        let parsed = parser.parse("plain text");
        let segments = DecorationManager::decorated_segments("plain text", &parsed);

        assert_eq!(segments.len(), 1, "Plain text should be one segment");
        assert_eq!(segments[0].text, "plain text");
        assert_eq!(segments[0].css_class, "");
    }

    #[test]
    fn test_decorated_segments_with_tag() {
        let parser = InlineParser::default();
        let content = "text #urgent more";
        let parsed = parser.parse(content);
        let segments = DecorationManager::decorated_segments(content, &parsed);

        assert_eq!(segments.len(), 3, "text + tag + more = 3 segments");

        // Plain "text "
        assert_eq!(segments[0].text, "text ");
        assert_eq!(segments[0].css_class, "");

        // Tag "#urgent"
        assert_eq!(segments[1].text, "#urgent");
        assert_eq!(segments[1].css_class, "decoration-tag");
        assert_eq!(segments[1].label, "#urgent");

        // Plain " more"
        assert_eq!(segments[2].text, " more");
        assert_eq!(segments[2].css_class, "");
    }

    #[test]
    fn test_decorated_segments_with_page_ref() {
        let parser = InlineParser::default();
        let content = "see [[Page]]";
        let parsed = parser.parse(content);
        let segments = DecorationManager::decorated_segments(content, &parsed);

        assert_eq!(segments.len(), 2, "text + pageref = 2 segments");

        // Plain "see "
        assert_eq!(segments[0].text, "see ");
        assert_eq!(segments[0].css_class, "");

        // Page ref
        assert_eq!(segments[1].text, "[[Page]]");
        assert_eq!(segments[1].css_class, "decoration-page-ref");
        assert_eq!(segments[1].label, "Page");
    }

    #[test]
    fn test_decorated_segments_only_decorations() {
        let parser = InlineParser::default();
        let content = "[[A]] #b";
        let parsed = parser.parse(content);
        let segments = DecorationManager::decorated_segments(content, &parsed);

        // [[A]] and #b — but there might be text between them
        // The parser produces: PageRef [[A]] and Tag #b
        // with no text in between because ' ' is between them
        // Let me check actual parser output...

        // Actually: "[[A]] #b" — text at pos 0-4 is [[A]], space at 4-5 is text,
        // then #b at 5-7
        // So: PageRef[0-4], Text[4-5]=" ", Tag[5-7]
        assert_eq!(
            segments.len(),
            3,
            "Should get 3 segments: PageRef + space + Tag"
        );

        assert_eq!(segments[0].text, "[[A]]");
        assert_eq!(segments[0].css_class, "decoration-page-ref");

        assert_eq!(segments[1].text, " ");
        assert_eq!(segments[1].css_class, "");

        assert_eq!(segments[2].text, "#b");
        assert_eq!(segments[2].css_class, "decoration-tag");
    }

    #[test]
    fn test_decorated_segments_empty_content() {
        let segments = DecorationManager::decorated_segments("", &ParsedContent::default());
        assert!(
            segments.is_empty(),
            "Empty content should produce no segments"
        );
    }

    #[test]
    fn test_decorated_segments_property() {
        let parser = InlineParser::default();
        let content = "status:: done";
        let parsed = parser.parse(content);
        let segments = DecorationManager::decorated_segments(content, &parsed);

        // The property parsing should give us a Property segment and possibly trailing text
        let prop_seg = segments
            .iter()
            .find(|s| s.css_class == "decoration-property");
        assert!(
            prop_seg.is_some(),
            "Should have a property-decorated segment"
        );
        if let Some(seg) = prop_seg {
            assert_eq!(seg.label, "status");
        }
    }
}
