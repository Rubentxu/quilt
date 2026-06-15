//! Canonicalizer service — V1 Markdown implementation.

pub mod markdown;

#[cfg(test)]
mod markdown_tests;

pub use markdown::MarkdownCanonicalizer;
