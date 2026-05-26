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

/// Rich render item for property-aware display in the non-editing view.
///
/// Each variant carries enough information to be rendered with proper
/// visual treatment (badges, pills, icons) without referencing parser
/// internals. The range is needed for in-place text replacement when
/// the user interacts with a property.
#[derive(Debug, Clone)]
pub enum RenderItem {
    /// Standard decorated text segment (existing behavior — tags, refs, plain text)
    DecoratedText {
        text: String,
        css_class: &'static str,
        label: String,
    },
    /// Status property `status:: TODO` / `status:: DOING` / `status:: DONE`
    PropertyStatus {
        full_text: String,
        key: String,
        value: String,
        valid: bool,
        range: Range,
    },
    /// Priority property `priority:: A` / `priority:: B` / `priority:: C`
    PropertyPriority {
        full_text: String,
        key: String,
        value: String,
        valid: bool,
        range: Range,
    },
    /// Date property `scheduled:: YYYY-MM-DD`
    PropertyScheduled {
        full_text: String,
        key: String,
        value: String,
        range: Range,
    },
    /// Deadline property `deadline:: YYYY-MM-DD`
    PropertyDeadline {
        full_text: String,
        key: String,
        value: String,
        range: Range,
    },
    /// Tags property `tags:: a, b, c`
    PropertyTags {
        full_text: String,
        key: String,
        value: String,
        tags: Vec<String>,
        range: Range,
    },
    /// Generic/unknown property key:: value
    PropertyGeneric {
        full_text: String,
        key: String,
        value: String,
        range: Range,
    },
}

/// Check if a property key is a known first-class property.
pub fn is_known_property(key: &str) -> bool {
    matches!(
        key.to_lowercase().as_str(),
        "status" | "priority" | "scheduled" | "deadline" | "tags"
    )
}

/// Get valid values for a known property key (status/priority).
pub fn known_property_values(key: &str) -> Option<&'static [&'static str]> {
    match key.to_lowercase().as_str() {
        "status" => Some(&["TODO", "DOING", "DONE"]),
        "priority" => Some(&["A", "B", "C"]),
        _ => None,
    }
}

impl DecorationManager {
    /// Build rich render items suitable for the non-editing property-aware view.
    ///
    /// This extends `decorated_segments` by adding semantic information
    /// for typed properties (status, priority, dates, tags) so the view
    /// can render them with proper visual treatment and interactivity.
    ///
    /// Non-property segments fall back to standard `RenderItem::DecoratedText`.
    pub fn build_render_items(content: &str, parsed: &ParsedContent) -> Vec<RenderItem> {
        if content.is_empty() {
            return vec![];
        }

        let decorations = Self::build_decorations(parsed);
        Self::apply_render_items(content, &decorations)
    }

    /// Apply decorations to text, producing rich render items.
    ///
    /// For property segments, checks the key against known types and
    /// produces typed variants. Unknown properties become `PropertyGeneric`.
    /// Non-property segments fall back to `DecoratedText`.
    fn apply_render_items(content: &str, decorations: &[Decoration]) -> Vec<RenderItem> {
        if content.is_empty() {
            return vec![];
        }

        let mut sorted: Vec<&Decoration> = decorations.iter().collect();
        sorted.sort_by_key(|d| d.range.start);

        // Build a map from range.start -> Segment for looking up property details
        let mut items = Vec::new();
        let mut pos = 0;

        for deco in sorted {
            // Emit plain text before this decoration
            if deco.range.start > pos {
                let text = &content[pos..deco.range.start];
                items.push(RenderItem::DecoratedText {
                    text: text.to_string(),
                    css_class: "",
                    label: String::new(),
                });
            }

            let text = &content[deco.range.start..deco.range.end];

            match &deco.kind {
                DecorationKind::Property { key } => {
                    // We need the value: it's after `key:: `
                    // Compute by stripping the key prefix and ":: " separator
                    let key_prefix = format!("{}::", key);
                    let value_part = if text.starts_with(&key_prefix) {
                        text[key_prefix.len()..].trim().to_string()
                    } else {
                        String::new()
                    };

                    let range = deco.range.clone();
                    let lower_key = key.to_lowercase();

                    match lower_key.as_str() {
                        "status" => {
                            let valid = matches!(
                                value_part.to_uppercase().as_str(),
                                "TODO" | "DOING" | "DONE"
                            );
                            items.push(RenderItem::PropertyStatus {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                valid,
                                range,
                            });
                        }
                        "priority" => {
                            let valid = matches!(value_part.to_uppercase().as_str(), "A" | "B" | "C");
                            items.push(RenderItem::PropertyPriority {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                valid,
                                range,
                            });
                        }
                        "scheduled" => {
                            items.push(RenderItem::PropertyScheduled {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                range,
                            });
                        }
                        "deadline" => {
                            items.push(RenderItem::PropertyDeadline {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                range,
                            });
                        }
                        "tags" => {
                            let tags: Vec<String> = value_part
                                .split(',')
                                .map(|t| t.trim().to_string())
                                .filter(|t| !t.is_empty())
                                .collect();
                            items.push(RenderItem::PropertyTags {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                tags,
                                range,
                            });
                        }
                        _ => {
                            items.push(RenderItem::PropertyGeneric {
                                full_text: text.to_string(),
                                key: key.clone(),
                                value: value_part.clone(),
                                range,
                            });
                        }
                    }
                }
                // Non-property decorations: render as styled text
                _ => {
                    let (css_class, label) = match &deco.kind {
                        DecorationKind::PageLink { page_name } => {
                            ("decoration-page-ref", page_name.clone())
                        }
                        DecorationKind::BlockLink { block_uuid } => {
                            ("decoration-block-ref", block_uuid.clone())
                        }
                        DecorationKind::Tag { tag_name } => {
                            ("decoration-tag", format!("#{}", tag_name))
                        }
                        DecorationKind::SearchMatch { query } => {
                            ("decoration-search", query.clone())
                        }
                        DecorationKind::AutocompleteActive { .. } => {
                            ("decoration-autocomplete", String::new())
                        }
                        DecorationKind::Property { .. } => unreachable!(), // handled above
                    };

                    items.push(RenderItem::DecoratedText {
                        text: text.to_string(),
                        css_class,
                        label,
                    });
                }
            }

            pos = deco.range.end;
        }

        // Emit remaining plain text after last decoration
        if pos < content.len() {
            let text = &content[pos..];
            items.push(RenderItem::DecoratedText {
                text: text.to_string(),
                css_class: "",
                label: String::new(),
            });
        }

        items
    }

    /// Compute the next status in the cycle: TODO → DOING → DONE → TODO
    pub fn cycle_status(current: &str) -> &'static str {
        match current.to_uppercase().as_str() {
            "TODO" => "DOING",
            "DOING" => "DONE",
            "DONE" => "TODO",
            _ => "TODO",
        }
    }

    /// Compute the next priority in the cycle: A → B → C → A
    pub fn cycle_priority(current: &str) -> &'static str {
        match current.to_uppercase().as_str() {
            "A" => "B",
            "B" => "C",
            "C" => "A",
            _ => "A",
        }
    }
}

/// Replace a property value in the content string given its range and key.
///
/// Given the original content, the range covering the full property
/// expression, the key, and the new value, produces the updated content.
pub fn replace_property_value(content: &str, range: &Range, key: &str, new_value: &str) -> String {
    format!(
        "{}{}:: {}{}",
        &content[..range.start],
        key,
        new_value,
        &content[range.end..]
    )
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

    // ── build_render_items tests ──

    #[test]
    fn test_render_items_plain_text() {
        let parser = InlineParser::default();
        let content = "hello world";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        assert_eq!(items.len(), 1, "Plain text should produce 1 item");
        match &items[0] {
            RenderItem::DecoratedText { text, css_class, .. } => {
                assert_eq!(text, "hello world");
                assert_eq!(*css_class, "");
            }
            other => panic!("Expected DecoratedText, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_status_property_valid() {
        let parser = InlineParser::default();
        let content = "status:: TODO";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        assert_eq!(items.len(), 1, "Should produce 1 render item");
        match &items[0] {
            RenderItem::PropertyStatus {
                key,
                value,
                valid,
                ..
            } => {
                assert_eq!(key, "status");
                assert_eq!(value, "TODO");
                assert!(valid, "TODO should be valid status");
            }
            other => panic!("Expected PropertyStatus, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_status_property_invalid() {
        let parser = InlineParser::default();
        let content = "status:: INVALID";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyStatus { valid, .. } => {
                assert!(!valid, "INVALID should not be valid status");
            }
            other => panic!("Expected PropertyStatus, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_priority_property() {
        let parser = InlineParser::default();
        let content = "priority:: A";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyPriority {
                key,
                value,
                valid,
                ..
            } => {
                assert_eq!(key, "priority");
                assert_eq!(value, "A");
                assert!(valid, "A should be valid priority");
            }
            other => panic!("Expected PropertyPriority, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_priority_invalid() {
        let parser = InlineParser::default();
        let content = "priority:: X";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyPriority { valid, .. } => {
                assert!(!valid, "X should not be valid priority");
            }
            other => panic!("Expected PropertyPriority, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_scheduled_property() {
        let parser = InlineParser::default();
        let content = "scheduled:: 2026-05-28";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyScheduled { key, value, .. } => {
                assert_eq!(key, "scheduled");
                assert_eq!(value, "2026-05-28");
            }
            other => panic!("Expected PropertyScheduled, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_deadline_property() {
        let parser = InlineParser::default();
        let content = "deadline:: 2026-05-30";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyDeadline { key, value, .. } => {
                assert_eq!(key, "deadline");
                assert_eq!(value, "2026-05-30");
            }
            other => panic!("Expected PropertyDeadline, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_tags_property() {
        let parser = InlineParser::default();
        let content = "tags:: a, b, c";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyTags {
                key,
                value,
                tags,
                ..
            } => {
                assert_eq!(key, "tags");
                assert_eq!(value, "a, b, c");
                assert_eq!(tags.len(), 3);
                assert!(tags.contains(&"a".to_string()));
                assert!(tags.contains(&"b".to_string()));
                assert!(tags.contains(&"c".to_string()));
            }
            other => panic!("Expected PropertyTags, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_generic_property() {
        let parser = InlineParser::default();
        let content = "custom:: some_value";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        match &items[0] {
            RenderItem::PropertyGeneric { key, value, .. } => {
                assert_eq!(key, "custom");
                assert_eq!(value, "some_value");
            }
            other => panic!("Expected PropertyGeneric, got {:?}", other),
        }
    }

    #[test]
    fn test_render_items_mixed_content() {
        let parser = InlineParser::default();
        let content = "Meeting #team about [[Project]] status:: DOING priority:: A";
        let parsed = parser.parse(content);
        let items = DecorationManager::build_render_items(content, &parsed);

        // Should have: text, tag, text, pageref, text, status, text, priority
        assert!(
            items.len() >= 5,
            "Mixed content should produce multiple items, got {}",
            items.len()
        );

        let status_found = items.iter().any(|item| matches!(item, RenderItem::PropertyStatus { .. }));
        let priority_found = items.iter().any(|item| matches!(item, RenderItem::PropertyPriority { .. }));
        let _tag_found = items.iter().any(|item| matches!(item, RenderItem::DecoratedText { css_class: "decoration-tag", .. }));

        assert!(status_found, "Should find PropertyStatus");
        assert!(priority_found, "Should find PropertyPriority");
    }

    #[test]
    fn test_render_items_empty_content() {
        let items = DecorationManager::build_render_items("", &ParsedContent::default());
        assert!(
            items.is_empty(),
            "Empty content should produce no items"
        );
    }

    // ── cycle_status tests ──

    #[test]
    fn test_cycle_status_todo_to_doing() {
        assert_eq!(DecorationManager::cycle_status("TODO"), "DOING");
    }

    #[test]
    fn test_cycle_status_doing_to_done() {
        assert_eq!(DecorationManager::cycle_status("DOING"), "DONE");
    }

    #[test]
    fn test_cycle_status_done_to_todo() {
        assert_eq!(DecorationManager::cycle_status("DONE"), "TODO");
    }

    #[test]
    fn test_cycle_status_case_insensitive() {
        assert_eq!(DecorationManager::cycle_status("todo"), "DOING");
        assert_eq!(DecorationManager::cycle_status("doing"), "DONE");
        assert_eq!(DecorationManager::cycle_status("done"), "TODO");
    }

    #[test]
    fn test_cycle_status_unknown_defaults_to_todo() {
        assert_eq!(DecorationManager::cycle_status("invalid"), "TODO");
    }

    // ── cycle_priority tests ──

    #[test]
    fn test_cycle_priority_a_to_b() {
        assert_eq!(DecorationManager::cycle_priority("A"), "B");
    }

    #[test]
    fn test_cycle_priority_b_to_c() {
        assert_eq!(DecorationManager::cycle_priority("B"), "C");
    }

    #[test]
    fn test_cycle_priority_c_to_a() {
        assert_eq!(DecorationManager::cycle_priority("C"), "A");
    }

    #[test]
    fn test_cycle_priority_case_insensitive() {
        assert_eq!(DecorationManager::cycle_priority("a"), "B");
        assert_eq!(DecorationManager::cycle_priority("b"), "C");
        assert_eq!(DecorationManager::cycle_priority("c"), "A");
    }

    #[test]
    fn test_cycle_priority_unknown_defaults_to_a() {
        assert_eq!(DecorationManager::cycle_priority("X"), "A");
    }

    // ── replace_property_value tests ──

    #[test]
    fn test_replace_property_value_status() {
        let content = "status:: TODO";
        let range = Range::new(0, 13); // covers all 13 chars of "status:: TODO"
        let result = replace_property_value(content, &range, "status", "DOING");
        assert_eq!(result, "status:: DOING");
    }

    #[test]
    fn test_replace_property_value_with_context() {
        let content = "Meeting status:: TODO priority:: A";
        let range = Range::new(8, 21); // covers "status:: TODO" = 13 chars
        let result = replace_property_value(content, &range, "status", "DOING");
        assert_eq!(result, "Meeting status:: DOING priority:: A");
    }

    #[test]
    fn test_replace_property_value_empty_string() {
        let content = "";
        let range = Range::new(0, 0);
        let result = replace_property_value(content, &range, "key", "value");
        assert_eq!(result, "key:: value");
    }
}
