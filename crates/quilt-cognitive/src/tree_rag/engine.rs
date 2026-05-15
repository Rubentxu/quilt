//! TreeRAG engine — MCP-first document generation infrastructure
//!
//! In the MCP-first architecture, Quilt provides structured data/query/PDF-rendering
//! capabilities, and the connected AI agent does LLM synthesis.
//!
//! This engine provides:
//! - Topic exploration (structural clustering via FTS + tags + page refs)
//! - Tree navigation (hierarchical block trees per page)
//! - Document assembly (renders agent-provided sections as Markdown/PDF)
//! - Block summary storage (agent-generated summaries persisted via MCP)
//! - Index status and rebuild tracking

use crate::tree_rag::types::{
    AssembledSection, Citation, GeneratedReport, ReportFormat, ReportRequest, ReportScope,
    TopicCluster, TreeIndex, TreeNode, TreeRagConfig, TreeRagStatus,
};
use quilt_domain::entities::BlockSummary;
use quilt_domain::repositories::{BlockRepository, BlockSummaryRepository, PageRepository};
use quilt_domain::value_objects::Uuid;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;

// ── Error ────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum TreeRagError {
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),

    #[error("No blocks found for topic: {0}")]
    NoBlocksFound(String),

    #[error("Page not found: {0}")]
    PageNotFound(Uuid),

    #[error("PDF generation failed: {0}")]
    Pdf(String),

    #[error("Max blocks exceeded: {0} > {1}")]
    MaxBlocksExceeded(usize, usize),
}

// ── Engine ───────────────────────────────────────────────────────────────

/// TreeRAG engine — MCP-first data/query infrastructure.
///
/// Provides tree navigation, structural clustering, document assembly,
/// and PDF rendering. The AI agent handles synthesis.
#[derive(Clone)]
pub struct TreeRagEngine {
    pub config: Arc<TreeRagConfig>,
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    pub summary_repo: Arc<dyn BlockSummaryRepository>,
}

impl std::fmt::Debug for TreeRagEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeRagEngine")
            .field("config", &self.config)
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("page_repo", &"Arc<dyn PageRepository>")
            .field("summary_repo", &"Arc<dyn BlockSummaryRepository>")
            .finish()
    }
}

impl TreeRagEngine {
    /// Create a new TreeRagEngine (no ai_client needed — MCP-first).
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        summary_repo: Arc<dyn BlockSummaryRepository>,
    ) -> Self {
        Self {
            config: Arc::new(TreeRagConfig::default()),
            block_repo,
            page_repo,
            summary_repo,
        }
    }

    pub fn with_config(mut self, config: TreeRagConfig) -> Self {
        self.config = Arc::new(config);
        self
    }

    // ── Public API ──────────────────────────────────────────────────────

    /// Explore a topic: discover pages, retrieve blocks, cluster them by structure.
    ///
    /// Uses FTS text matching + tag overlap + page_ref proximity — NO embeddings.
    pub async fn explore_topic(
        &self,
        topic: &str,
        scope: &ReportScope,
    ) -> Result<Vec<TopicCluster>, TreeRagError> {
        let page_ids = self.resolve_scope(scope).await?;
        if page_ids.is_empty() {
            return Ok(vec![]);
        }

        // Collect all blocks from scoped pages
        let mut all_blocks = Vec::new();
        for page_id in &page_ids {
            let blocks = self.block_repo.get_by_page(*page_id).await?;
            all_blocks.extend(blocks);
        }

        if all_blocks.is_empty() {
            return Ok(vec![]);
        }

        // Cap to search limit
        let max = self.config.max_blocks_search;
        if all_blocks.len() > max {
            all_blocks.truncate(max);
        }

        // Structural clustering: group by page, score by keyword match
        let clusters = self.structural_cluster(topic, &all_blocks);
        Ok(clusters)
    }

    /// Build a navigable tree from a page's blocks.
    pub async fn build_tree(&self, page_id: Uuid) -> Result<TreeIndex, TreeRagError> {
        let page = self
            .page_repo
            .get_by_id(page_id)
            .await?
            .ok_or(TreeRagError::PageNotFound(page_id))?;
        let blocks = self.block_repo.get_by_page(page_id).await?;

        let root = build_subtree(self.summary_repo.as_ref(), None, &blocks).await?;
        Ok(TreeIndex {
            page_id,
            page_name: page.name,
            total_blocks: blocks.len(),
            root,
        })
    }

    /// Query/filter a tree by text match on title or summary.
    pub async fn query_tree(&self, page_id: Uuid, query: &str) -> Result<TreeIndex, TreeRagError> {
        let tree = self.build_tree(page_id).await?;
        let query_lower = query.to_lowercase();

        let filtered_root = self.filter_tree_node(&tree.root, &query_lower);
        Ok(TreeIndex {
            page_id,
            page_name: tree.page_name,
            total_blocks: self.count_nodes(&filtered_root),
            root: filtered_root,
        })
    }

    /// Assemble a Markdown document from agent-provided sections.
    ///
    /// The agent synthesizes the content; Quilt formats it with citations.
    pub fn assemble_document(
        &self,
        title: &str,
        description: &str,
        sections: &[AssembledSection],
    ) -> String {
        let mut doc = format!("# {}\n\n{}\n\n", title, description);

        for section in sections {
            let heading_prefix = "#".repeat(section.level as usize + 1);
            doc.push_str(&format!("{} {}\n\n", heading_prefix, section.heading));
            doc.push_str(&section.content);
            doc.push_str("\n\n");

            // Source citations
            if !section.source_block_ids.is_empty() {
                doc.push_str("*Sources: ");
                let citation_strs: Vec<String> = section
                    .source_block_ids
                    .iter()
                    .map(|id| format!("[{}]", id))
                    .collect();
                doc.push_str(&citation_strs.join(", "));
                doc.push_str("*\n\n");
            }

            // Subsections
            for sub in &section.subsections {
                let sub_prefix = "#".repeat(sub.level as usize + 1);
                doc.push_str(&format!("{} {}\n\n", sub_prefix, sub.heading));
                doc.push_str(&sub.content);
                doc.push_str("\n\n");
            }
        }

        doc
    }

    /// Render Markdown to PDF.
    pub fn render_pdf(&self, markdown: &str) -> Result<Vec<u8>, TreeRagError> {
        crate::tree_rag::pdf::render_markdown_to_pdf(markdown, self.config.as_ref())
            .map_err(TreeRagError::Pdf)
    }

    /// Assemble a full report: builds tree, fetches citations, optionally renders PDF.
    ///
    /// This is a convenience method that combines explore + build_tree + assemble_document.
    /// For full control, use explore_topic + build_tree + assemble_document separately.
    pub async fn generate_report(
        &self,
        request: &ReportRequest,
    ) -> Result<GeneratedReport, TreeRagError> {
        let max_blocks = request
            .max_blocks
            .unwrap_or(self.config.max_blocks_per_report);

        // Explore and cluster
        let clusters = self.explore_topic(&request.topic, &request.scope).await?;

        if clusters.is_empty() {
            return Err(TreeRagError::NoBlocksFound(request.topic.clone()));
        }

        // Build trees for contributing pages
        let mut source_pages_set = std::collections::HashSet::new();
        let mut all_block_ids = Vec::new();
        for cluster in &clusters {
            all_block_ids.extend(&cluster.block_ids);
        }

        // Deduplicate and cap
        all_block_ids.sort_by_key(|a: &Uuid| a.to_string());
        all_block_ids.dedup();
        if all_block_ids.len() > max_blocks {
            all_block_ids.truncate(max_blocks);
        }

        // Collect source pages
        for &block_id in &all_block_ids {
            if let Ok(Some(block)) = self.block_repo.get_by_id(block_id).await {
                source_pages_set.insert(block.page_id);
            }
        }

        // Build tree for first page (representative)
        let first_page_id = *source_pages_set.iter().next().unwrap_or(&Uuid::nil());
        let _tree = if first_page_id != Uuid::nil() {
            self.build_tree(first_page_id).await.ok()
        } else {
            None
        };

        // Collect citations
        let citations = self.collect_citations(&all_block_ids).await?;

        // Assemble placeholder sections from clusters (agent replaces these)
        let sections: Vec<AssembledSection> = clusters
            .iter()
            .take(request.max_sections.unwrap_or(self.config.max_sections))
            .map(|c| AssembledSection {
                heading: c.label.clone(),
                level: 2,
                content: c.summary.clone(),
                source_block_ids: c.block_ids.clone(),
                subsections: vec![],
            })
            .collect();

        let title = format!("Report: {}", request.topic);
        let description = format!(
            "Structured report on '{}' using {} blocks from {} pages",
            request.topic,
            all_block_ids.len(),
            source_pages_set.len()
        );

        let markdown = self.assemble_document(&title, &description, &sections);

        let pdf_bytes = if request.format == ReportFormat::FullDocument {
            Some(self.render_pdf(&markdown)?)
        } else {
            None
        };

        let mut source_pages: Vec<String> = Vec::new();
        for page_id in source_pages_set {
            if let Ok(Some(page)) = self.page_repo.get_by_id(page_id).await {
                source_pages.push(page.name);
            }
        }
        source_pages.sort();

        Ok(GeneratedReport {
            title,
            description,
            markdown,
            pdf_bytes,
            sections,
            citations,
            source_pages,
            block_count: all_block_ids.len(),
            generated_at: chrono::Utc::now(),
        })
    }

    /// Get the current status of the index.
    pub async fn status(&self) -> Result<TreeRagStatus, TreeRagError> {
        let pages = self.page_repo.get_all().await?;
        let mut total_blocks = 0usize;
        let mut indexed_blocks = 0usize;

        for page in &pages {
            let blocks = self.block_repo.get_by_page(page.id).await?;
            total_blocks += blocks.len();
            for block in &blocks {
                if self.summary_repo.get(block.id).await?.is_some() {
                    indexed_blocks += 1;
                }
            }
        }

        Ok(TreeRagStatus {
            total_blocks,
            indexed_blocks,
            pending_blocks: total_blocks.saturating_sub(indexed_blocks),
            scheduled_tasks: 0,
        })
    }

    /// Save a block summary (generated by the AI agent via MCP).
    pub async fn save_block_summary(
        &self,
        block_id: Uuid,
        summary: String,
    ) -> Result<(), TreeRagError> {
        let content_hash = self
            .block_repo
            .get_by_id(block_id)
            .await?
            .map(|b| Self::hash_content(&b.content))
            .unwrap_or_default();

        let bs = BlockSummary::new(block_id, summary, content_hash);
        self.summary_repo.upsert(&bs).await?;
        Ok(())
    }

    /// Rebuild index: count stale blocks (content hash changed).
    /// Does NOT generate summaries — the agent generates summaries via save_block_summary.
    pub async fn rebuild_index(&self, scope: Option<&ReportScope>) -> Result<usize, TreeRagError> {
        let page_ids = if let Some(s) = scope {
            self.resolve_scope(s).await?
        } else {
            let pages = self.page_repo.get_all().await?;
            pages.iter().map(|p| p.id).collect()
        };

        let mut stale_count = 0;
        for page_id in &page_ids {
            let blocks = self.block_repo.get_by_page(*page_id).await?;
            for block in blocks {
                let current_hash = Self::hash_content(&block.content);
                if let Some(existing) = self.summary_repo.get(block.id).await? {
                    if existing.is_stale(&current_hash) {
                        stale_count += 1;
                    }
                } else {
                    // No summary exists — counts as pending
                    stale_count += 1;
                }
            }
        }
        Ok(stale_count)
    }

    // ── Internal: Scope Resolution ─────────────────────────────────────

    async fn resolve_scope(&self, scope: &ReportScope) -> Result<Vec<Uuid>, TreeRagError> {
        match scope {
            ReportScope::Auto => {
                let pages = self.page_repo.get_all().await?;
                Ok(pages.iter().map(|p| p.id).collect())
            }
            ReportScope::Pages(names) => {
                let mut ids = Vec::new();
                for name in names {
                    if let Some(page) = self.page_repo.get_by_name(name).await? {
                        ids.push(page.id);
                    }
                }
                Ok(ids)
            }
            ReportScope::JournalLast(days) => {
                let since = chrono::Utc::now() - chrono::Duration::days(*days as i64);
                let pages = self.page_repo.get_updated_since(since).await?;
                Ok(pages.iter().filter(|p| p.journal).map(|p| p.id).collect())
            }
            ReportScope::Tagged(_tag) => {
                // Tag filtering via FTS happens in structural_cluster
                let pages = self.page_repo.get_all().await?;
                Ok(pages.iter().map(|p| p.id).collect())
            }
            ReportScope::AllPages => {
                let pages = self.page_repo.get_all().await?;
                Ok(pages.iter().map(|p| p.id).collect())
            }
        }
    }

    // ── Internal: Structural Clustering ────────────────────────────────

    /// Cluster blocks by structural affinity: page grouping + keyword match + tag overlap.
    /// NO embeddings, NO LLM calls.
    fn structural_cluster(
        &self,
        topic: &str,
        blocks: &[quilt_domain::entities::Block],
    ) -> Vec<TopicCluster> {
        let topic_lower = topic.to_lowercase();
        let topic_keywords: Vec<&str> = topic_lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 2)
            .collect();

        // Group blocks by page
        let mut page_groups: std::collections::HashMap<Uuid, Vec<Uuid>> =
            std::collections::HashMap::new();
        let mut block_page_map: std::collections::HashMap<Uuid, Uuid> =
            std::collections::HashMap::new();
        for block in blocks {
            page_groups.entry(block.page_id).or_default().push(block.id);
            block_page_map.insert(block.id, block.page_id);
        }

        let mut clusters: Vec<TopicCluster> = Vec::new();

        // Primary cluster: blocks whose content/tags match topic keywords
        let mut primary_block_ids = Vec::new();
        let mut secondary_block_ids = Vec::new();

        for block in blocks {
            let content_lower = block.content.to_lowercase();
            let matches: usize = topic_keywords
                .iter()
                .filter(|kw| content_lower.contains(*kw))
                .count();

            if matches >= topic_keywords.len().min(2) {
                primary_block_ids.push(block.id);
            } else if matches > 0 {
                secondary_block_ids.push(block.id);
            }
        }

        if !primary_block_ids.is_empty() {
            clusters.push(TopicCluster {
                label: topic.to_string(),
                summary: format!(
                    "Blocks related to '{}' — {} relevant blocks found",
                    topic,
                    primary_block_ids.len()
                ),
                block_ids: primary_block_ids.clone(),
                relevance: 0.9,
            });
        }

        if !secondary_block_ids.is_empty() {
            clusters.push(TopicCluster {
                label: format!("Related to {}", topic),
                summary: format!(
                    "Blocks with partial relevance to '{}' — {} blocks",
                    topic,
                    secondary_block_ids.len()
                ),
                block_ids: secondary_block_ids,
                relevance: 0.6,
            });
        }

        // Per-page clusters for pages with many relevant blocks
        for (page_id, block_ids) in page_groups {
            let relevant: Vec<Uuid> = block_ids
                .iter()
                .filter(|id| primary_block_ids.contains(id))
                .copied()
                .collect();

            if relevant.len() >= 3 {
                clusters.push(TopicCluster {
                    label: format!("Page group ({} blocks)", relevant.len()),
                    summary: format!("Blocks from page {} related to '{}'", page_id, topic),
                    block_ids: relevant,
                    relevance: 0.7,
                });
            }
        }

        // Sort by relevance descending
        clusters.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        clusters
    }

    // ── Internal: Citations ─────────────────────────────────────────────

    async fn collect_citations(&self, block_ids: &[Uuid]) -> Result<Vec<Citation>, TreeRagError> {
        let mut citations = Vec::new();
        for &block_id in block_ids {
            if let Ok(Some(block)) = self.block_repo.get_by_id(block_id).await {
                let page_name = self
                    .page_repo
                    .get_by_id(block.page_id)
                    .await?
                    .map(|p| p.name)
                    .unwrap_or_default();

                let snippet = block.content.chars().take(100).collect::<String>();
                citations.push(Citation {
                    section: String::new(),
                    block_id,
                    page_name,
                    snippet,
                });
            }
        }
        citations.sort_by(|a, b| a.page_name.cmp(&b.page_name));
        citations.dedup_by(|a, b| a.block_id == b.block_id);
        Ok(citations)
    }

    // ── Internal: Tree filtering ───────────────────────────────────────

    fn filter_tree_node(&self, node: &TreeNode, query: &str) -> TreeNode {
        let matches = node.title.to_lowercase().contains(query)
            || node.summary.to_lowercase().contains(query);

        let filtered_children: Vec<TreeNode> = node
            .children
            .iter()
            .map(|child| self.filter_tree_node(child, query))
            .filter(|n| !n.children.is_empty() || n.title.to_lowercase().contains(query))
            .collect();

        if matches || !filtered_children.is_empty() {
            TreeNode {
                block_id: node.block_id,
                page_name: node.page_name.clone(),
                title: node.title.clone(),
                summary: node.summary.clone(),
                children_count: filtered_children.len(),
                children: filtered_children,
            }
        } else {
            TreeNode {
                block_id: node.block_id,
                page_name: node.page_name.clone(),
                title: node.title.clone(),
                summary: node.summary.clone(),
                children_count: 0,
                children: vec![],
            }
        }
    }

    fn count_nodes(&self, node: &TreeNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|c| self.count_nodes(c))
            .sum::<usize>()
    }

    fn hash_content(content: &str) -> Vec<u8> {
        Sha256::digest(content.as_bytes()).to_vec()
    }
}

// ── Tree builder (recursive, avoiding async recursion issues) ──────────

fn build_subtree<'a>(
    summary_repo: &'a (dyn BlockSummaryRepository + Sync),
    parent_id: Option<Uuid>,
    all_blocks: &'a [quilt_domain::entities::Block],
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<TreeNode, TreeRagError>> + Send + 'a>>
{
    Box::pin(async move {
        let children: Vec<&quilt_domain::entities::Block> = all_blocks
            .iter()
            .filter(|b| b.parent_id == parent_id)
            .collect();

        if children.is_empty() && parent_id.is_some() {
            if let Some(block) = all_blocks.iter().find(|b| b.id == parent_id.unwrap()) {
                let summary = summary_repo
                    .get(block.id)
                    .await?
                    .map(|s| s.summary)
                    .unwrap_or_default();
                return Ok(TreeNode {
                    block_id: block.id,
                    page_name: String::new(),
                    title: block.content.lines().next().unwrap_or("").to_string(),
                    summary,
                    children_count: 0,
                    children: vec![],
                });
            }
        }

        let mut nodes = Vec::new();
        for child in &children {
            let summary = summary_repo
                .get(child.id)
                .await?
                .map(|s| s.summary)
                .unwrap_or_default();
            let grand_children = build_subtree(summary_repo, Some(child.id), all_blocks).await;
            let sub_nodes = match grand_children {
                Ok(n) if n.children_count > 0 || !n.summary.is_empty() => vec![n],
                _ => vec![],
            };

            nodes.push(TreeNode {
                block_id: child.id,
                page_name: String::new(),
                title: child.content.lines().next().unwrap_or("").to_string(),
                summary,
                children_count: sub_nodes.len(),
                children: sub_nodes,
            });
        }

        if parent_id.is_none() {
            return Ok(TreeNode {
                block_id: Uuid::new_v4(),
                page_name: String::new(),
                title: "root".to_string(),
                summary: String::new(),
                children_count: nodes.len(),
                children: nodes,
            });
        }

        nodes
            .into_iter()
            .next()
            .ok_or_else(|| TreeRagError::NoBlocksFound("build_subtree".to_string()))
    })
}
