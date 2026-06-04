//! Markdown import parser.
//!
//! Parses Markdown files (Quilt-flavored) into a tree of blocks
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

/// Errors that can occur during Markdown parsing.
#[derive(Debug, Error, PartialEq)]
pub enum MigrationError {
    #[error("Unclosed frontmatter: missing closing '---'")]
    UnclosedFrontmatter,

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

/// Parse a Markdown import string into frontmatter and blocks.
pub fn parse_md_import(input: &str) -> Result<(Frontmatter, Vec<RawBlock>), MigrationError> {
    let mut frontmatter = Frontmatter::default();
    let mut blocks: Vec<RawBlock> = Vec::new();
    
    let mut lines = input.lines().peekable();
    
    // Check for frontmatter
    if let Some(first) = lines.peek() {
        if first.trim() == "---" {
            lines.next();
            
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
    
    // Parse blocks using indent-based tree building
    // We track where to add children at each indent level
    // parent_at_indent[indent] = index in blocks (or children of some block)
    let mut parent_at_indent: Vec<(usize, usize)> = Vec::new(); // (indent, index in parent's children or blocks)
    
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        // Calculate indent level (2 spaces = 1 level)
        let indent = line.len() - line.trim_start().len();
        let indent_level = indent / 2;
        
        // Skip property lines
        if parse_property_line(trimmed).is_some() {
            continue;
        }
        
        // Adjust parent stack - remove entries for deeper indents than current
        parent_at_indent.truncate(indent_level);
        
        // Create new block
        let new_block = RawBlock {
            indent_level,
            content: trimmed.to_string(),
            properties: Vec::new(),
            children: Vec::new(),
        };
        
        // Find where to add based on indent level
        if indent_level == 0 {
            // Top-level block - add directly to blocks
            blocks.push(new_block);
            let idx = blocks.len() - 1;
            parent_at_indent.push((indent_level, idx));
        } else if let Some(&(parent_indent, _parent_idx)) = parent_at_indent.last() {
            // Add as child of the appropriate parent
            if parent_indent == indent_level - 1 {
                // Parent is one level up - add to its children
                let parent_block = get_block_mut(&mut blocks, &parent_at_indent);
                if let Some(parent) = parent_block {
                    parent.children.push(new_block);
                    let child_idx = parent.children.len() - 1;
                    parent_at_indent.push((indent_level, child_idx));
                }
            }
        }
    }
    
    Ok((frontmatter, blocks))
}

/// Get a mutable reference to a block by following the parent_at_indent path
fn get_block_mut<'a>(blocks: &'a mut Vec<RawBlock>, path: &[(usize, usize)]) -> Option<&'a mut RawBlock> {
    if path.is_empty() {
        return None;
    }
    // First entry is at top level
    let mut current = &mut blocks[path[0].1];
    // Subsequent entries are in children
    for &(_, child_idx) in &path[1..] {
        current = &mut current.children[child_idx];
    }
    Some(current)
}

/// Parse a frontmatter property line like "key: value"
/// Returns None if the line doesn't match the format (single colon, not double).
fn parse_property_line(line: &str) -> Option<(String, String)> {
    // Frontmatter properties use single colon, not double colon
    // So "title: My Page" matches but "property:: value" does not
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    let key = parts[0].trim();
    let value = parts[1].trim();
    
    if key.is_empty() {
        return None;
    }
    
    // Must have exactly one colon (single colon for frontmatter)
    // If there are more colons, check it's not a double-colon property
    if line.contains("::") {
        return None;
    }
    
    Some((key.to_string(), value.to_string()))
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
        let (fm, _blocks) = parse_md_import(input).unwrap();
        assert_eq!(fm.properties.len(), 2);
        assert_eq!(fm.properties[0].key, "title");
        assert_eq!(fm.properties[0].value, "My Page");
    }

    #[test]
    fn parse_page_without_frontmatter() {
        let input = "Just some content without frontmatter.";
        let (fm, blocks) = parse_md_import(input).unwrap();
        assert!(fm.properties.is_empty());
        assert!(!blocks.is_empty() || input.is_empty());
    }

    #[test]
    fn unclosed_frontmatter_returns_error() {
        let input = r#"---
title: My Page
"#;
        let result = parse_md_import(input);
        assert!(matches!(result, Err(MigrationError::UnclosedFrontmatter)));
    }

    #[test]
    fn parse_nested_blocks_tree_structure() {
        let input = "\
- Root
  - Level 1
    - Level 2
";
        let (_, blocks) = parse_md_import(input).unwrap();
        
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].content, "- Root");
        assert_eq!(blocks[0].children.len(), 1);
        assert_eq!(blocks[0].children[0].content, "- Level 1");
        assert_eq!(blocks[0].children[0].children.len(), 1);
        assert_eq!(blocks[0].children[0].children[0].content, "- Level 2");
        assert_eq!(blocks[0].children[0].children[0].children.len(), 0);
    }

    #[test]
    fn parse_multiple_siblings_at_top_level() {
        let input = "\
- Block 1
- Block 2
- Block 3
";
        let (_, blocks) = parse_md_import(input).unwrap();
        
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].content, "- Block 1");
        assert_eq!(blocks[1].content, "- Block 2");
        assert_eq!(blocks[2].content, "- Block 3");
    }

    #[test]
    fn parse_mixed_nesting() {
        let input = "\
- Root 1
  - Child 1.1
  - Child 1.2
- Root 2
  - Child 2.1
";
        let (_, blocks) = parse_md_import(input).unwrap();
        
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].content, "- Root 1");
        assert_eq!(blocks[0].children.len(), 2);
        assert_eq!(blocks[1].content, "- Root 2");
        assert_eq!(blocks[1].children.len(), 1);
    }
}
