//! Incremental inline parser for semantic syntax
//!
//! Parses `[[Page]]`, `((Block))`, `#tag`, and `property:: value` syntax
//! from block content. Provides range information for decorations.

use std::collections::HashMap;

/// Find the end boundary of a property value.
/// The value extends until:
/// - A following property pattern (`word:: `) is found, OR
/// - End of content.
fn find_property_value_boundary(content: &str, start: usize) -> usize {
    let bytes = content.as_bytes();
    let mut i = start;
    while i + 2 < bytes.len() {
        if bytes[i] == b':' && bytes[i + 1] == b':' {
            // Check if the char before :: is part of a valid property key
            if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
                // Walk backward to find key start
                let mut key_start = i - 1;
                while key_start > start && bytes[key_start - 1].is_ascii_alphanumeric() {
                    key_start -= 1;
                }
                // Check key is preceded by a space or is at content start
                let precedes_property =
                    key_start == start || (key_start > 0 && bytes[key_start - 1] == b' ');
                if precedes_property {
                    // Trim trailing space before the next property
                    let mut boundary = key_start;
                    while boundary > start && bytes[boundary - 1] == b' ' {
                        boundary -= 1;
                    }
                    return boundary;
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    content.len()
}

/// Character range in the source text
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Range {
    pub start: usize,
    pub end: usize,
}

impl Range {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// A parsed segment of inline content
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Segment {
    /// Plain text content
    Text { content: String, range: Range },
    /// Page reference `[[Page Name]]` or `[[Page Name|alias]]`
    ///
    /// `page_name` is the target used for resolution and the `href`.
    /// `alias` is the optional display text after the first `|` — the
    /// page itself is identified by `page_name` only.
    PageRef {
        page_name: String,
        alias: Option<String>,
        raw: String,
        range: Range,
    },
    /// Block reference `((UUID))`
    BlockRef {
        block_uuid: String,
        raw: String,
        range: Range,
    },
    /// Tag `#tagname`
    Tag {
        name: String,
        raw: String,
        range: Range,
    },
    /// Property `property:: value`
    Property {
        key: String,
        value: String,
        raw: String,
        range: Range,
    },
    /// Bold text `**text**`
    Bold {
        content: String,
        raw: String,
        range: Range,
    },
    /// Italic text `*text*`
    Italic {
        content: String,
        raw: String,
        range: Range,
    },
    /// Inline code `` `code` ``
    Code {
        content: String,
        raw: String,
        range: Range,
    },
    /// Link `[text](url)`
    Link {
        text: String,
        url: String,
        raw: String,
        range: Range,
    },
    /// Bold+Italic `***text***`
    BoldItalic {
        content: String,
        raw: String,
        range: Range,
    },
    /// Strikethrough `~~text~~`
    Strikethrough {
        content: String,
        raw: String,
        range: Range,
    },
    /// Highlight `^^text^^`
    Highlight {
        content: String,
        raw: String,
        range: Range,
    },
    /// Header `# ` `## ` `### ` ... at start of block
    Header {
        level: u8,
        content: String,
        raw: String,
        range: Range,
    },
}

/// Parsed content with all segments and raw text
#[derive(Debug, Clone, Default)]
pub struct ParsedContent {
    pub raw_text: String,
    pub segments: Vec<Segment>,
}

/// Normalized content ready for domain model
#[derive(Debug, Clone, Default)]
pub struct NormalizedContent {
    pub page_refs: Vec<String>,
    pub block_refs: Vec<String>,
    pub tags: Vec<String>,
    pub properties: Vec<(String, String)>,
}

impl NormalizedContent {
    pub fn properties_map(&self) -> HashMap<String, String> {
        self.properties.iter().cloned().collect()
    }
}

/// Aggregated semantic data safe for UI consumption.
/// This is a derived view of the parsed content — no mutation, no I/O.
#[derive(Debug, Clone, Default)]
pub struct SemanticData {
    /// All tags found (from `#tag` and `tags::`)
    pub tags: Vec<String>,
    /// All page references found
    pub page_refs: Vec<String>,
    /// All block references found
    pub block_refs: Vec<String>,
    /// Property count
    pub properties_count: usize,
    /// Map of properties
    pub properties: HashMap<String, String>,
}

impl ParsedContent {
    /// Extract aggregated semantic data for UI consumption.
    /// This method is safe, pure, and has no side effects.
    pub fn semantic_data(&self) -> SemanticData {
        let mut tags = Vec::new();
        let mut page_refs = Vec::new();
        let mut block_refs = Vec::new();
        let mut properties = Vec::new();

        for segment in &self.segments {
            match segment {
                Segment::Tag { name, .. } => {
                    let lower = name.to_lowercase();
                    if !tags.contains(&lower) {
                        tags.push(lower);
                    }
                }
                Segment::PageRef { page_name, .. } => {
                    page_refs.push(page_name.clone());
                }
                Segment::BlockRef { block_uuid, .. } => {
                    block_refs.push(block_uuid.clone());
                }
                Segment::Property { key, value, .. } => {
                    if key == "tags" && !value.is_empty() {
                        for tag in value.split(',') {
                            let t = tag.trim().to_lowercase();
                            if !t.is_empty() && !tags.contains(&t) {
                                tags.push(t);
                            }
                        }
                    } else {
                        properties.push((key.clone(), value.clone()));
                    }
                }
                Segment::Text { .. } => {}
                Segment::Bold { .. } => {}
                Segment::BoldItalic { .. } => {}
                Segment::Italic { .. } => {}
                Segment::Code { .. } => {}
                Segment::Link { .. } => {}
                Segment::Strikethrough { .. } => {}
                Segment::Highlight { .. } => {}
                Segment::Header { .. } => {}
            }
        }

        let properties_count = properties.len();
        let prop_map: HashMap<String, String> = properties.into_iter().collect();

        SemanticData {
            tags,
            page_refs,
            block_refs,
            properties_count,
            properties: prop_map,
        }
    }
}

/// Inline semantic parser
///
/// Parses block content and extracts:
/// - Page references `[[Page Name]]`
/// - Block references `((UUID))`
/// - Tags `#tagname`
/// - Properties `property:: value`
#[derive(Debug, Clone, Default)]
pub struct InlineParser {
    #[allow(dead_code)]
    // Parser state for incremental parsing (future use)
    state: (),
}

impl InlineParser {
    pub fn new() -> Self {
        Self { state: () }
    }
    /// Parse content from scratch
    pub fn parse(&self, content: &str) -> ParsedContent {
        let mut segments = Vec::new();
        let mut pos = 0;
        let bytes = content.as_bytes();

        while pos < content.len() {
            // Try to match each pattern at current position
            if let Some((segment, len)) = self.try_parse_page_ref(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_block_ref(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_tag(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_property(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_bold_italic(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_bold(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_italic(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_code(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_link(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_strikethrough(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_highlight(content, pos) {
                segments.push(segment);
                pos += len;
            } else if let Some((segment, len)) = self.try_parse_header(content, pos) {
                segments.push(segment);
                pos += len;
            } else {
                // Plain text - find next special character
                let start = pos;
                let mut end = pos;
                while end < content.len() {
                    // Check if we're at a special pattern start
                    let remaining = &content[end..];
                    if remaining.starts_with("[[")
                        || remaining.starts_with("((")
                        || remaining.starts_with("***")
                        || remaining.starts_with('#')
                        || remaining.starts_with("::")
                        || remaining.starts_with("**")
                        || remaining.starts_with("~~")
                        || remaining.starts_with("^^")
                        || remaining.starts_with('*')
                        || remaining.starts_with('`')
                        || remaining.starts_with('[')
                        || remaining.starts_with("property::")
                    {
                        break;
                    }
                    // Advance by the byte length of the next char, so we
                    // never land in the middle of a multi-byte UTF-8
                    // sequence (which would panic on the next `&content[end..]`
                    // slice).
                    end += remaining.chars().next().map_or(1, |c| c.len_utf8());
                }
                if end == pos {
                    // We're at a :: that didn't form a valid property — consume as pair
                    if pos + 1 < content.len() && bytes[pos] == b':' && bytes[pos + 1] == b':' {
                        segments.push(Segment::Text {
                            content: "::".to_string(),
                            range: Range::new(pos, pos + 2),
                        });
                        pos += 2;
                        continue;
                    }
                    // Check for block ref brackets that weren't matched - skip as unit
                    if pos + 1 < content.len() && bytes[pos] == b'(' && bytes[pos + 1] == b'(' {
                        // Unmatched (( - skip both characters
                        end = pos + 2;
                    } else if pos + 1 < content.len()
                        && bytes[pos] == b')'
                        && bytes[pos + 1] == b')'
                    {
                        // Unmatched )) - skip both characters
                        end = pos + 2;
                    } else {
                        end = pos + 1;
                    }
                }
                let text = &content[start..end];
                if !text.is_empty() {
                    segments.push(Segment::Text {
                        content: text.to_string(),
                        range: Range::new(start, end),
                    });
                }
                pos = end;
            }
        }

        ParsedContent {
            raw_text: content.to_string(),
            segments,
        }
    }

    /// Try to parse a page reference at the given position
    fn try_parse_page_ref(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("[[") {
            return None;
        }

        // Find the closing ]]
        if let Some(end_pos) = remaining[2..].find("]]") {
            let inner = &remaining[2..end_pos + 2];
            let raw = format!("[[{}]]", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);

            // Split on the FIRST `|` to extract an optional alias.
            // `[[` shouldn't be nested in our case, so a plain `find('|')`
            // is sufficient. `[[Page|]]` (empty alias) is treated as no
            // alias — there's no meaningful display text.
            let (page_name, alias) = match inner.find('|') {
                Some(idx) => {
                    let name = inner[..idx].to_string();
                    let rest = inner[idx + 1..].to_string();
                    let trimmed = rest.trim();
                    if trimmed.is_empty() {
                        (name, None)
                    } else {
                        (name, Some(trimmed.to_string()))
                    }
                }
                None => (inner.to_string(), None),
            };

            return Some((
                Segment::PageRef {
                    page_name,
                    alias,
                    raw,
                    range,
                },
                raw_len,
            ));
        }

        None
    }

    /// Try to parse a block reference at the given position
    fn try_parse_block_ref(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("((") {
            return None;
        }

        // Find the closing ))
        if let Some(end_pos) = remaining[2..].find("))") {
            let inner = &remaining[2..end_pos + 2];
            let raw = format!("(({}))", inner);

            // Validate UUID format
            let uuid_part = inner.trim();
            if Self::is_valid_uuid(uuid_part) {
                let raw_len = raw.len();
                let range = Range::new(pos, pos + raw_len);
                return Some((
                    Segment::BlockRef {
                        block_uuid: uuid_part.to_string(),
                        raw,
                        range,
                    },
                    raw_len,
                ));
            }
        }

        None
    }

    /// Check if a string is a valid UUID format
    fn is_valid_uuid(s: &str) -> bool {
        // UUID format: 8-4-4-4-12 hex characters
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return false;
        }
        if parts[0].len() != 8
            || parts[1].len() != 4
            || parts[2].len() != 4
            || parts[3].len() != 4
            || parts[4].len() != 12
        {
            return false;
        }
        parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
    }

    /// Try to parse a tag at the given position
    fn try_parse_tag(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with('#') {
            return None;
        }

        // Tag: # followed by alphanumeric, underscores, hyphens
        let after_hash = &remaining[1..];
        let tag_len = after_hash
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
            .unwrap_or(after_hash.len());

        if tag_len == 0 {
            return None;
        }

        let tag_name = &after_hash[..tag_len];
        let raw = format!("#{}", tag_name);
        let raw_len = raw.len();
        let range = Range::new(pos, pos + raw_len);

        // Normalize tag name to lowercase
        Some((
            Segment::Tag {
                name: tag_name.to_lowercase(),
                raw,
                range,
            },
            raw_len,
        ))
    }

    /// Try to parse a property at the given position
    fn try_parse_property(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];

        // Look for property:: pattern
        let property_prefix = "property::";
        if remaining.starts_with(property_prefix) {
            let value_start = pos + property_prefix.len();
            let value = if value_start < content.len() {
                let value_end = find_property_value_boundary(content, value_start);
                content[value_start..value_end].trim().to_string()
            } else {
                String::new()
            };

            let raw_end = if value.is_empty() {
                value_start
            } else {
                value_start + value.len()
            };

            let range = Range::new(pos, raw_end);
            return Some((
                Segment::Property {
                    key: "property".to_string(),
                    value,
                    raw: "property::".to_string(),
                    range,
                },
                raw_end - pos,
            ));
        }

        // Check if we're right after a :: (meaning another property starts here)
        if pos > 0 && pos < content.len() {
            let prev_char = content.as_bytes()[pos - 1];
            let curr_char = content.as_bytes()[pos];
            // If previous char is : and current is not :, we're right after ::
            if prev_char == b':' && curr_char != b':' {
                return None; // Skip - this is the second : in ::
            }
        }

        // Look for :: at current position or immediately after current word
        // We need to find if there's a word followed by :: at this position
        let before = &content[..pos];

        // Find where the current word starts (go back from pos to find start of word).
        // We must advance by the full UTF-8 length of the whitespace char,
        // not by 1 — otherwise multi-byte whitespace (e.g. U+0085 "Next Line",
        // 2 bytes) would land `word_start` inside the char and panic the
        // `&content[word_start..]` slice below.
        let word_start = before
            .char_indices()
            .filter(|(_, c)| c.is_whitespace())
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        // Guard: the property key must start at or after the current
        // parse position. If `word_start < pos`, the key begins in a
        // region the main loop has already consumed (e.g. as plain text
        // or another segment). Producing a segment whose range starts
        // before `pos` would overlap previously-emitted segments and
        // violate the proptest invariants `segments_dont_overlap` and
        // `total_consumed_at_most_input`. Bail out instead.
        if word_start < pos {
            return None;
        }

        // Check if there's a :: immediately after the word
        let after_word = &content[word_start..];
        // Check if after_word starts with "{key}::" pattern
        let key_and_prefix = after_word.split("::").next().unwrap_or("");
        let key = key_and_prefix.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return None;
        }
        // Verify that :: actually follows this key
        if !after_word.starts_with(&format!("{}::", key)) {
            return None;
        }

        // Now parse the value after ::
        // colon_pos is the position of the first ':' after the key
        let colon_pos = word_start + key.len();
        let after_colons = &content[colon_pos + 2..]; // Skip the "::"

        // Skip leading whitespace after ::
        let trimmed = after_colons.trim_start();
        let skipped = after_colons.len() - trimmed.len();

        // Calculate actual start of the value content
        // after_colons starts at colon_pos + 2
        // trimmed starts after 'skipped' characters
        // value starts at (colon_pos + 2) + skipped
        let value_actual_start = colon_pos + 2 + skipped;

        // Value extends until next property or end of content
        let value_end = find_property_value_boundary(content, value_actual_start);
        let value = content[value_actual_start..value_end]
            .trim_end()
            .to_string();

        let raw_end = value_end;

        let range = Range::new(word_start, raw_end);

        Some((
            Segment::Property {
                key: key.to_string(),
                value,
                raw: format!("{}::", key),
                range,
            },
            raw_end - pos,
        ))
    }

    /// Try to parse bold+italic `***text***`
    fn try_parse_bold_italic(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("***") {
            return None;
        }
        // Find the closing ***
        if let Some(end_pos) = remaining[3..].find("***") {
            let inner = &remaining[3..end_pos + 3];
            let raw = format!("***{}***", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);
            return Some((
                Segment::BoldItalic {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }
        None
    }

    /// Try to parse bold text `**text**`
    fn try_parse_bold(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("**") {
            return None;
        }

        // Find the closing **
        if let Some(end_pos) = remaining[2..].find("**") {
            let inner = &remaining[2..end_pos + 2];
            let raw = format!("**{}**", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);

            return Some((
                Segment::Bold {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }

        None
    }

    /// Try to parse italic text `*text*`
    fn try_parse_italic(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with('*') {
            return None;
        }

        // Make sure it's not ** (bold)
        if remaining.starts_with("**") {
            return None;
        }

        // Find the closing *
        if let Some(end_pos) = remaining[1..].find('*') {
            let inner = &remaining[1..end_pos + 1];
            // Don't parse if contains newline (likely not intended emphasis)
            if inner.contains('\n') {
                return None;
            }
            let raw = format!("*{}*", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);

            return Some((
                Segment::Italic {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }

        None
    }

    /// Try to parse inline code `` `code` ``
    fn try_parse_code(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with('`') {
            return None;
        }

        // Find the closing `
        if let Some(end_pos) = remaining[1..].find('`') {
            let inner = &remaining[1..end_pos + 1];
            let raw = format!("`{}`", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);

            return Some((
                Segment::Code {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }

        None
    }

    /// Try to parse a markdown link `[text](url)`
    fn try_parse_link(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with('[') {
            return None;
        }

        // Find the closing ] - bracket_end is relative to remaining[1..]
        let bracket_end = remaining[1..].find(']')?;
        // Text is from after [ to the ] - need to add 1 because bracket_end is relative to [1..]
        let text = &remaining[1..bracket_end + 1];

        // Check for ( after ]
        let after_bracket_pos = bracket_end + 2; // Skip past [text]
        if after_bracket_pos >= remaining.len() {
            return None;
        }
        let after_bracket = &remaining[after_bracket_pos..];
        if !after_bracket.starts_with('(') {
            return None;
        }

        // Find the closing )
        let url_end = after_bracket[1..].find(')')?;
        let url = &after_bracket[1..url_end + 1];

        let raw_len = after_bracket_pos + 1 + url_end + 1; // Total length of [text](url)
        let range = Range::new(pos, pos + raw_len);

        Some((
            Segment::Link {
                text: text.to_string(),
                url: url.to_string(),
                raw: content[pos..pos + raw_len].to_string(),
                range,
            },
            raw_len,
        ))
    }

    /// Try to parse strikethrough `~~text~~`
    fn try_parse_strikethrough(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("~~") {
            return None;
        }
        if let Some(end_pos) = remaining[2..].find("~~") {
            let inner = &remaining[2..end_pos + 2];
            let raw = format!("~~{}~~", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);
            return Some((
                Segment::Strikethrough {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }
        None
    }

    /// Try to parse highlight `^^text^^`
    fn try_parse_highlight(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        let remaining = &content[pos..];
        if !remaining.starts_with("^^") {
            return None;
        }
        if let Some(end_pos) = remaining[2..].find("^^") {
            let inner = &remaining[2..end_pos + 2];
            let raw = format!("^^{}^^", inner);
            let raw_len = raw.len();
            let range = Range::new(pos, pos + raw_len);
            return Some((
                Segment::Highlight {
                    content: inner.to_string(),
                    raw,
                    range,
                },
                raw_len,
            ));
        }
        None
    }

    /// Try to parse header `# ` through `###### ` at start of block
    fn try_parse_header(&self, content: &str, pos: usize) -> Option<(Segment, usize)> {
        // Only match at position 0 (start of block)
        if pos != 0 {
            return None;
        }
        let remaining = &content[pos..];
        let level = remaining.chars().take_while(|c| *c == '#').count();
        if level == 0 || level > 6 {
            return None;
        }
        // Must be followed by a space
        if remaining.len() <= level || remaining.as_bytes().get(level) != Some(&b' ') {
            return None;
        }
        // Header content is everything after "# " until end of block
        let inner = remaining[level + 1..].trim();
        let raw_len = remaining.len(); // Header consumes the whole block
        let raw = remaining.to_string();
        let range = Range::new(pos, pos + raw_len);
        Some((
            Segment::Header {
                level: level as u8,
                content: inner.to_string(),
                raw,
                range,
            },
            raw_len,
        ))
    }

    /// Normalize parsed content to domain entities
    pub fn normalize(&self, parsed: &ParsedContent) -> NormalizedContent {
        let mut page_refs = Vec::new();
        let mut block_refs = Vec::new();
        let mut tags = Vec::new();
        let mut properties = Vec::new();

        for segment in &parsed.segments {
            match segment {
                Segment::PageRef { page_name, .. } => {
                    page_refs.push(page_name.clone());
                }
                Segment::BlockRef { block_uuid, .. } => {
                    block_refs.push(block_uuid.clone());
                }
                Segment::Tag { name, .. } => {
                    tags.push(name.clone());
                }
                Segment::Property { key, value, .. } => {
                    // Normalize #tag to tags property
                    if key == "tags" && !value.is_empty() {
                        // Split by comma and add individual tags
                        for tag in value.split(',') {
                            let tag = tag.trim().to_lowercase();
                            if !tag.is_empty() {
                                tags.push(tag);
                            }
                        }
                    } else {
                        properties.push((key.clone(), value.clone()));
                    }
                }
                Segment::Text { .. } => {}
                Segment::Bold { .. } => {}
                Segment::BoldItalic { .. } => {}
                Segment::Italic { .. } => {}
                Segment::Code { .. } => {}
                Segment::Link { .. } => {}
                Segment::Strikethrough { .. } => {}
                Segment::Highlight { .. } => {}
                Segment::Header { .. } => {}
            }
        }

        NormalizedContent {
            page_refs,
            block_refs,
            tags,
            properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_uuid() {
        assert!(InlineParser::is_valid_uuid(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(!InlineParser::is_valid_uuid("invalid"));
        assert!(!InlineParser::is_valid_uuid("550e8400-e29b-41d4-a716")); // too short
        assert!(!InlineParser::is_valid_uuid(
            "550e8400-e29b-41d4-a716-446655440000-extra"
        )); // too long
    }

    #[test]
    fn test_parse_page_ref_at_start() {
        let parser = InlineParser::default();
        let result = parser.parse("[[Page]] text");
        assert_eq!(result.segments.len(), 2);
        match &result.segments[0] {
            Segment::PageRef { page_name, .. } => assert_eq!(page_name, "Page"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_page_ref_at_end() {
        let parser = InlineParser::default();
        let result = parser.parse("text [[Page]]");
        assert_eq!(result.segments.len(), 2);
        match &result.segments[1] {
            Segment::PageRef { page_name, .. } => assert_eq!(page_name, "Page"),
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_parse_tag_normalization() {
        let parser = InlineParser::default();
        let result = parser.parse("#UPPERCASE");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::Tag { name, .. } => assert_eq!(name, "uppercase"),
            _ => panic!("Expected Tag"),
        }
    }

    #[test]
    fn test_parse_multiple_properties() {
        let parser = InlineParser::default();
        let result = parser.parse("status:: todo priority:: high");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(prop_map.get("status"), Some(&"todo".to_string()));
        assert_eq!(prop_map.get("priority"), Some(&"high".to_string()));
    }

    #[test]
    fn test_parse_mixed_inline_syntax() {
        let parser = InlineParser::default();
        // "Meeting with #team about [[Project Plan]] status:: active"
        let result = parser.parse("Meeting with #team about [[Project Plan]] status:: active");

        // We should have at least 5 segments
        assert!(
            result.segments.len() >= 5,
            "Expected at least 5 segments, got {}",
            result.segments.len()
        );

        // First segment should be text
        match &result.segments[0] {
            Segment::Text { content, .. } => {
                assert!(content.starts_with("Meeting"));
            }
            _ => panic!("Expected Text as first segment"),
        }

        // Should have a tag
        let has_tag = result
            .segments
            .iter()
            .any(|s| matches!(s, Segment::Tag { .. }));
        assert!(has_tag, "Expected at least one Tag segment");

        // Should have a page ref
        let has_page_ref = result
            .segments
            .iter()
            .any(|s| matches!(s, Segment::PageRef { .. }));
        assert!(has_page_ref, "Expected at least one PageRef segment");
    }

    #[test]
    fn test_normalize_tags_to_properties() {
        let parser = InlineParser::default();
        // Tags should be normalized to the tags property
        let result = parser.parse("#tag1 #tag2 #tag3");
        let normalized = parser.normalize(&result);

        assert_eq!(normalized.tags.len(), 3);
        assert!(normalized.tags.contains(&"tag1".to_string()));
        assert!(normalized.tags.contains(&"tag2".to_string()));
        assert!(normalized.tags.contains(&"tag3".to_string()));
    }

    #[test]
    fn test_normalize_explicit_tags_property() {
        let parser = InlineParser::default();
        // tags:: should also add to tags (comma-separated values)
        let result = parser.parse("tags:: important, urgent");
        let normalized = parser.normalize(&result);

        // Both tags should be extracted
        assert!(normalized.tags.contains(&"important".to_string()));
        // "urgent" is also extracted — the parser captures the full value
        // until the next `key::` boundary or end of content.
        assert!(normalized.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_parse_block_ref_uuid_validation() {
        let parser = InlineParser::default();

        // Valid UUID format
        let result = parser.parse("((550e8400-e29b-41d4-a716-446655440000))");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::BlockRef { block_uuid, .. } => {
                assert_eq!(block_uuid, "550e8400-e29b-41d4-a716-446655440000");
            }
            _ => panic!("Expected BlockRef"),
        }

        // Invalid UUID (not UUID format) - now parsed as text with (( skipped
        // Produces: Text("((") + Text("invalid-uuid))")
        let result2 = parser.parse("((invalid-uuid))");
        assert_eq!(result2.segments.len(), 2);
        match &result2.segments[0] {
            Segment::Text { content, .. } => {
                assert_eq!(content, "((");
            }
            _ => panic!("Expected Text for first segment"),
        }
        match &result2.segments[1] {
            Segment::Text { .. } => {}
            _ => panic!("Expected Text for invalid block ref"),
        }
    }

    #[test]
    fn test_range_information_preserved() {
        let parser = InlineParser::default();
        let result = parser.parse("a[[bc]]d");

        assert_eq!(result.segments.len(), 3);

        // First text "a" at [0, 1)
        match &result.segments[0] {
            Segment::Text { content, range } => {
                assert_eq!(content, "a");
                assert_eq!(range.start, 0);
                assert_eq!(range.end, 1);
            }
            _ => panic!("Expected Text"),
        }

        // PageRef "bc" at [1, 7) - [[bc]] spans positions 1-6
        match &result.segments[1] {
            Segment::PageRef {
                page_name, range, ..
            } => {
                assert_eq!(page_name, "bc");
                assert_eq!(range.start, 1);
                assert_eq!(range.end, 7); // [[bc]] is 6 chars: [, [, b, c, ], ]
            }
            _ => panic!("Expected PageRef"),
        }

        // Last text "d" at [7, 8)
        match &result.segments[2] {
            Segment::Text { content, range } => {
                assert_eq!(content, "d");
                assert_eq!(range.start, 7);
                assert_eq!(range.end, 8);
            }
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_page_ref_normalization() {
        let parser = InlineParser::default();
        let result = parser.parse("See [[My Page]] for details");
        let normalized = parser.normalize(&result);

        assert_eq!(normalized.page_refs.len(), 1);
        assert_eq!(normalized.page_refs[0], "My Page");
    }

    // ── G1: [[Page|alias]] support ────────────────────────────────────
    //
    // The wikilink syntax `[[Page|alias]]` should split on the first
    // `|`, using the part before for the page lookup and the part
    // after for display. The full raw text is preserved for round-trip.

    #[test]
    fn test_page_ref_with_alias_splits_on_pipe() {
        let parser = InlineParser::default();
        let result = parser.parse("[[Real Page|alias]]");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::PageRef {
                page_name,
                alias,
                raw,
                ..
            } => {
                assert_eq!(page_name, "Real Page");
                assert_eq!(alias.as_deref(), Some("alias"));
                assert_eq!(raw, "[[Real Page|alias]]");
            }
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_page_ref_alias_can_contain_spaces() {
        let parser = InlineParser::default();
        let result = parser.parse("[[Page|multi word alias]]");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::PageRef {
                page_name, alias, ..
            } => {
                assert_eq!(page_name, "Page");
                assert_eq!(alias.as_deref(), Some("multi word alias"));
            }
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_page_ref_without_alias_keeps_no_alias() {
        let parser = InlineParser::default();
        let result = parser.parse("[[Page]]");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::PageRef {
                page_name, alias, ..
            } => {
                assert_eq!(page_name, "Page");
                assert!(alias.is_none(), "no pipe → alias must be None");
            }
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_page_ref_empty_alias_is_treated_as_no_alias() {
        let parser = InlineParser::default();
        let result = parser.parse("[[Page|]]");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::PageRef {
                page_name, alias, ..
            } => {
                assert_eq!(page_name, "Page");
                assert!(alias.is_none(), "[[Page|]] → alias must be None");
            }
            _ => panic!("Expected PageRef"),
        }
    }

    #[test]
    fn test_page_ref_alias_normalization_uses_page_name() {
        // Normalize must yield the page name (not the alias) so the
        // ref service can resolve the correct target.
        let parser = InlineParser::default();
        let result = parser.parse("See [[My Page|My Display]] for details");
        let normalized = parser.normalize(&result);

        assert_eq!(normalized.page_refs.len(), 1);
        assert_eq!(
            normalized.page_refs[0], "My Page",
            "normalized page_refs must use the page name, not the alias"
        );
    }

    #[test]
    fn test_block_ref_normalization() {
        let parser = InlineParser::default();
        let result = parser.parse("Link to ((550e8400-e29b-41d4-a716-446655440000)) here");
        let normalized = parser.normalize(&result);

        assert_eq!(normalized.block_refs.len(), 1);
        assert_eq!(
            normalized.block_refs[0],
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_parse_property_with_colon_in_value() {
        let parser = InlineParser::default();
        // Value containing a URL with colons
        let result = parser.parse("link:: https://example.com");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();

        assert_eq!(
            prop_map.get("link"),
            Some(&"https://example.com".to_string())
        );
    }

    #[test]
    fn test_parse_empty_property_value() {
        let parser = InlineParser::default();
        let result = parser.parse("property::");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::Property { key, value, .. } => {
                assert_eq!(key, "property");
                assert_eq!(value, "");
            }
            _ => panic!("Expected Property"),
        }
    }

    // ── BUG 1: property parsing in mixed text like "text status:: active" ──

    #[test]
    fn test_property_after_plain_text() {
        let parser = InlineParser::default();
        let result = parser.parse("text status:: active");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(
            prop_map.get("status"),
            Some(&"active".to_string()),
            "status:: active after text should be parsed as property"
        );
    }

    #[test]
    fn test_property_after_text_multiple() {
        let parser = InlineParser::default();
        let result = parser.parse("meeting status:: active priority:: high");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(prop_map.get("status"), Some(&"active".to_string()));
        assert_eq!(prop_map.get("priority"), Some(&"high".to_string()));
    }

    #[test]
    fn test_property_after_text_with_tag() {
        let parser = InlineParser::default();
        let result = parser.parse("Meeting with #team about [[Project Plan]] status:: active");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(
            prop_map.get("status"),
            Some(&"active".to_string()),
            "status property after mixed syntax should parse correctly"
        );
    }

    // ── BUG 2: tags:: a, b, c only extracts first tag ──

    #[test]
    fn test_tags_property_multiple_tags() {
        let parser = InlineParser::default();
        let result = parser.parse("tags:: a, b, c");
        let normalized = parser.normalize(&result);
        assert_eq!(
            normalized.tags.len(),
            3,
            "tags:: a, b, c should produce 3 tags, got {}: {:?}",
            normalized.tags.len(),
            normalized.tags
        );
        assert!(normalized.tags.contains(&"a".to_string()));
        assert!(normalized.tags.contains(&"b".to_string()));
        assert!(normalized.tags.contains(&"c".to_string()));
    }

    #[test]
    fn test_tags_property_no_spaces() {
        let parser = InlineParser::default();
        let result = parser.parse("tags:: a,b,c");
        let normalized = parser.normalize(&result);
        assert_eq!(
            normalized.tags.len(),
            3,
            "tags:: a,b,c should produce 3 tags, got {}: {:?}",
            normalized.tags.len(),
            normalized.tags
        );
        assert!(normalized.tags.contains(&"a".to_string()));
        assert!(normalized.tags.contains(&"b".to_string()));
        assert!(normalized.tags.contains(&"c".to_string()));
    }

    #[test]
    fn test_tags_property_with_other_properties() {
        let parser = InlineParser::default();
        let result = parser.parse("status:: done tags:: backend, urgent priority:: high");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(prop_map.get("status"), Some(&"done".to_string()));
        assert_eq!(prop_map.get("priority"), Some(&"high".to_string()));
        assert_eq!(
            normalized.tags.len(),
            2,
            "tags:: should produce 2 tags, got {}: {:?}",
            normalized.tags.len(),
            normalized.tags
        );
        assert!(normalized.tags.contains(&"backend".to_string()));
        assert!(normalized.tags.contains(&"urgent".to_string()));
    }

    // ── Parser API: semantic_data() method ──

    #[test]
    fn test_semantic_data_tags() {
        let parser = InlineParser::default();
        let result = parser.parse("#tag1 #tag2");
        let sem = result.semantic_data();
        assert_eq!(sem.tags.len(), 2);
        assert!(sem.tags.contains(&"tag1".to_string()));
        assert!(sem.tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_semantic_data_mixed() {
        let parser = InlineParser::default();
        let result = parser.parse("#urgent meeting [[Project Alpha]] status:: active");
        let sem = result.semantic_data();
        assert_eq!(sem.tags.len(), 1);
        assert!(sem.tags.contains(&"urgent".to_string()));
        assert_eq!(sem.page_refs.len(), 1);
        assert_eq!(sem.page_refs[0], "Project Alpha");
        assert_eq!(sem.properties_count, 1);
        assert_eq!(sem.properties.get("status"), Some(&"active".to_string()));
    }

    #[test]
    fn test_semantic_data_tags_with_property() {
        let parser = InlineParser::default();
        let result = parser.parse("#bug tags:: frontend, urgent");
        let sem = result.semantic_data();
        assert_eq!(
            sem.tags.len(),
            3,
            "#bug + tags:: frontend, urgent should produce 3 tags total"
        );
        assert!(sem.tags.contains(&"bug".to_string()));
        assert!(sem.tags.contains(&"frontend".to_string()));
        assert!(sem.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_dcolon_as_plain_text_not_property() {
        let parser = InlineParser::default();
        let result = parser.parse("some text :: not a property");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert!(
            prop_map.is_empty(),
            ":: without a valid key should not produce any property"
        );
        assert!(
            normalized.tags.is_empty(),
            ":: without a valid key should not produce any tags"
        );
    }

    #[test]
    fn test_property_at_start_of_content() {
        let parser = InlineParser::default();
        let result = parser.parse("status:: active");
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();
        assert_eq!(prop_map.get("status"), Some(&"active".to_string()));
    }

    // ── TRIANGULATION: edge cases ──

    #[test]
    fn test_tags_many_values() {
        let parser = InlineParser::default();
        let result = parser.parse("tags:: a, b, c, d, e");
        let normalized = parser.normalize(&result);
        assert_eq!(normalized.tags.len(), 5);
        for t in &["a", "b", "c", "d", "e"] {
            assert!(normalized.tags.contains(&t.to_string()));
        }
    }

    #[test]
    fn test_mixed_syntax_with_tags_and_properties() {
        let parser = InlineParser::default();
        // Full realistic block content
        let result = parser.parse(
            "Meeting #standup with [[Team]] about Q1 planning status:: done priority:: high tags:: meeting, planning",
        );
        let normalized = parser.normalize(&result);
        let prop_map = normalized.properties_map();

        // Tags from #tag
        assert!(normalized.tags.contains(&"standup".to_string()));
        // Tags from tags:: property
        assert!(normalized.tags.contains(&"meeting".to_string()));
        assert!(normalized.tags.contains(&"planning".to_string()));
        // Total tags
        assert_eq!(normalized.tags.len(), 3);

        // Properties
        assert_eq!(prop_map.get("status"), Some(&"done".to_string()));
        assert_eq!(prop_map.get("priority"), Some(&"high".to_string()));

        // Refs
        assert!(normalized.page_refs.contains(&"Team".to_string()));
    }

    #[test]
    fn test_parse_empty_string() {
        let parser = InlineParser::default();
        let result = parser.parse("");
        assert!(result.segments.is_empty());
        let sem = result.semantic_data();
        assert!(sem.tags.is_empty());
        assert!(sem.page_refs.is_empty());
        assert_eq!(sem.properties_count, 0);
    }

    #[test]
    fn test_parse_only_whitespace() {
        let parser = InlineParser::default();
        let result = parser.parse("   ");
        assert_eq!(result.segments.len(), 1);
        match &result.segments[0] {
            Segment::Text { content, .. } => {
                assert_eq!(content, "   ");
            }
            _ => panic!("Expected Text for whitespace"),
        }
    }

    #[test]
    fn test_unclosed_page_ref() {
        let parser = InlineParser::default();
        let result = parser.parse("text [[unclosed");
        // [[unclosed should be treated as text since there's no ]]
        let has_page_ref = result
            .segments
            .iter()
            .any(|s| matches!(s, Segment::PageRef { .. }));
        assert!(!has_page_ref, "Unclosed [[ should not be a PageRef");
    }

    #[test]
    fn test_normalize_deduplicates_tags() {
        let parser = InlineParser::default();
        // #tag and tags:: with same tag — semantic_data deduplicates
        let result = parser.parse("#urgent tags:: urgent, normal");
        let sem = result.semantic_data();
        assert_eq!(sem.tags.len(), 2);
        assert!(sem.tags.contains(&"urgent".to_string()));
        assert!(sem.tags.contains(&"normal".to_string()));
    }

    #[test]
    fn test_parse_link() {
        let parser = InlineParser::default();
        let input = "[Google](https://google.com)";
        let result = parser.parse(input);

        // Debug output
        for (i, seg) in result.segments.iter().enumerate() {
            eprintln!("Segment {}: {:?}", i, seg);
        }

        // Should have exactly 1 segment (the link)
        assert_eq!(
            result.segments.len(),
            1,
            "Expected 1 segment, got {}: {:?}",
            result.segments.len(),
            result.segments
        );

        match &result.segments[0] {
            Segment::Link { text, url, .. } => {
                assert_eq!(text, "Google");
                assert_eq!(url, "https://google.com");
            }
            other => panic!("Expected Link, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_bold() {
        let parser = InlineParser::default();
        let result = parser.parse("This is **bold** text");
        assert_eq!(result.segments.len(), 3);
        match &result.segments[1] {
            Segment::Bold { content, .. } => {
                assert_eq!(content, "bold");
            }
            other => panic!("Expected Bold, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_italic() {
        let parser = InlineParser::default();
        let result = parser.parse("This is *italic* text");
        assert_eq!(result.segments.len(), 3);
        match &result.segments[1] {
            Segment::Italic { content, .. } => {
                assert_eq!(content, "italic");
            }
            other => panic!("Expected Italic, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_code() {
        let parser = InlineParser::default();
        let result = parser.parse("Use `code` here");
        assert_eq!(result.segments.len(), 3);
        match &result.segments[1] {
            Segment::Code { content, .. } => {
                assert_eq!(content, "code");
            }
            other => panic!("Expected Code, got {:?}", other),
        }
    }
}
