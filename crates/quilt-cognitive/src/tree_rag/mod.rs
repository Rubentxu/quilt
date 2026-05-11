//! TreeRAG — MCP-first document generation engine
//!
//! In MCP-first mode, Quilt provides structured data and query capabilities,
//! while the connected AI agent handles LLM synthesis.

pub mod engine;
pub mod format_cache;
pub mod pdf;
pub mod types;

pub use engine::TreeRagEngine;
pub use format_cache::{FormatCache, MldocAst, MldocAstNode, MldocContent};
pub use types::{
    AssembledSection, Citation, GeneratedReport, ReportRequest,
    ReportScope, TopicCluster, TreeIndex, TreeNode, TreeRagConfig, TreeRagStatus,
};
