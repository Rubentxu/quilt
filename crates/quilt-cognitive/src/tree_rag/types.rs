//! TreeRAG types — data structures for the TreeRAG engine
//!
//! This module defines all input/output types for the TreeRAG engine.
//! In the MCP-first architecture, Quilt provides structured data and the
//! AI agent handles synthesis.

use quilt_domain::value_objects::Uuid;
use serde::{Deserialize, Serialize};

// ── Scope ───────────────────────────────────────────────────────────────

/// Defines which pages to include in a report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ReportScope {
    /// The LLM decides which pages to include
    Auto,
    /// Specific pages by name
    Pages(Vec<String>),
    /// Last N days of journal entries
    JournalLast(u32),
    /// Blocks tagged with a specific tag
    Tagged(String),
    /// Every page in the graph
    AllPages,
}

// ── Format ──────────────────────────────────────────────────────────────

/// Output format for reports.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Markdown document only
    Markdown,
    /// Markdown + PDF bytes
    FullDocument,
}

// ── Request ─────────────────────────────────────────────────────────────

/// A request to generate a structured report from the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    /// Natural language topic (e.g., "Guía de async patterns en Rust")
    pub topic: String,
    /// Which pages to include
    pub scope: ReportScope,
    /// Output format
    pub format: ReportFormat,
    /// Override: max blocks to search (default from config)
    pub max_blocks: Option<usize>,
    /// Override: max sections in the outline (default from config)
    pub max_sections: Option<usize>,
    /// Override: max blocks per section (default from config)
    pub max_blocks_per_section: Option<usize>,
}

// ── Pipeline intermediates ──────────────────────────────────────────────

/// A cluster of blocks sharing a common theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCluster {
    /// Human-readable label
    pub label: String,
    /// Brief description of the cluster
    pub summary: String,
    /// Block IDs belonging to this cluster
    pub block_ids: Vec<Uuid>,
    /// Relevance score (0.0 - 1.0)
    pub relevance: f32,
}

/// A section of content for document assembly, provided by the AI agent.
/// The agent synthesizes the content; Quilt stores and renders it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledSection {
    /// Section heading
    pub heading: String,
    /// Heading level (1 = top, 2 = subsection, etc.)
    pub level: u8,
    /// Content text (already synthesized by the agent)
    pub content: String,
    /// Source block IDs for citations
    pub source_block_ids: Vec<Uuid>,
    /// Subsections
    pub subsections: Vec<AssembledSection>,
}

// ── Internal tree representation ────────────────────────────────────────

/// A node in the navigable block tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    /// Block ID
    pub block_id: Uuid,
    /// Page name this block belongs to
    pub page_name: String,
    /// First line of content or heading
    pub title: String,
    /// LLM-generated summary (may be empty if not indexed)
    pub summary: String,
    /// Number of children
    pub children_count: usize,
    /// Child nodes (recursive, may be pruned)
    pub children: Vec<TreeNode>,
}

/// A complete page-level tree index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeIndex {
    pub page_id: Uuid,
    pub page_name: String,
    pub root: TreeNode,
    pub total_blocks: usize,
}

// ── Output ──────────────────────────────────────────────────────────────

/// The final generated report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    pub title: String,
    pub description: String,
    /// Full Markdown document
    pub markdown: String,
    /// PDF bytes (only if requested)
    pub pdf_bytes: Option<Vec<u8>>,
    /// Structured sections provided by the AI agent
    pub sections: Vec<AssembledSection>,
    /// All citations with source locations
    pub citations: Vec<Citation>,
    /// Pages that contributed content
    pub source_pages: Vec<String>,
    /// How many blocks were used
    pub block_count: usize,
    /// When the report was generated
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// A citation linking report content to source blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Section heading where this appears
    pub section: String,
    /// Source block ID
    pub block_id: Uuid,
    /// Page containing the block
    pub page_name: String,
    /// First 100 chars of source content
    pub snippet: String,
}

/// Status of the TreeRAG index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeRagStatus {
    /// Total blocks in the graph
    pub total_blocks: usize,
    /// Blocks with summaries
    pub indexed_blocks: usize,
    /// Blocks pending indexing
    pub pending_blocks: usize,
    /// Pending scheduled tasks
    pub scheduled_tasks: usize,
}

// ── Configuration ───────────────────────────────────────────────────────

/// Configuration for the TreeRAG engine.
#[derive(Debug, Clone)]
pub struct TreeRagConfig {
    /// Max blocks to consider during search/exploration
    pub max_blocks_search: usize,
    /// Max blocks included in a report
    pub max_blocks_per_report: usize,
    /// Max top-level sections
    pub max_sections: usize,
    /// Max source blocks per section
    pub max_blocks_per_section: usize,
    /// Max tokens for a block summary
    pub summary_max_tokens: usize,
    /// Max blocks per index batch operation
    pub max_blocks_per_index_batch: usize,
    /// Whether to lazily index on query
    pub auto_index_on_query: bool,
    /// Hours between scheduled index rebuilds
    pub index_rebuild_interval_hours: u32,
    /// PDF body font size in points
    pub pdf_font_size: u8,
    /// Whether to include a table of contents in PDF
    pub pdf_include_toc: bool,
}

impl Default for TreeRagConfig {
    fn default() -> Self {
        Self {
            max_blocks_search: 500,
            max_blocks_per_report: 200,
            max_sections: 10,
            max_blocks_per_section: 5,
            summary_max_tokens: 80,
            max_blocks_per_index_batch: 50,
            auto_index_on_query: true,
            index_rebuild_interval_hours: 24,
            pdf_font_size: 11,
            pdf_include_toc: true,
        }
    }
}
