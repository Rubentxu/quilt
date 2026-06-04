//! Logseq markdown parser.
//!
//! Parses Logseq-flavored markdown files into a tree of blocks
//! with optional YAML frontmatter.

use thiserror::Error;

/// Parsed frontmatter key-value pair.
#[derive(Debug, Clone, PartialEq)]
pub struct FrontmatterProperty {
    pub key: String,
    pub value: String,
}

/// Parsed frontmatter block.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frontmatter {
    pub properties: Vec<FrontmatterProperty>,
}

/// One block in the parsed document tree.
#[derive(Debug, Clone, PartialEq)]
pub struct RawBlock {
    /// Indent level (0 = top-level, 1 = first indent, etc.)
    pub indent_level: usize,
    /// Raw content line (without the bullet/hyphen marker).
    pub content: String,
    /// Properties declared on this block (after `::`).
    pub properties: Vec<FrontmatterProperty>,
    /// Child blocks nested under this block.
    pub children: Vec<RawBlock>,
}

/// Errors that can occur during Logseq markdown parsing.
#[derive(Debug, Error, PartialEq)]
pub enum MigrationError {
    #[error("Unclosed frontmatter: missing closing '---'")]
    UnclosedFrontmatter,

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

/// Parse a Logseq markdown string into frontmatter and blocks.
pub fn parse_logseq(input: &str) -> Result<(Frontmatter, Vec<RawBlock>), MigrationError> {
    let mut frontmatter = Frontmatter::default();
    let mut blocks: Vec<RawBlock> = Vec::new();
    
    let mut lines = input.lines().peekable();
    
    // Check for frontmatter
    if let Some(first) = lines.peek() {
        if first.trim() == "---" {
            // Consume the opening ---
            lines.next();
            
            // Parse frontmatter lines until closing ---
            let mut in_frontmatter = true;
            while let Some(line) = lines.next() {
                let trimmed = line.trim();
                if trimmed == "---" {
                    in_frontmatter = false;
                    break;
                }
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = parse_property_line(trimmed) {
                    frontmatter.properties.push(FrontmatterProperty { key, value });
                }
            }
            
            if in_frontmatter {
                return Err(MigrationError::UnclosedFrontmatter);
            }
        }
    }
    
    // Parse blocks
    let mut current_indent = 0;
    let mut block_content_lines: Vec<(usize, String)> = Vec::new();
    
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        // Calculate indent level (2 spaces = 1 level)
        let indent = line.len() - line.trim_start().len();
        let indent_level = indent / 2;
        
        // Check for property line
        if let Some((key, value)) = parse_property_line(trimmed) {
            // This is a property, not a block
            continue;
        }
        
        // Check if this is a new block at a different indent
        if indent_level != current_indent || block_content_lines.is_empty() {
            if !block_content_lines.is_empty() {
                // Save the previous block
                let content = block_content_lines.iter()
                    .map(|(_, l)| l.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                if !content.is_empty() {
                    blocks.push(RawBlock {
                        indent_level: current_indent,
                        content,
                        properties: Vec::new(),
                        children: Vec::new(),
                    });
                }
                block_content_lines.clear();
            }
            current_indent = indent_level;
        }
        
        block_content_lines.push((indent_level, trimmed.to_string()));
    }
    
    // Don't forget the last block
    if !block_content_lines.is_empty() {
        let content = block_content_lines.iter()
            .map(|(_, l)| l.clone())
            .collect::<Vec<_>>()
            .join("\n");
        if !content.is_empty() {
            blocks.push(RawBlock {
                indent_level: current_indent,
                content,
                properties: Vec::new(),
                children: Vec::new(),
            });
        }
    }
    
    Ok((frontmatter, blocks))
}

/// Parse a property line like "key: value" or "key: value: with: colons"
/// Returns None if the line doesn't contain a colon (not a property).
fn parse_property_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.splitn(2, ':');
    let key = parts.next()?.trim().to_string();
    let value = parts.next().map(|v| v.trim().to_string()).unwrap_or_default();
    
    if key.is_empty() {
        return None;
    }
    
    // If there's no colon separator, this isn't a property line
    if !line.contains(':') {
        return None;
    }
    
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_frontmatter() {
        let input = r#"---
title: My Page
tags: [tag1, tag2]
---
"#;
        let (fm, _blocks) = parse_logseq(input).unwrap();
        assert_eq!(fm.properties.len(), 2);
        assert_eq!(fm.properties[0].key, "title");
        assert_eq!(fm.properties[0].value, "My Page");
        assert_eq!(fm.properties[1].key, "tags");
        assert_eq!(fm.properties[1].value, "[tag1, tag2]");
    }

    #[test]
    fn parse_page_without_frontmatter() {
        let input = "Just some content without frontmatter.";
        let (fm, blocks) = parse_logseq(input).unwrap();
        assert!(fm.properties.is_empty());
        assert!(!blocks.is_empty() || input.is_empty());
    }

    #[test]
    fn unclosed_frontmatter_returns_error() {
        let input = r#"---
title: My Page
"#;
        let result = parse_logseq(input);
        assert!(matches!(result, Err(MigrationError::UnclosedFrontmatter)));
    }
}
