//! Markdown renderer with support for page links [[page]], block links ((block-id)), and Mermaid diagrams

use pulldown_cmark::{html, Event, Parser, Tag, TagEnd};
use regex::Regex;
use std::collections::HashMap;

pub struct MarkdownRenderer {
    page_links: HashMap<String, String>,
    block_links: HashMap<String, String>,
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self {
            page_links: HashMap::new(),
            block_links: HashMap::new(),
        }
    }

    pub fn with_page_links(mut self, links: HashMap<String, String>) -> Self {
        self.page_links = links;
        self
    }

    pub fn with_block_links(mut self, links: HashMap<String, String>) -> Self {
        self.block_links = links;
        self
    }

    pub fn render_to_html(&self, markdown: &str) -> String {
        let with_page_links = self.render_page_links(markdown);
        let with_block_links = self.render_block_links(&with_page_links);
        let with_mermaid = self.render_mermaid_blocks(&with_block_links);
        self.parse_and_render(&with_mermaid)
    }

    fn render_page_links(&self, markdown: &str) -> String {
        let page_link_re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        page_link_re
            .replace_all(markdown, |caps: &regex::Captures| {
                let page_name = &caps[1];
                let href = format!("/page/{}", urlencoding::encode(page_name));
                format!(r##"<a href="{}" class="page-link">{}</a>"##, href, page_name)
            })
            .to_string()
    }

    fn render_block_links(&self, markdown: &str) -> String {
        let block_link_re = Regex::new(r"\(\(([a-zA-Z0-9-_]+)\)\)").unwrap();
        block_link_re
            .replace_all(markdown, |caps: &regex::Captures| {
                let block_id = &caps[1];
                let display_text = self
                    .block_links
                    .get(block_id)
                    .cloned()
                    .unwrap_or_else(|| block_id.to_string());
                format!(
                    r##"<a href="#" class="block-link" data-block-id="{}">{}</a>"##,
                    block_id, display_text
                )
            })
            .to_string()
    }

    fn render_mermaid_blocks(&self, markdown: &str) -> String {
        let mermaid_re = Regex::new(r"```mermaid\n([\s\S]*?)```").unwrap();
        mermaid_re
            .replace_all(markdown, |caps: &regex::Captures| {
                let code = &caps[1];
                format!(r#"<div class="mermaid">{}</div>"#, code.trim())
            })
            .to_string()
    }

    fn parse_and_render(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        html_output
    }
}

pub fn render_markdown(markdown: &str) -> String {
    let renderer = MarkdownRenderer::new();
    renderer.render_to_html(markdown)
}

pub fn render_markdown_with_links(
    markdown: &str,
    page_links: HashMap<String, String>,
    block_links: HashMap<String, String>,
) -> String {
    let renderer = MarkdownRenderer::new()
        .with_page_links(page_links)
        .with_block_links(block_links);
    renderer.render_to_html(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let html = render_markdown("# Hello\n\nThis is **bold** and *italic*.");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
    }

    #[test]
    fn test_page_links() {
        let html = render_markdown("See [[My Page]] for details.");
        assert!(html.contains(r#"class="page-link""#));
        assert!(html.contains("My Page"));
    }

    #[test]
    fn test_block_links() {
        let block_links = HashMap::from([("abc123".to_string(), "My Block Content".to_string())]);
        let html = render_markdown_with_links("Reference ((abc123)) here.", HashMap::new(), block_links);
        assert!(html.contains(r#"class="block-link""#));
        assert!(html.contains(r#"data-block-id="abc123""#));
    }

    #[test]
    fn test_mermaid_blocks() {
        let html = render_markdown("```mermaid\ngraph TD\n  A-->B\n```");
        assert!(html.contains(r#"class="mermaid""#));
        assert!(html.contains("graph TD"));
        assert!(html.contains("A-->B"));
    }
}
