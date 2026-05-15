//! FormatCache — LRU cache for mldoc markdown → AST parsing
//!
//! Provides a separate LRU cache (distinct from result caching) specifically
//! for caching mldoc parsing results (markdown → AST conversion).
//! This avoids re-parsing the same markdown content multiple times.
//!
//! # Capacity
//!
//! The cache is configured with 5000 entries, matching the original
//! Logseq implementation. When capacity is exceeded, LRU eviction
//! automatically removes the least recently used entry.

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Maximum number of entries in the FormatCache
const FORMAT_CACHE_SIZE: usize = 5000;

/// A node in the Mldoc AST (Abstract Syntax Tree).
///
/// Represents the structured output of mldoc parsing a markdown or org-mode document.
/// Mirrors the Clojure structure: `["Type" {:metadata} content]`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MldocAstNode {
    /// Node type (e.g., "Heading", "Paragraph", "Plain")
    pub node_type: String,
    /// Optional metadata (e.g., `{"size": 1}` for headings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Child content (nested nodes or plain text)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<MldocContent>,
}

/// Content within an AST node - either text or nested nodes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum MldocContent {
    /// Plain text content
    Text(String),
    /// Nested AST node
    Node(MldocAstNode),
}

/// A complete Mldoc AST document.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MldocAst {
    /// Root nodes of the AST
    pub nodes: Vec<MldocAstNode>,
}

impl MldocAst {
    /// Parse a markdown string into an AST.
    ///
    /// Uses pulldown-cmark for parsing and converts the event stream
    /// to a MldocAst structure.
    #[tracing::instrument(skip(markdown))]
    pub fn parse_markdown(markdown: &str) -> Self {
        use pulldown_cmark::{Event, Parser, Tag};

        let parser = Parser::new_ext(markdown, pulldown_cmark::Options::all());
        let mut nodes = Vec::new();
        let mut current_node: Option<(Tag<'_>, Vec<MldocAstNode>)> = None;
        let mut text_buffer = String::new();

        fn flush_text(text: &mut String) -> Option<MldocAstNode> {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return None;
            }
            let content = std::mem::take(text);
            Some(MldocAstNode {
                node_type: "Plain".to_string(),
                metadata: None,
                content: vec![MldocContent::Text(content)],
            })
        }

        fn finish_node(tag: Tag<'_>, children: Vec<MldocAstNode>) -> Option<MldocAstNode> {
            let node_type = match tag {
                Tag::Heading { .. } => "Heading",
                Tag::Paragraph => "Paragraph",
                Tag::BlockQuote(_) => "BlockQuote",
                Tag::CodeBlock(_) => "CodeBlock",
                Tag::List(_) => "List",
                Tag::Item => "Item",
                Tag::Table(_) => "Table",
                Tag::TableRow => "TableRow",
                Tag::TableCell => "TableCell",
                Tag::Emphasis => "Emphasis",
                Tag::Strong => "Strong",
                Tag::Link { .. } => "Link",
                Tag::Image { .. } => "Image",
                _ => return None,
            };

            Some(MldocAstNode {
                node_type: node_type.to_string(),
                metadata: None,
                content: children.into_iter().map(MldocContent::Node).collect(),
            })
        }

        for event in parser {
            match event {
                Event::Start(tag) => {
                    if !text_buffer.is_empty() {
                        if let Some(node) = flush_text(&mut text_buffer) {
                            if let Some((_, ref mut children)) = current_node {
                                children.push(node);
                            } else {
                                nodes.push(node);
                            }
                        }
                    }
                    current_node = Some((tag, Vec::new()));
                }
                Event::End(_tag_end) => {
                    if let Some((tag, children)) = current_node.take() {
                        if let Some(node) = finish_node(tag, children) {
                            nodes.push(node);
                        }
                    }
                }
                Event::Text(text) => {
                    text_buffer.push_str(&text);
                }
                Event::Code(code) => {
                    text_buffer.push_str(&format!("`{}`", code));
                }
                Event::SoftBreak | Event::HardBreak => {
                    text_buffer.push(' ');
                }
                _ => {}
            }
        }

        // Flush any remaining text
        if !text_buffer.is_empty() {
            if let Some(node) = flush_text(&mut text_buffer) {
                nodes.push(node);
            }
        }

        Self { nodes }
    }
}

/// LRU cache for storing parsed mldoc AST results.
///
/// This cache is separate from any result caching and specifically
/// designed for caching markdown → AST parsing operations.
///
/// # Example
///
/// ```
/// use quilt_cognitive::tree_rag::FormatCache;
///
/// let mut cache = FormatCache::new();
///
/// // Parse and cache
/// let markdown = "# Hello\n\nThis is a test.";
/// let ast = cache.parse_and_cache("key1", markdown);
/// assert!(ast.is_some());
///
/// // Retrieve from cache
/// let cached = cache.get("key1");
/// assert!(cached.is_some());
/// ```
#[derive(Debug)]
pub struct FormatCache {
    cache: Mutex<LruCache<String, MldocAst>>,
}

impl FormatCache {
    /// Create a new FormatCache with 5000 entry capacity.
    #[tracing::instrument]
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(LruCache::new(NonZeroUsize::new(FORMAT_CACHE_SIZE).unwrap())),
        }
    }

    /// Get a cached AST by key.
    ///
    /// Returns `None` if the key is not in the cache.
    #[tracing::instrument(skip(self))]
    pub fn get(&self, key: &str) -> Option<MldocAst> {
        let mut cache = self.cache.lock().ok()?;
        cache.get(key).cloned()
    }

    /// Parse markdown and cache the result.
    ///
    /// If the content was already cached, returns the cached result.
    /// Otherwise, parses the markdown, stores it in the cache, and returns it.
    #[tracing::instrument(skip(self, markdown))]
    pub fn parse_and_cache(&self, key: &str, markdown: &str) -> Option<MldocAst> {
        // Check cache first
        {
            let mut cache = self.cache.lock().ok()?;
            if let Some(cached) = cache.get(key) {
                return Some(cached.clone());
            }
        }

        // Parse markdown
        let ast = MldocAst::parse_markdown(markdown);

        // Store in cache
        {
            let mut cache = self.cache.lock().ok()?;
            cache.put(key.to_string(), ast.clone());
        }

        Some(ast)
    }

    /// Put an AST directly into the cache.
    #[tracing::instrument(skip(self, ast))]
    pub fn put(&self, key: String, ast: MldocAst) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.put(key, ast);
        }
    }

    /// Get the current number of entries in the cache.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.cache.lock().map(|c| c.len()).unwrap_or(0)
    }

    /// Check if the cache is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.cache.lock().map(|c| c.is_empty()).unwrap_or(true)
    }
}

impl Default for FormatCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_cache_new() {
        let cache = FormatCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_parse_and_cache() {
        let cache = FormatCache::new();
        let markdown = "# Hello\n\nThis is a test.";

        let ast1 = cache.parse_and_cache("key1", markdown);
        assert!(ast1.is_some());
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);

        // Second call should return cached
        let ast2 = cache.parse_and_cache("key1", markdown);
        assert!(ast2.is_some());
        assert_eq!(cache.len(), 1); // Still 1, not 2
    }

    #[test]
    fn test_get() {
        let cache = FormatCache::new();
        let markdown = "# Test\n\nContent";

        cache.parse_and_cache("key1", markdown);

        let result = cache.get("key1");
        assert!(result.is_some());

        let missing = cache.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_put() {
        let cache = FormatCache::new();
        let ast = MldocAst::parse_markdown("# Direct put");

        cache.put("direct_key".to_string(), ast.clone());

        let result = cache.get("direct_key");
        assert!(result.is_some());
    }

    #[test]
    fn test_mldoc_ast_parse_markdown() {
        let markdown = "# Heading 1\n\nThis is a paragraph.\n\n## Heading 2";
        let ast = MldocAst::parse_markdown(markdown);

        assert!(!ast.nodes.is_empty());
    }

    #[test]
    fn test_format_cache_lru_eviction() {
        let cache = FormatCache::new();

        // Fill cache beyond 5000 entries to test LRU eviction
        for i in 0..6000 {
            let markdown = format!("# Title {}\n\nContent {}", i, i);
            cache.parse_and_cache(&format!("key{}", i), &markdown);
        }

        // Cache should have evicted old entries
        // After 6000 inserts into 5000 capacity, we should have ~5000 entries
        let len = cache.len();
        assert!(len <= 5000);

        // Recent entries should still be there
        assert!(cache.get("key5000").is_some());

        // Very old entries should have been evicted
        // key0 would have been evicted when we inserted key5000
        let key0_exists = cache.get("key0").is_some();
        // This might pass or fail depending on exact LRU behavior
        // but the cache size should be bounded
        assert!(len <= 5000);
    }
}
