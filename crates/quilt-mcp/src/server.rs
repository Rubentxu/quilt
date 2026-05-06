//! MCP Server implementation for Quilt
//!
//! Implements the Model Context Protocol for AI agent integration.
//! Wired to real repositories, search, and query services.

use crate::errors::McpError;
use crate::notifications::{
    BlockChangedEvent, ChangeType, Notification, NotificationEvent, NotificationParams,
    PageCreatedEvent,
};
use crate::resources::Resource;
use crate::tools::Tool;
use quilt_application::query_service::QueryService;
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository, TagRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, TaskMarker, Uuid};
use quilt_search::{SearchIndexManager, SearchService};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::instrument;

// ── Request / Response types ───────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize { params: InitializeParams },
    #[serde(rename = "tools/list")]
    ListTools,
    #[serde(rename = "tools/call")]
    CallTool { params: CallToolParams },
    #[serde(rename = "resources/list")]
    ListResources,
    #[serde(rename = "resources/read")]
    ReadResource { params: ReadResourceParams },
    #[serde(rename = "notifications_enabled")]
    EnableNotifications,
}

#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<Roots>,
    pub sampling: Option<Sampling>,
}

#[derive(Debug, Deserialize)]
pub struct Roots {
    pub list: bool,
}

#[derive(Debug, Deserialize)]
pub struct Sampling {}

#[derive(Debug, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
pub enum McpResponse {
    #[serde(rename = "initialize")]
    Initialize(InitializeResult),
    #[serde(rename = "tools/list")]
    ToolsList(ToolsListResult),
    #[serde(rename = "tools/call")]
    ToolsCall(ToolsCallResult),
    #[serde(rename = "resources/list")]
    ResourcesList(ResourcesListResult),
    #[serde(rename = "resources/read")]
    ResourcesRead(ResourceReadResult),
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolCapabilities,
    pub resources: ResourceCapabilities,
    pub notifications: NotificationCapabilities,
}

#[derive(Debug, Serialize)]
pub struct ToolCapabilities {
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ResourceCapabilities {
    pub subscribe: bool,
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationCapabilities {}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize)]
pub struct ToolsCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: Resource },
}

#[derive(Debug, Serialize)]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize)]
pub struct ResourceReadResult {
    pub contents: Vec<ResourceContent>,
}

#[derive(Debug, Serialize)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
}

// ── Cognitive Engine Status DTO ───────────────────────────────────────

/// Status of all cognitive engines in the MCP server.
///
/// Returned by [`McpServer::cognitive_engine_status`] to allow Tauri commands
/// to check availability without triggering engine initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CognitiveEngineStatus {
    /// Whether the CognitiveMirror engine is available.
    pub cognitive_mirror: bool,
    /// Whether the SerendipityEngine is available.
    pub serendipity_engine: bool,
    /// Whether the AgentMemory is available.
    pub agent_memory: bool,
    /// Whether the ArgumentCartographer is available.
    pub argument_cartographer: bool,
    /// Whether the MentalModelGardener is available.
    pub mental_model_gardener: bool,
    /// Whether the CounterfactualExplorer is available.
    pub counterfactual_explorer: bool,
    /// Whether the KnowledgeEvolutionTracker is available.
    pub knowledge_evolution_tracker: bool,
}

// ── McpServer ──────────────────────────────────────────────────────────

/// MCP server for Quilt knowledge graph operations.
///
/// This server implements the Model Context Protocol, providing AI agents
/// with tools to query and modify the Quilt knowledge graph.
///
/// # Type Parameters
///
/// The server uses trait objects for repositories, allowing any
/// implementation of the repository traits to be used.
///
/// # Example
///
/// ```
/// use quilt_mcp::McpServer;
/// use quilt_search::SearchService;
/// use std::sync::Arc;
///
/// async {
///     // let server = McpServer::new(
///     //     Arc::new(block_repo),
///     //     Arc::new(page_repo),
///     //     Arc::new(tag_repo),
///     //     Arc::new(search),
///     // );
/// };
/// ```
pub struct McpServer {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    tag_repo: Arc<dyn TagRepository>,
    search_service: Arc<SearchService>,
    search_index: Option<Arc<SearchIndexManager>>,
    query_service: QueryService,
    pool: Option<sqlx::SqlitePool>,
    notification_sender: broadcast::Sender<Notification>,
    // Cognitive services (optional)
    #[allow(dead_code)]
    cognitive_mirror: Option<Arc<quilt_cognitive::CognitiveMirror>>,
    #[allow(dead_code)]
    serendipity_engine: Option<Arc<quilt_cognitive::SerendipityEngine>>,
    #[allow(dead_code)]
    agent_memory: Option<Arc<quilt_cognitive::AgentMemory>>,
    #[allow(dead_code)]
    argument_cartographer: Option<Arc<quilt_cognitive::ArgumentCartographer>>,
    #[allow(dead_code)]
    mental_model_gardener: Option<Arc<quilt_cognitive::MentalModelGardener>>,
    #[allow(dead_code)]
    counterfactual_explorer: Option<Arc<quilt_cognitive::CounterfactualExplorer>>,
    #[allow(dead_code)]
    knowledge_evolution_tracker: Option<Arc<quilt_cognitive::KnowledgeEvolutionTracker>>,
}

impl McpServer {
    /// Creates a new MCP server with the given repository and service dependencies.
    ///
    /// # Arguments
    ///
    /// * `block_repo` - Repository for block persistence
    /// * `page_repo` - Repository for page persistence
    /// * `tag_repo` - Repository for tag management
    /// * `search_service` - Service for full-text search
    ///
    /// # Example
    ///
    /// ```
    /// use quilt_mcp::McpServer;
    /// use quilt_search::SearchService;
    /// use std::sync::Arc;
    ///
    /// async {
    ///     // let server = McpServer::new(block_repo, page_repo, tag_repo, search_service);
    /// };
    /// ```
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        tag_repo: Arc<dyn TagRepository>,
        search_service: Arc<SearchService>,
    ) -> Self {
        let (notification_sender, _) = broadcast::channel(100);
        Self {
            block_repo,
            page_repo,
            tag_repo,
            search_service,
            search_index: None,
            query_service: QueryService::new(),
            pool: None,
            notification_sender,
            cognitive_mirror: None,
            serendipity_engine: None,
            agent_memory: None,
            argument_cartographer: None,
            mental_model_gardener: None,
            counterfactual_explorer: None,
            knowledge_evolution_tracker: None,
        }
    }

    /// Set the SQLite connection pool for query execution.
    ///
    /// When set, `logseq_query` will execute queries against the database
    /// and return actual block results instead of just the SQL plan.
    pub fn with_pool(mut self, pool: sqlx::SqlitePool) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Set the search index manager for index maintenance tools.
    ///
    /// When set, `logseq_rebuild_index` and `logseq_index_health` tools
    /// become available for managing the FTS5 index.
    pub fn with_search_index(mut self, search_index: Arc<SearchIndexManager>) -> Self {
        self.search_index = Some(search_index);
        self
    }

    /// Add cognitive services to the MCP server.
    ///
    /// All cognitive services are optional — the server works without them,
    /// but cognitive tools and resources will only appear when their corresponding
    /// engine is provided.
    pub fn with_cognitive(
        self,
        cognitive_mirror: Option<Arc<quilt_cognitive::CognitiveMirror>>,
        serendipity_engine: Option<Arc<quilt_cognitive::SerendipityEngine>>,
        agent_memory: Option<Arc<quilt_cognitive::AgentMemory>>,
        argument_cartographer: Option<Arc<quilt_cognitive::ArgumentCartographer>>,
        mental_model_gardener: Option<Arc<quilt_cognitive::MentalModelGardener>>,
        counterfactual_explorer: Option<Arc<quilt_cognitive::CounterfactualExplorer>>,
        knowledge_evolution_tracker: Option<Arc<quilt_cognitive::KnowledgeEvolutionTracker>>,
    ) -> Self {
        Self {
            cognitive_mirror,
            serendipity_engine,
            agent_memory,
            argument_cartographer,
            mental_model_gardener,
            counterfactual_explorer,
            knowledge_evolution_tracker,
            ..self
        }
    }

    /// Subscribe to notifications.
    ///
    /// Returns a receiver that will receive all subsequent notifications.
    /// The receiver should be used in a separate task to process notifications.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut receiver = server.subscribe();
    /// tokio::spawn(async move {
    ///     while let Ok(notification) = receiver.recv().await {
    ///         println!("Received: {:?}", notification);
    ///     }
    /// });
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.notification_sender.subscribe()
    }

    /// Emit a block changed notification.
    ///
    /// # Arguments
    ///
    /// * `block_id` - The UUID of the block that changed
    /// * `change_type` - The type of change (Created, Updated, Deleted)
    #[instrument(skip(self))]
    pub fn emit_block_changed(&self, block_id: Uuid, change_type: ChangeType) {
        let notification = Notification {
            method: "notifications/block_changed".to_string(),
            params: NotificationParams {
                event: NotificationEvent::BlockChanged(BlockChangedEvent {
                    block_id: block_id.to_string(),
                    change_type,
                }),
            },
        };
        // Ignore send error - if no receivers, that's ok
        let _ = self.notification_sender.send(notification);
    }

    /// Emit a page created notification.
    ///
    /// # Arguments
    ///
    /// * `page_id` - The UUID of the page that was created
    /// * `page_name` - The name of the page that was created
    #[instrument(skip(self))]
    pub fn emit_page_created(&self, page_id: Uuid, page_name: String) {
        let notification = Notification {
            method: "notifications/page_created".to_string(),
            params: NotificationParams {
                event: NotificationEvent::PageCreated(PageCreatedEvent {
                    page_id: page_id.to_string(),
                    page_name,
                }),
            },
        };
        // Ignore send error - if no receivers, that's ok
        let _ = self.notification_sender.send(notification);
    }

    // ── Cognitive Engine Delegation Methods ──────────────────────
    // These public methods delegate to internal tool implementations but return
    // typed Results instead of JSON strings, providing a clean API for Tauri commands.

    /// Returns the availability status of all cognitive engines.
    ///
    /// This method is cheap to call — it only checks if the engine `Arc`s are `Some`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let status = self.cognitive_engine_status();
    /// if status.cognitive_mirror {
    ///     // Engine is available
    /// }
    /// ```
    #[instrument(skip(self))]
    pub fn cognitive_engine_status(&self) -> CognitiveEngineStatus {
        CognitiveEngineStatus {
            cognitive_mirror: self.cognitive_mirror.is_some(),
            serendipity_engine: self.serendipity_engine.is_some(),
            agent_memory: self.agent_memory.is_some(),
            argument_cartographer: self.argument_cartographer.is_some(),
            mental_model_gardener: self.mental_model_gardener.is_some(),
            counterfactual_explorer: self.counterfactual_explorer.is_some(),
            knowledge_evolution_tracker: self.knowledge_evolution_tracker.is_some(),
        }
    }

    /// Analyze a page's cognitive structure (clusters, frontiers, gaps).
    ///
    /// # Arguments
    ///
    /// * `page_name` - Name of the page to analyze
    ///
    /// # Returns
    ///
    /// Returns the cognitive map as JSON value on success, or an error message if:
    /// - The CognitiveMirror engine is not configured
    /// - The page does not exist
    #[instrument(skip(self))]
    pub async fn cognitive_mirror_analysis(
        &self,
        page_name: &str,
    ) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({ "page_name": page_name });
        let json_string = self.tool_cognitive_mirror(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Find unexpected connections between knowledge blocks.
    ///
    /// # Arguments
    ///
    /// * `since` - Optional timestamp to filter connections discovered after this time
    /// * `limit` - Maximum number of connections to return (default: 20)
    /// * `min_confidence` - Minimum confidence score (0.0-1.0, default: 0.3)
    #[instrument(skip(self))]
    pub async fn serendipity_query(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
        limit: usize,
        min_confidence: f32,
    ) -> Result<serde_json::Value, String> {
        let since_str = since.map(|dt| dt.to_rfc3339());
        let args = serde_json::json!({
            "since": since_str,
            "limit": limit,
            "min_confidence": min_confidence
        });
        let json_string = self.tool_serendipity(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Query the agent memory store.
    ///
    /// # Arguments
    ///
    /// * `domain` - Memory domain to query (agent ID)
    /// * `query` - Optional free-text search query
    /// * `limit` - Maximum number of results (default: 10)
    #[instrument(skip(self))]
    pub async fn agent_memory_query(
        &self,
        domain: &str,
        query: Option<&str>,
        limit: usize,
    ) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({
            "domain": domain,
            "query": query,
            "limit": limit
        });
        let json_string = self.tool_agent_memory(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Map the argument structure in a page.
    ///
    /// # Arguments
    ///
    /// * `page_name` - Name of the page to analyze
    #[instrument(skip(self))]
    pub async fn argument_map_page(&self, page_name: &str) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({ "page_name": page_name });
        let json_string = self.tool_argument_map(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Build a mental model for an agent from journal entries.
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent ID (journal prefix)
    #[instrument(skip(self))]
    pub async fn mental_model_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({ "agent_id": agent_id });
        let json_string = self.tool_mental_model(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Explore counterfactual scenarios and alternative branches.
    ///
    /// # Arguments
    ///
    /// * `scenario` - The scenario to explore
    /// * `decision_point` - The decision point to analyze
    #[instrument(skip(self))]
    pub async fn counterfactual_explore(
        &self,
        scenario: &str,
        decision_point: &str,
    ) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({
            "scenario": scenario,
            "decision_point": decision_point
        });
        let json_string = self.tool_counterfactual_explore(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Track how knowledge and beliefs evolve over time.
    ///
    /// # Arguments
    ///
    /// * `topic` - Topic to track
    /// * `timespan_days` - Time window in days (default: 30)
    #[instrument(skip(self))]
    pub async fn knowledge_evolution_track(
        &self,
        topic: &str,
        timespan_days: u32,
    ) -> Result<serde_json::Value, String> {
        let args = serde_json::json!({
            "topic": topic,
            "timespan_days": timespan_days
        });
        let json_string = self.tool_knowledge_evolution(&args).await?;
        serde_json::from_str(&json_string).map_err(|e| format!("Failed to parse response: {}", e))
    }

    // ── Tool definitions ──────────────────────────────────────────

    fn tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool {
                name: "logseq_query".to_string(),
                description: "Execute a Logseq DSL query against the current graph".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "dsl": { "type": "string", "description": "DSL query string" },
                        "limit": { "type": "integer", "description": "Max results", "default": 100 }
                    },
                    "required": ["dsl"]
                }),
            },
            Tool {
                name: "logseq_create_block".to_string(),
                description: "Create a new block on a page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "content": { "type": "string", "description": "Block content (markdown)" },
                        "parent_id": { "type": "string", "description": "Parent block UUID (optional)" },
                        "marker": { "type": "string", "description": "Task marker: now, later, todo, done, cancelled (optional)" }
                    },
                    "required": ["page_name", "content"]
                }),
            },
            Tool {
                name: "logseq_search".to_string(),
                description: "Full-text search across all pages and blocks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "limit": { "type": "integer", "description": "Max results", "default": 50 }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "logseq_get_block_tree".to_string(),
                description: "Get a block with all its children recursively".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Block UUID (root)" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "logseq_get_page_blocks".to_string(),
                description: "Get all blocks on a page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "format": { "type": "string", "description": "Format: markdown or org", "default": "markdown" }
                    },
                    "required": ["page_name"]
                }),
            },
            Tool {
                name: "logseq_list_pages".to_string(),
                description: "List all pages in the graph".to_string(),
                input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            },
            Tool {
                name: "logseq_get_journal".to_string(),
                description: "Get or create a journal page for a specific date".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "description": "Date in YYYY-MM-DD format" }
                    },
                    "required": ["date"]
                }),
            },
            Tool {
                name: "logseq_create_task".to_string(),
                description: "Create a task with optional deadline".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "content": { "type": "string", "description": "Task content" },
                        "deadline": { "type": "string", "description": "Deadline date YYYY-MM-DD (optional)" },
                        "priority": { "type": "string", "description": "Priority: a, b, or c (optional)" }
                    },
                    "required": ["page_name", "content"]
                }),
            },
            Tool {
                name: "logseq_link_blocks".to_string(),
                description: "Link one block to another (create a reference)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": { "type": "string", "description": "Source block UUID" },
                        "target_id": { "type": "string", "description": "Target block UUID" }
                    },
                    "required": ["source_id", "target_id"]
                }),
            },
            Tool {
                name: "logseq_get_backlinks".to_string(),
                description: "Get all backlinks pointing to a block".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Target block UUID" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "logseq_delete_block".to_string(),
                description: "Delete a block (soft-delete to recycle bin)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "block_id": { "type": "string", "description": "Block UUID" }
                    },
                    "required": ["block_id"]
                }),
            },
            Tool {
                name: "logseq_rebuild_index".to_string(),
                description: "Rebuild the full-text search index. Use 'incremental' mode to re-index only blocks updated since a timestamp, or 'full' mode to rebuild everything.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "mode": { "type": "string", "description": "Rebuild mode: 'full' or 'incremental'", "default": "full" },
                        "since": { "type": "string", "description": "ISO timestamp for incremental mode (e.g., '2024-01-01T00:00:00Z')" }
                    }
                }),
            },
            Tool {
                name: "logseq_index_health".to_string(),
                description: "Check the health of the search index. Returns FTS entry count, block count, and whether they are in sync.".to_string(),
                input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            },
        ];

        // Add cognitive tools if their engines are configured
        if self.cognitive_mirror.is_some() {
            tools.push(Tool {
                name: "logseq_cognitive_mirror".to_string(),
                description: "Analyze a page's cognitive structure (clusters, frontiers, gaps)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name to analyze" }
                    },
                    "required": ["page_name"]
                }),
            });
        }

        if self.serendipity_engine.is_some() {
            tools.push(Tool {
                name: "logseq_serendipity".to_string(),
                description: "Find unexpected connections between knowledge blocks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "since": { "type": "string", "description": "ISO timestamp filter (optional)" },
                        "limit": { "type": "integer", "description": "Max results", "default": 20 },
                        "min_confidence": { "type": "number", "description": "Min confidence 0.0-1.0", "default": 0.3 }
                    }
                }),
            });
        }

        if self.agent_memory.is_some() {
            tools.push(Tool {
                name: "logseq_agent_memory".to_string(),
                description: "Query the agent memory store".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "domain": { "type": "string", "description": "Memory domain" },
                        "query": { "type": "string", "description": "FTS query (optional)" },
                        "limit": { "type": "integer", "description": "Max results", "default": 10 }
                    },
                    "required": ["domain"]
                }),
            });
        }

        if self.argument_cartographer.is_some() {
            tools.push(Tool {
                name: "logseq_argument_map".to_string(),
                description: "Map argument structure in a page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name to analyze" },
                        "max_depth": { "type": "integer", "description": "Max traversal depth", "default": 5 }
                    },
                    "required": ["page_name"]
                }),
            });
        }

        if self.mental_model_gardener.is_some() {
            tools.push(Tool {
                name: "logseq_mental_model".to_string(),
                description: "Get the mental model for an agent from journal entries".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "agent_id": { "type": "string", "description": "Agent ID (journal prefix)" },
                        "time_window": { "type": "string", "description": "Time window in days (optional)" }
                    },
                    "required": ["agent_id"]
                }),
            });
        }

        if self.counterfactual_explorer.is_some() {
            tools.push(Tool {
                name: "logseq_counterfactual".to_string(),
                description: "Explore counterfactual scenarios and alternative branches".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "scenario": { "type": "string", "description": "The scenario to explore" },
                        "decision_point": { "type": "string", "description": "The decision point to analyze" }
                    },
                    "required": ["scenario", "decision_point"]
                }),
            });
        }

        if self.knowledge_evolution_tracker.is_some() {
            tools.push(Tool {
                name: "logseq_knowledge_evolution".to_string(),
                description: "Track how knowledge and beliefs evolve over time".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "Topic to track" },
                        "timespan_days": { "type": "integer", "description": "Time window in days", "default": 30 }
                    },
                    "required": ["topic"]
                }),
            });
        }

        tools
    }

    fn resources(&self) -> Vec<Resource> {
        let mut resources = vec![
            Resource {
                uri: "logseq://graph".to_string(),
                name: "Current Graph".to_string(),
                description: "Full graph data with blocks, pages, and connections".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "logseq://pages".to_string(),
                name: "All Pages".to_string(),
                description: "List of all pages in the graph".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "logseq://journals".to_string(),
                name: "Journal Pages".to_string(),
                description: "List of all journal pages".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "logseq://tags".to_string(),
                name: "All Tags".to_string(),
                description: "List of all tags with usage counts".to_string(),
                mime_type: "application/json".to_string(),
            },
        ];

        // Add cognitive resources if their engines are configured
        if self.cognitive_mirror.is_some() {
            resources.push(Resource {
                uri: "logseq://cognitive/map".to_string(),
                name: "Cognitive Map".to_string(),
                description: "Overall cognitive analysis summary".to_string(),
                mime_type: "application/json".to_string(),
            });
        }

        if self.serendipity_engine.is_some() {
            resources.push(Resource {
                uri: "logseq://cognitive/serendipity".to_string(),
                name: "Serendipity Discoveries".to_string(),
                description: "Recent unexpected connections discovered".to_string(),
                mime_type: "application/json".to_string(),
            });
        }

        if self.argument_cartographer.is_some() {
            resources.push(Resource {
                uri: "logseq://cognitive/arguments/{page}".to_string(),
                name: "Argument Map".to_string(),
                description: "Argument structure for a specific page".to_string(),
                mime_type: "application/json".to_string(),
            });
        }

        if self.mental_model_gardener.is_some() {
            resources.push(Resource {
                uri: "logseq://cognitive/mental-models".to_string(),
                name: "Mental Model Garden".to_string(),
                description: "Mental model beliefs and evolution".to_string(),
                mime_type: "application/json".to_string(),
            });
        }

        resources
    }

    // ── Request handler ───────────────────────────────────────────

    #[instrument(skip(self))]
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request {
            McpRequest::Initialize { params: _ } => McpResponse::Initialize(InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: ToolCapabilities {
                        list_changed: false,
                    },
                    resources: ResourceCapabilities {
                        subscribe: false,
                        list_changed: false,
                    },
                    notifications: NotificationCapabilities {},
                },
                server_info: ServerInfo {
                    name: "quilt-mcp".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            }),
            McpRequest::ListTools => McpResponse::ToolsList(ToolsListResult {
                tools: self.tools(),
            }),
            McpRequest::CallTool { params } => self.execute_tool(params).await,
            McpRequest::ListResources => McpResponse::ResourcesList(ResourcesListResult {
                resources: self.resources(),
            }),
            McpRequest::ReadResource { params } => self.read_resource(params).await,
            McpRequest::EnableNotifications => McpResponse::Initialize(InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: ToolCapabilities {
                        list_changed: false,
                    },
                    resources: ResourceCapabilities {
                        subscribe: false,
                        list_changed: false,
                    },
                    notifications: NotificationCapabilities {},
                },
                server_info: ServerInfo {
                    name: "quilt-mcp".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            }),
        }
    }

    // ── Tool execution ────────────────────────────────────────────

    #[instrument(skip(self))]
    async fn execute_tool(&self, params: CallToolParams) -> McpResponse {
        let result = match params.name.as_str() {
            "logseq_query" => self.tool_query(&params.arguments).await,
            "logseq_create_block" => self.tool_create_block(&params.arguments).await,
            "logseq_search" => self.tool_search(&params.arguments).await,
            "logseq_get_block_tree" => self.tool_get_block_tree(&params.arguments).await,
            "logseq_get_page_blocks" => self.tool_get_page_blocks(&params.arguments).await,
            "logseq_list_pages" => self.tool_list_pages(&params.arguments).await,
            "logseq_get_journal" => self.tool_get_journal(&params.arguments).await,
            "logseq_create_task" => self.tool_create_task(&params.arguments).await,
            "logseq_link_blocks" => self.tool_link_blocks(&params.arguments).await,
            "logseq_get_backlinks" => self.tool_get_backlinks(&params.arguments).await,
            "logseq_delete_block" => self.tool_delete_block(&params.arguments).await,
            "logseq_rebuild_index" => self.tool_rebuild_index(&params.arguments).await,
            "logseq_index_health" => self.tool_index_health(&params.arguments).await,
            // Cognitive tools
            "logseq_cognitive_mirror" => self.tool_cognitive_mirror(&params.arguments).await,
            "logseq_serendipity" => self.tool_serendipity(&params.arguments).await,
            "logseq_agent_memory" => self.tool_agent_memory(&params.arguments).await,
            "logseq_argument_map" => self.tool_argument_map(&params.arguments).await,
            "logseq_mental_model" => self.tool_mental_model(&params.arguments).await,
            "logseq_counterfactual" => self.tool_counterfactual_explore(&params.arguments).await,
            "logseq_knowledge_evolution" => self.tool_knowledge_evolution(&params.arguments).await,
            // Unknown tool - return proper MCP error
            _ => {
                return McpResponse::ToolsCall(ToolsCallResult {
                    content: vec![ContentBlock::Text {
                        text: McpError::method_not_found(&params.name).to_string(),
                    }],
                    is_error: Some(true),
                })
            }
        };

        match result {
            Ok(text) => McpResponse::ToolsCall(ToolsCallResult {
                content: vec![ContentBlock::Text { text }],
                is_error: Some(false),
            }),
            Err(e) => McpResponse::ToolsCall(ToolsCallResult {
                content: vec![ContentBlock::Text { text: e }],
                is_error: Some(true),
            }),
        }
    }

    // ── Resource reading ──────────────────────────────────────────

    async fn read_resource(&self, params: ReadResourceParams) -> McpResponse {
        let content = match params.uri.as_str() {
            "logseq://graph" => self.resource_graph().await,
            "logseq://pages" => self.resource_pages().await,
            "logseq://journals" => self.resource_journals().await,
            "logseq://tags" => self.resource_tags().await,
            // Cognitive resources
            "logseq://cognitive/map" => self.resource_cognitive_map().await,
            "logseq://cognitive/serendipity" => self.resource_cognitive_serendipity().await,
            uri if uri.starts_with("logseq://cognitive/arguments/") => {
                self.resource_arguments(uri).await
            }
            "logseq://cognitive/mental-models" => self.resource_mental_models().await,
            _ => Err(format!("Unknown resource: {}", params.uri)),
        };

        match content {
            Ok(text) => McpResponse::ResourcesRead(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: params.uri,
                    mime_type: "application/json".to_string(),
                    text: Some(text),
                }],
            }),
            Err(e) => McpResponse::ResourcesRead(ResourceReadResult {
                contents: vec![ResourceContent {
                    uri: params.uri,
                    mime_type: "text/plain".to_string(),
                    text: Some(e),
                }],
            }),
        }
    }

    // ── Tool implementations ─────────────────────────────────────

    async fn tool_query(&self, args: &serde_json::Value) -> Result<String, String> {
        let dsl = args
            .get("dsl")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'dsl' parameter")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

        // If pool is available, execute the query against the database
        if let Some(pool) = &self.pool {
            let result = self.query_service.execute(dsl, limit, pool).await?;

            let blocks_json: Vec<serde_json::Value> =
                result.blocks.iter().map(block_to_json).collect();

            return Ok(serde_json::to_string_pretty(&serde_json::json!({
                "count": result.count,
                "blocks": blocks_json,
                "sql": result.sql,
            }))
            .unwrap_or_else(|e| format!("Serialization error: {}", e)));
        }

        // Fallback: plan only (no DB)
        match self.query_service.prepare(dsl, limit) {
            Ok(result) => {
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "ast": result.ast,
                    "sql": result.sql,
                    "params": result.params,
                    "note": "Query planned (no database connected). Use with_pool() to enable execution."
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }
            Err(e) => Err(format!("Query error: {}", e)),
        }
    }

    async fn tool_create_block(&self, args: &serde_json::Value) -> Result<String, String> {
        let page_name = args
            .get("page_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_name' parameter")?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' parameter")?;
        let parent_id = parse_optional_uuid(args, "parent_id")?;
        let marker = parse_optional_marker(args, "marker")?;

        // Ensure page exists or create it
        let page = match self.page_repo.get_by_name(page_name).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let page = Page::new(PageCreate {
                    name: page_name.to_string(),
                    title: None,
                    namespace_id: None,
                    journal_day: None,
                    format: BlockFormat::Markdown,
                    file_id: None,
                })
                .map_err(|e| e.to_string())?;
                self.page_repo
                    .insert(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                page
            }
            Err(e) => return Err(e.to_string()),
        };

        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id,
            order: 1.0,
            marker,
            format: BlockFormat::Markdown,
            properties: Default::default(),
        })
        .map_err(|e| e.to_string())?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "id": block.id.to_string(),
            "page_id": block.page_id.to_string(),
            "page_name": page_name,
            "content": content,
            "parent_id": parent_id.map(|id| id.to_string()),
            "marker": marker.map(|m| format!("{:?}", m)),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_search(&self, args: &serde_json::Value) -> Result<String, String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'query' parameter")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

        let results = self
            .search_service
            .search(query, limit)
            .await
            .map_err(|e| e.to_string())?;

        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "block_id": r.block_id,
                    "page_name": r.page_name,
                    "snippet": r.snippet,
                    "score": r.score,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "count": results.len(),
            "results": json_results,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_get_block_tree(&self, args: &serde_json::Value) -> Result<String, String> {
        let block_id = parse_uuid(args, "block_id")?;
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Block not found: {}", block_id))?;

        let children = self
            .block_repo
            .get_children(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "block": block_to_json(&block),
            "children": children.iter().map(block_to_json).collect::<Vec<_>>(),
            "children_count": children.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_get_page_blocks(&self, args: &serde_json::Value) -> Result<String, String> {
        let page_name = args
            .get("page_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_name' parameter")?;

        let page = self
            .page_repo
            .get_by_name(page_name)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Page not found: {}", page_name))?;

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "page": { "id": page.id.to_string(), "name": page.name },
            "blocks": blocks.iter().map(block_to_json).collect::<Vec<_>>(),
            "count": blocks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_list_pages(&self, _args: &serde_json::Value) -> Result<String, String> {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;

        let page_list: Vec<serde_json::Value> = pages
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "title": p.title,
                    "journal": p.journal,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "count": page_list.len(),
            "pages": page_list,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_get_journal(&self, args: &serde_json::Value) -> Result<String, String> {
        let date_str = args
            .get("date")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'date' parameter")?;

        let day = JournalDay::from_str(date_str).map_err(|e| e.to_string())?;

        let page = match self
            .page_repo
            .get_journal(day)
            .await
            .map_err(|e| e.to_string())?
        {
            Some(p) => p,
            None => {
                // Create journal page
                let page =
                    Page::new_journal(day, BlockFormat::Markdown).map_err(|e| e.to_string())?;
                self.page_repo
                    .insert(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                page
            }
        };

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "page": { "id": page.id.to_string(), "name": page.name, "journal_day": day.as_i32() },
            "blocks": blocks.iter().map(block_to_json).collect::<Vec<_>>(),
            "block_count": blocks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_create_task(&self, args: &serde_json::Value) -> Result<String, String> {
        let page_name = args
            .get("page_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'page_name' parameter")?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'content' parameter")?;

        // Ensure page exists
        let page = match self.page_repo.get_by_name(page_name).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let p = Page::new(PageCreate {
                    name: page_name.to_string(),
                    title: None,
                    namespace_id: None,
                    journal_day: None,
                    format: BlockFormat::Markdown,
                    file_id: None,
                })
                .map_err(|e| e.to_string())?;
                self.page_repo.insert(&p).await.map_err(|e| e.to_string())?;
                p
            }
            Err(e) => return Err(e.to_string()),
        };

        let block = Block::new(BlockCreate {
            page_id: page.id,
            content: content.to_string(),
            parent_id: None,
            order: 1.0,
            marker: Some(TaskMarker::Todo),
            format: BlockFormat::Markdown,
            properties: Default::default(),
        })
        .map_err(|e| e.to_string())?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "id": block.id.to_string(),
            "page_name": page_name,
            "content": content,
            "marker": "TODO",
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_link_blocks(&self, args: &serde_json::Value) -> Result<String, String> {
        let source_id = parse_uuid(args, "source_id")?;
        let target_id = parse_uuid(args, "target_id")?;

        // Verify both blocks exist
        let mut source = self
            .block_repo
            .get_by_id(source_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Source block not found: {}", source_id))?;

        self.block_repo
            .get_by_id(target_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Target block not found: {}", target_id))?;

        // Add reference to source block's refs list
        if !source.refs.contains(&target_id) {
            source.refs.push(target_id);
            self.block_repo
                .update(&source)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(serde_json::json!({
            "status": "linked",
            "source_id": source_id.to_string(),
            "target_id": target_id.to_string(),
        })
        .to_string())
    }

    async fn tool_get_backlinks(&self, args: &serde_json::Value) -> Result<String, String> {
        let block_id = parse_uuid(args, "block_id")?;
        let backlinks = self
            .block_repo
            .get_backlinks(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "block_id": block_id.to_string(),
            "backlinks": backlinks.iter().map(block_to_json).collect::<Vec<_>>(),
            "count": backlinks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    async fn tool_delete_block(&self, args: &serde_json::Value) -> Result<String, String> {
        let block_id = parse_uuid(args, "block_id")?;

        self.block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Block not found: {}", block_id))?;

        self.block_repo
            .delete(block_id)
            .await
            .map_err(|e| e.to_string())?;

        let deleted_at = chrono::Utc::now().to_rfc3339();

        Ok(serde_json::json!({
            "status": "deleted",
            "block_id": block_id.to_string(),
        })
        .to_string())
    }

    async fn tool_rebuild_index(&self, args: &serde_json::Value) -> Result<String, String> {
        let index = self.search_index.as_ref().ok_or_else(|| {
            "SearchIndexManager not configured. Use with_search_index() to enable.".to_string()
        })?;

        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");

        let count = match mode {
            "full" => index.rebuild_full().await?,
            "incremental" => {
                let since = args
                    .get("since")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(1));
                index.rebuild_incremental(since).await?
            }
            other => {
                return Err(format!(
                    "Unknown rebuild mode: {}. Use 'full' or 'incremental'.",
                    other
                ))
            }
        };

        Ok(serde_json::json!({
            "status": "rebuilt",
            "mode": mode,
            "indexed_blocks": count,
        })
        .to_string())
    }

    async fn tool_index_health(&self, _args: &serde_json::Value) -> Result<String, String> {
        let index = self.search_index.as_ref().ok_or_else(|| {
            "SearchIndexManager not configured. Use with_search_index() to enable.".to_string()
        })?;

        let health = index.health_check().await?;

        Ok(serde_json::json!({
            "fts_count": health.fts_count,
            "blocks_count": health.blocks_count,
            "in_sync": health.in_sync,
            "status": if health.in_sync { "healthy" } else { "out_of_sync" },
        })
        .to_string())
    }

    // ── Resource implementations ──────────────────────────────────

    async fn resource_graph(&self) -> Result<String, String> {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page_count = pages.len();

        let journal_count = pages.iter().filter(|p| p.journal).count();

        let mut all_blocks = Vec::new();
        for page in &pages {
            let blocks = self
                .block_repo
                .get_by_page(page.id)
                .await
                .map_err(|e| e.to_string())?;
            all_blocks.extend(blocks);
        }

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "pages": page_count,
            "journals": journal_count,
            "blocks": all_blocks.len(),
            "last_updated": chrono::Utc::now().to_rfc3339(),
        }))
        .unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_pages(&self) -> Result<String, String> {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;

        let page_list: Vec<serde_json::Value> = pages
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "title": p.title,
                    "journal": p.journal,
                    "journal_day": p.journal_day.map(|d| d.as_i32()),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&page_list).unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_journals(&self) -> Result<String, String> {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let journals: Vec<_> = pages.into_iter().filter(|p| p.journal).collect();

        let journal_list: Vec<serde_json::Value> = journals
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "journal_day": p.journal_day.map(|d| d.as_i32()),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&journal_list).unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_tags(&self) -> Result<String, String> {
        let tag_counts = self
            .tag_repo
            .get_tag_counts()
            .await
            .map_err(|e| e.to_string())?;

        let tag_list: Vec<serde_json::Value> = tag_counts
            .iter()
            .map(|(tag, count)| {
                serde_json::json!({
                    "name": tag,
                    "count": count,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&tag_list).unwrap_or_else(|e| e.to_string()))
    }

    // ── Cognitive tool handlers ───────────────────────────────────

    async fn tool_cognitive_mirror(&self, args: &serde_json::Value) -> Result<String, String> {
        let page_name = args
            .get("page_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'page_name' parameter".to_string())?;

        let mirror = self
            .cognitive_mirror
            .as_ref()
            .ok_or_else(|| "CognitiveMirror not configured".to_string())?;

        // Find page by name
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page = pages
            .iter()
            .find(|p| p.name == page_name)
            .ok_or_else(|| format!("Page not found: {}", page_name))?;

        let map = mirror.analyze(page.id).await.map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&map).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_serendipity(&self, args: &serde_json::Value) -> Result<String, String> {
        let since = args
            .get("since")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(20);

        let min_confidence = args
            .get("min_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.3) as f32;

        let engine = self
            .serendipity_engine
            .as_ref()
            .ok_or_else(|| "SerendipityEngine not configured".to_string())?;

        let since_utc = since.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::days(7));
        let since_days_ago = (chrono::Utc::now() - since_utc).num_days();
        let query = quilt_cognitive::serendipity::SerendipityQuery {
            topic: None,
            limit,
            offset: 0,
            min_confidence,
            temporal_window_days: Some(since_days_ago),
            page_id: None,
        };

        let connections = engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&connections).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_agent_memory(&self, args: &serde_json::Value) -> Result<String, String> {
        let domain = args
            .get("domain")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'domain' parameter".to_string())?;

        let query = args.get("query").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize)
            .unwrap_or(10);

        let memory = self
            .agent_memory
            .as_ref()
            .ok_or_else(|| "AgentMemory not configured".to_string())?;

        let mem_query = quilt_cognitive::agent_memory::MemoryQuery {
            agent_id: domain.to_string(),
            context: None,
            query: query.map(String::from),
            limit,
        };

        let entries = memory
            .retrieve(mem_query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&entries).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_argument_map(&self, args: &serde_json::Value) -> Result<String, String> {
        let page_name = args
            .get("page_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'page_name' parameter".to_string())?;

        let cartographer = self
            .argument_cartographer
            .as_ref()
            .ok_or_else(|| "ArgumentCartographer not configured".to_string())?;

        // Find page by name
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page = pages
            .iter()
            .find(|p| p.name == page_name)
            .ok_or_else(|| format!("Page not found: {}", page_name))?;

        let graph = cartographer
            .map_arguments(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_mental_model(&self, args: &serde_json::Value) -> Result<String, String> {
        let agent_id = args
            .get("agent_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'agent_id' parameter".to_string())?;

        let _time_window = args
            .get("time_window")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .map(|days| chrono::Duration::days(days));

        let gardener = self
            .mental_model_gardener
            .as_ref()
            .ok_or_else(|| "MentalModelGardener not configured".to_string())?;

        let model = gardener
            .build_model(agent_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&model).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_counterfactual_explore(
        &self,
        args: &serde_json::Value,
    ) -> Result<String, String> {
        let scenario = args
            .get("scenario")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'scenario' parameter".to_string())?;

        let decision_point = args
            .get("decision_point")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'decision_point' parameter".to_string())?;

        let explorer = self
            .counterfactual_explorer
            .as_ref()
            .ok_or_else(|| "CounterfactualExplorer not configured".to_string())?;

        let tree = explorer
            .explore(scenario, decision_point)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&tree).unwrap_or_else(|e| e.to_string()))
    }

    async fn tool_knowledge_evolution(&self, args: &serde_json::Value) -> Result<String, String> {
        let topic = args
            .get("topic")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'topic' parameter".to_string())?;

        let timespan_days = args
            .get("timespan_days")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32)
            .unwrap_or(30);

        let tracker = self
            .knowledge_evolution_tracker
            .as_ref()
            .ok_or_else(|| "KnowledgeEvolutionTracker not configured".to_string())?;

        let timeline = tracker
            .track(topic, timespan_days)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&timeline).unwrap_or_else(|e| e.to_string()))
    }

    // ── Cognitive resource handlers ─────────────────────────────────

    async fn resource_cognitive_map(&self) -> Result<String, String> {
        let mirror = self
            .cognitive_mirror
            .as_ref()
            .ok_or_else(|| "CognitiveMirror not configured".to_string())?;

        // Get overall stats by analyzing recent pages
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let recent_pages: Vec<_> = pages.iter().take(10).collect();

        let mut total_clusters = 0;
        let mut total_frontiers = 0;
        let mut total_gaps = 0;
        let pages_count = recent_pages.len();

        for page in &recent_pages {
            if let Ok(map) = mirror.analyze(page.id).await {
                total_clusters += map.clusters.len();
                total_frontiers += map.frontiers.len();
                total_gaps += map.gaps.len();
            }
        }

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "total_clusters": total_clusters,
            "total_frontiers": total_frontiers,
            "total_gaps": total_gaps,
            "pages_analyzed": pages_count,
        }))
        .unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_cognitive_serendipity(&self) -> Result<String, String> {
        let engine = self
            .serendipity_engine
            .as_ref()
            .ok_or_else(|| "SerendipityEngine not configured".to_string())?;

        let query = quilt_cognitive::serendipity::SerendipityQuery {
            topic: None,
            limit: 20,
            offset: 0,
            min_confidence: 0.3,
            temporal_window_days: Some(30),
            page_id: None,
        };

        let connections = engine
            .find_connections(query)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&connections).unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_arguments(&self, uri: &str) -> Result<String, String> {
        let page_name = uri
            .strip_prefix("logseq://cognitive/arguments/")
            .ok_or_else(|| "Invalid arguments resource URI".to_string())?;

        let cartographer = self
            .argument_cartographer
            .as_ref()
            .ok_or_else(|| "ArgumentCartographer not configured".to_string())?;

        // Find page by name
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let page = pages
            .iter()
            .find(|p| p.name == page_name)
            .ok_or_else(|| format!("No arguments found for page: {}", page_name))?;

        let graph = cartographer
            .map_arguments(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&graph).unwrap_or_else(|e| e.to_string()))
    }

    async fn resource_mental_models(&self) -> Result<String, String> {
        let gardener = self
            .mental_model_gardener
            .as_ref()
            .ok_or_else(|| "MentalModelGardener not configured".to_string())?;

        // Get all journals as potential agents
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;
        let journals: Vec<_> = pages.iter().filter(|p| p.journal).collect();

        let mut models = Vec::new();
        for journal in journals.iter().take(10) {
            if let Ok(model) = gardener.build_model(&journal.name).await {
                models.push(model);
            }
        }

        Ok(serde_json::to_string_pretty(&models).unwrap_or_else(|e| e.to_string()))
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn parse_uuid(args: &serde_json::Value, key: &str) -> Result<Uuid, String> {
    let s = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing '{}' parameter", key))?;
    Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))
}

fn parse_optional_uuid(args: &serde_json::Value, key: &str) -> Result<Option<Uuid>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(Some(
            Uuid::parse_str(s).ok_or_else(|| format!("Invalid UUID: {}", s))?,
        )),
        _ => Ok(None),
    }
}

fn parse_optional_marker(
    args: &serde_json::Value,
    key: &str,
) -> Result<Option<TaskMarker>, String> {
    match args.get(key).and_then(|v| v.as_str()) {
        Some("now") => Ok(Some(TaskMarker::Now)),
        Some("later") => Ok(Some(TaskMarker::Later)),
        Some("todo") => Ok(Some(TaskMarker::Todo)),
        Some("done") => Ok(Some(TaskMarker::Done)),
        Some("cancelled") => Ok(Some(TaskMarker::Cancelled)),
        Some(s) if s.is_empty() => Ok(None),
        None => Ok(None),
        Some(other) => Err(format!("Invalid marker: {}", other)),
    }
}

fn block_to_json(block: &Block) -> serde_json::Value {
    serde_json::json!({
        "id": block.id.to_string(),
        "page_id": block.page_id.to_string(),
        "parent_id": block.parent_id.map(|id| id.to_string()),
        "order": block.order,
        "level": block.level,
        "content": block.content,
        "marker": block.marker.as_ref().map(|m| format!("{:?}", m)),
        "priority": block.priority.as_ref().map(|p| format!("{:?}", p)),
        "collapsed": block.collapsed,
        "created_at": block.created_at.to_rfc3339(),
        "updated_at": block.updated_at.to_rfc3339(),
    })
}

// ── Integration Tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_infrastructure::database::sqlite::connection;
    use quilt_infrastructure::database::sqlite::repositories::{
        SqliteBlockRepository, SqlitePageRepository, SqliteTagRepository,
    };
    use sqlx::SqlitePool;

    async fn setup_server() -> (McpServer, SqlitePool) {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");
        connection::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let server = McpServer::new(
            Arc::new(SqliteBlockRepository::new(pool.clone())),
            Arc::new(SqlitePageRepository::new(pool.clone())),
            Arc::new(SqliteTagRepository::new(pool.clone())),
            Arc::new(SearchService::new(pool.clone())),
        )
        .with_pool(pool.clone());
        (server, pool)
    }

    // ── Tool tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tool_list_pages_empty() {
        let (server, _pool) = setup_server().await;
        let result = server
            .tool_list_pages(&serde_json::json!({}))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["count"], 0);
    }

    #[tokio::test]
    async fn test_tool_create_block_and_list_pages() {
        let (server, _pool) = setup_server().await;

        // Create a block (auto-creates the page)
        let result = server
            .tool_create_block(&serde_json::json!({
                "page_name": "test-page",
                "content": "Hello MCP"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["page_name"], "test-page");
        assert_eq!(v["content"], "Hello MCP");

        // List pages should now show 1
        let result = server
            .tool_list_pages(&serde_json::json!({}))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["count"], 1);
        assert_eq!(v["pages"][0]["name"], "test-page");
    }

    #[tokio::test]
    async fn test_tool_get_page_blocks() {
        let (server, _pool) = setup_server().await;

        server
            .tool_create_block(&serde_json::json!({
                "page_name": "blocks-page",
                "content": "First block"
            }))
            .await
            .unwrap();
        server
            .tool_create_block(&serde_json::json!({
                "page_name": "blocks-page",
                "content": "Second block"
            }))
            .await
            .unwrap();

        let result = server
            .tool_get_page_blocks(&serde_json::json!({
                "page_name": "blocks-page"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["count"], 2);
    }

    #[tokio::test]
    async fn test_tool_create_task() {
        let (server, _pool) = setup_server().await;

        let result = server
            .tool_create_task(&serde_json::json!({
                "page_name": "tasks",
                "content": "Implement MCP"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["marker"], "TODO");
        assert_eq!(v["content"], "Implement MCP");

        // Verify block has TODO marker
        let result = server
            .tool_get_page_blocks(&serde_json::json!({
                "page_name": "tasks"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        // Verify block has TODO marker (Debug format: Some("Todo"))
        assert!(v["blocks"][0]["marker"].as_str().unwrap().contains("Todo"));
    }

    #[tokio::test]
    async fn test_tool_get_block_tree() {
        let (server, _pool) = setup_server().await;

        // Create parent block
        let result = server
            .tool_create_block(&serde_json::json!({
                "page_name": "tree-page",
                "content": "Parent"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        let parent_id = v["id"].as_str().unwrap();

        // Create child block
        server
            .tool_create_block(&serde_json::json!({
                "page_name": "tree-page",
                "content": "Child",
                "parent_id": parent_id
            }))
            .await
            .unwrap();

        // Get block tree
        let result = server
            .tool_get_block_tree(&serde_json::json!({
                "block_id": parent_id
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["block"]["content"], "Parent");
        assert_eq!(v["children_count"], 1);
        assert_eq!(v["children"][0]["content"], "Child");
    }

    #[tokio::test]
    async fn test_tool_link_blocks() {
        let (server, _pool) = setup_server().await;

        let r1 = server
            .tool_create_block(&serde_json::json!({
                "page_name": "links",
                "content": "Source"
            }))
            .await
            .unwrap();
        let source_id = serde_json::from_str::<serde_json::Value>(&r1).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let r2 = server
            .tool_create_block(&serde_json::json!({
                "page_name": "links",
                "content": "Target"
            }))
            .await
            .unwrap();
        let target_id = serde_json::from_str::<serde_json::Value>(&r2).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let result = server
            .tool_link_blocks(&serde_json::json!({
                "source_id": source_id,
                "target_id": target_id
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["status"], "linked");
    }

    #[tokio::test]
    async fn test_tool_delete_block() {
        let (server, _pool) = setup_server().await;

        let result = server
            .tool_create_block(&serde_json::json!({
                "page_name": "delete-page",
                "content": "To delete"
            }))
            .await
            .unwrap();
        let block_id = serde_json::from_str::<serde_json::Value>(&result).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let result = server
            .tool_delete_block(&serde_json::json!({
                "block_id": block_id
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["status"], "deleted");
    }

    #[tokio::test]
    async fn test_tool_search() {
        let (server, _pool) = setup_server().await;

        server
            .tool_create_block(&serde_json::json!({
                "page_name": "rust-page",
                "content": "Rust is a systems programming language"
            }))
            .await
            .unwrap();
        server
            .tool_create_block(&serde_json::json!({
                "page_name": "python-page",
                "content": "Python is great for scripting"
            }))
            .await
            .unwrap();

        let result = server
            .tool_search(&serde_json::json!({
                "query": "Rust"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v["count"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn test_tool_get_journal() {
        let (server, _pool) = setup_server().await;

        let result = server
            .tool_get_journal(&serde_json::json!({
                "date": "2026-05-03"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(
            v["page"]["journal_day"],
            serde_json::Value::Number(20260503.into())
        );
    }

    // ── Resource tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_resource_graph() {
        let (server, _pool) = setup_server().await;

        server
            .tool_create_block(&serde_json::json!({
                "page_name": "r1",
                "content": "Block 1"
            }))
            .await
            .unwrap();

        let result = server.resource_graph().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["pages"], 1);
        assert_eq!(v["blocks"], 1);
    }

    #[tokio::test]
    async fn test_resource_pages() {
        let (server, _pool) = setup_server().await;

        server
            .tool_create_block(&serde_json::json!({
                "page_name": "alpha",
                "content": "A"
            }))
            .await
            .unwrap();
        server
            .tool_create_block(&serde_json::json!({
                "page_name": "beta",
                "content": "B"
            }))
            .await
            .unwrap();

        let result = server.resource_pages().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_resource_tags() {
        let (server, _pool) = setup_server().await;

        let result = server
            .tool_create_block(&serde_json::json!({
                "page_name": "tagged-page",
                "content": "Tagged content"
            }))
            .await
            .unwrap();
        let page_id = serde_json::from_str::<serde_json::Value>(&result).unwrap()["page_id"]
            .as_str()
            .unwrap()
            .to_string();

        let pid = Uuid::parse_str(&page_id).unwrap();
        server.tag_repo.add_tag(pid, "rust").await.unwrap();
        server.tag_repo.add_tag(pid, "mcp").await.unwrap();

        let result = server.resource_tags().await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    // ── Protocol tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_initialize() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::Initialize {
                params: InitializeParams {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: ClientCapabilities {
                        roots: None,
                        sampling: None,
                    },
                },
            })
            .await;

        match response {
            McpResponse::Initialize(result) => {
                assert_eq!(result.protocol_version, "2024-11-05");
                assert_eq!(result.server_info.name, "quilt-mcp");
            }
            _ => panic!("Expected Initialize response"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_tools() {
        let (server, _pool) = setup_server().await;

        let response = server.handle_request(McpRequest::ListTools).await;

        match response {
            McpResponse::ToolsList(result) => {
                assert_eq!(result.tools.len(), 13);
                assert!(result.tools.iter().any(|t| t.name == "logseq_search"));
                assert!(result.tools.iter().any(|t| t.name == "logseq_create_block"));
            }
            _ => panic!("Expected ToolsList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_resources() {
        let (server, _pool) = setup_server().await;

        let response = server.handle_request(McpRequest::ListResources).await;

        match response {
            McpResponse::ResourcesList(result) => {
                assert_eq!(result.resources.len(), 4);
                assert!(result.resources.iter().any(|r| r.uri == "logseq://graph"));
                assert!(result.resources.iter().any(|r| r.uri == "logseq://tags"));
            }
            _ => panic!("Expected ResourcesList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_call_tool() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::CallTool {
                params: CallToolParams {
                    name: "logseq_list_pages".to_string(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;

        match response {
            McpResponse::ToolsCall(result) => {
                assert!(!result.is_error.unwrap());
                let v: serde_json::Value =
                    serde_json::from_str(&result.content[0].text().unwrap()).unwrap();
                assert_eq!(v["count"], 0);
            }
            _ => panic!("Expected ToolsCall response"),
        }
    }

    // Make ContentBlock text accessible in tests
    impl ContentBlock {
        fn text(&self) -> Option<&str> {
            match self {
                ContentBlock::Text { text } => Some(text),
                _ => None,
            }
        }
    }
}
