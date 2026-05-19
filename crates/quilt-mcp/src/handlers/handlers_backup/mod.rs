//! Handler traits for MCP tool execution.
//!
//! These traits decouple tool logic from [`McpServer`](super::McpServer),
//! reducing the server's field count and enabling focused testing of each
//! tool domain (blocks, pages, search, cognitive).

use async_trait::async_trait;
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;

/// Result type for handler methods.
/// Handlers return a JSON string representation of their result.
pub type HandlerResult = Result<String, String>;

/// Parameters for creating a block.
#[derive(Debug, serde::Deserialize)]
pub struct CreateBlockParams {
    pub page_name: String,
    pub content: String,
    pub parent_id: Option<Uuid>,
    pub marker: Option<String>,
}

/// Block with its children for tree representation.
#[derive(Debug, serde::Serialize)]
pub struct BlockWithChildren {
    pub id: Uuid,
    pub content: String,
    pub children: Vec<BlockWithChildren>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
}

/// Result for search operations.
#[derive(Debug, serde::Serialize)]
pub struct SearchResult {
    pub block_id: Uuid,
    pub page_name: String,
    pub snippet: String,
    pub score: f32,
}

/// Parameters for search operations.
#[derive(Debug, serde::Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub limit: Option<usize>,
}

/// Parameters for query (DSL) operations.
#[derive(Debug, serde::Deserialize)]
pub struct QueryParams {
    pub dsl: String,
    pub limit: Option<usize>,
}

/// Parameters for getting page blocks.
#[derive(Debug, serde::Deserialize)]
pub struct PageBlocksParams {
    pub page_name: String,
    pub format: Option<String>,
}

/// Page information for listing.
#[derive(Debug, serde::Serialize)]
pub struct PageInfo {
    pub id: Uuid,
    pub name: String,
    pub journal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_day: Option<i64>,
}

/// Parameters for journal page operations.
#[derive(Debug, serde::Deserialize)]
pub struct JournalParams {
    pub date: String,
}

/// Parameters for task creation.
#[derive(Debug, serde::Deserialize)]
pub struct CreateTaskParams {
    pub page_name: String,
    pub content: String,
    pub deadline: Option<String>,
    pub priority: Option<String>,
}

/// Block handler trait for block-related MCP tools.
///
/// Provides access to block CRUD operations and tree traversal.
#[async_trait]
pub trait BlockHandler: Send + Sync {
    /// Create a new block on a page.
    async fn create_block(&self, params: CreateBlockParams) -> HandlerResult;

    /// Get a block with all its children recursively.
    async fn get_block_tree(&self, block_id: Uuid) -> HandlerResult;

    /// Delete a block (soft-delete to recycle bin).
    async fn delete_block(&self, block_id: Uuid) -> HandlerResult;

    /// Restore a soft-deleted block from the recycle bin.
    async fn restore_block(&self, block_id: Uuid) -> HandlerResult;

    /// Link one block to another (create a reference).
    async fn link_blocks(&self, source_id: Uuid, target_id: Uuid) -> HandlerResult;

    /// Get all blocks that reference a given block.
    async fn get_backlinks(&self, block_id: Uuid) -> HandlerResult;

    /// Get all soft-deleted blocks in the recycle bin.
    async fn recycle_bin(&self) -> HandlerResult;

    /// List orphan pages (pages with no blocks).
    async fn orphan_pages(&self) -> HandlerResult;

    /// List orphan blocks (blocks not attached to any page).
    async fn orphan_blocks(&self) -> HandlerResult;
}

/// Page handler trait for page-related MCP tools.
///
/// Provides access to page operations and page content retrieval.
#[async_trait]
pub trait PageHandler: Send + Sync {
    /// List all pages in the graph.
    async fn list_pages(&self) -> HandlerResult;

    /// Get all blocks on a page.
    async fn get_page_blocks(&self, params: PageBlocksParams) -> HandlerResult;

    /// Get or create a journal page for a specific date.
    async fn get_journal(&self, params: JournalParams) -> HandlerResult;

    /// Create a task (block with marker) on a page.
    async fn create_task(&self, params: CreateTaskParams) -> HandlerResult;
}

/// Search handler trait for search and query operations.
///
/// Provides full-text search and DSL query execution.
#[async_trait]
pub trait SearchHandler: Send + Sync {
    /// Full-text search across all pages and blocks.
    async fn search(&self, params: SearchParams) -> HandlerResult;

    /// Execute a Logseq DSL query against the graph.
    async fn query(&self, params: QueryParams) -> HandlerResult;

    /// Rebuild the search index.
    async fn rebuild_index(&self, mode: Option<String>, since: Option<String>) -> HandlerResult;

    /// Check the health of the search index.
    async fn index_health(&self) -> HandlerResult;
}

/// Deep link handler trait for deep link operations.
///
/// Manages deep links between blocks, pages, and external URLs.
#[async_trait]
pub trait DeepLinkHandler: Send + Sync {
    /// Create a deep link.
    async fn create_deep_link(
        &self,
        source_id: Uuid,
        source_type: String,
        target_id: Option<Uuid>,
        target_page_name: Option<String>,
        link_type: String,
        external_url: Option<String>,
        link_text: Option<String>,
        context: Option<String>,
    ) -> HandlerResult;

    /// Get deep links from a source or to a target.
    async fn get_deep_links(
        &self,
        source_id: Option<Uuid>,
        source_type: Option<String>,
        target_id: Option<Uuid>,
        link_type: Option<String>,
        limit: Option<usize>,
    ) -> HandlerResult;

    /// Delete a deep link by its ID.
    async fn delete_deep_link(&self, id: Uuid) -> HandlerResult;
}

/// Settings handler trait for user settings operations.
#[async_trait]
pub trait SettingsHandler: Send + Sync {
    /// Get user settings.
    async fn get_settings(&self) -> HandlerResult;

    /// Update user settings.
    async fn update_settings(&self, settings: serde_json::Value) -> HandlerResult;
}

/// Daily summary handler trait.
#[async_trait]
pub trait DailySummaryHandler: Send + Sync {
    /// Get daily summary for a date.
    async fn get_daily_summary(&self, date: Option<String>) -> HandlerResult;
}

// Re-export all handler implementations
pub mod block;
pub mod cognitive;
pub mod deep_link;
pub mod page;
pub mod search;
pub mod settings;

pub use block::DefaultBlockHandler;
pub use cognitive::DefaultCognitiveHandler;
pub use deep_link::DefaultDeepLinkHandler;
pub use page::DefaultPageHandler;
pub use search::DefaultSearchHandler;
pub use settings::DefaultSettingsHandler;

/// A composed handler container that aggregates all handler traits.
///
/// This allows [`McpServer`](super::McpServer) to hold a single field
/// per domain instead of 20+ individual fields.
#[derive(Clone)]
pub struct HandlerContainer {
    pub block: Arc<dyn BlockHandler>,
    pub page: Arc<dyn PageHandler>,
    pub search: Arc<dyn SearchHandler>,
    pub deep_link: Arc<dyn DeepLinkHandler>,
    pub settings: Arc<dyn SettingsHandler>,
    pub daily_summary: Arc<dyn DailySummaryHandler>,
    // Cognitive services - stored as Arc for Option<CognitiveHandler>
    pub cognitive: Option<Arc<dyn CognitiveHandler>>,
}

/// Cognitive handler trait for cognitive engine operations.
///
/// This trait is separate because all cognitive engines are optional -
/// the server configures only the engines that are available.
#[async_trait]
pub trait CognitiveHandler: Send + Sync {
    /// Analyze a page's cognitive structure.
    async fn cognitive_mirror(&self, page_name: &str) -> HandlerResult;

    /// Find unexpected connections between knowledge blocks.
    async fn serendipity(
        &self,
        since: Option<String>,
        limit: Option<usize>,
        min_confidence: Option<f32>,
    ) -> HandlerResult;

    /// Query the agent memory store.
    async fn agent_memory(&self, domain: &str, query: Option<&str>, limit: Option<usize>) -> HandlerResult;

    /// Map argument structure in a page.
    async fn argument_map(&self, page_name: &str, max_depth: Option<usize>) -> HandlerResult;

    /// Get the mental model for an agent from journal entries.
    async fn mental_model(&self, agent_id: &str, time_window: Option<String>) -> HandlerResult;

    /// Explore counterfactual scenarios.
    async fn counterfactual_explore(&self, scenario: &str, decision_point: &str) -> HandlerResult;

    /// Track how knowledge evolves over time.
    async fn knowledge_evolution(&self, topic: &str, timespan_days: Option<usize>) -> HandlerResult;

    /// Get a daily cognitive briefing.
    async fn morning_briefing(&self) -> HandlerResult;

    /// Explore a topic in the knowledge graph.
    async fn explore_topic(&self, topic: &str, scope: Option<String>) -> HandlerResult;

    /// Build a navigable tree from a page's blocks.
    async fn build_tree(&self, page_id: Uuid) -> HandlerResult;

    /// Query/filter a TreeIndex.
    async fn query_tree(&self, page_id: Uuid, query: &str) -> HandlerResult;

    /// Assemble a report from sections.
    async fn assemble_report(
        &self,
        title: &str,
        description: &str,
        sections: serde_json::Value,
        render_pdf: bool,
    ) -> HandlerResult;

    /// Get tree RAG status.
    async fn tree_status(&self) -> HandlerResult;

    /// Save a block summary.
    async fn save_block_summary(&self, block_id: Uuid, summary: &str) -> HandlerResult;

    /// Rebuild the tree RAG index.
    async fn rebuild_tree_index(&self, scope: Option<String>) -> HandlerResult;

    /// Schedule a recurring background task.
    async fn schedule_task(&self, name: &str, cron_expr: &str, task_type: &str) -> HandlerResult;

    /// List all scheduled tasks.
    async fn list_tasks(&self) -> HandlerResult;
}
