//! MCP Server implementation for Quilt
//!
//! Implements the Model Context Protocol for AI agent integration.
//! This is a thin coordinator that delegates to handlers.

use crate::handlers::{ResourceProvider, ToolHandler};
use crate::protocol::*;
use std::sync::Arc;

/// MCP server for Quilt knowledge graph operations.
///
/// This server implements the Model Context Protocol, providing AI agents
/// with tools to query and modify the Quilt knowledge graph.
///
/// The server is a thin coordinator that delegates to handler implementations:
/// - Tool execution is handled by registered `ToolHandler` implementations
/// - Resource reading is handled by registered `ResourceProvider` implementations
pub struct McpServer {
    tool_handlers: Vec<Box<dyn ToolHandler>>,
    resource_providers: Vec<Box<dyn ResourceProvider>>,
}

impl McpServer {
    /// Creates a new MCP server with the given handlers.
    ///
    /// # Arguments
    ///
    /// * `tool_handlers` - Vector of tool handlers (each owns a domain of tools)
    /// * `resource_providers` - Vector of resource providers (each owns a domain of resources)
    pub fn new(
        tool_handlers: Vec<Box<dyn ToolHandler>>,
        resource_providers: Vec<Box<dyn ResourceProvider>>,
    ) -> Self {
        Self {
            tool_handlers,
            resource_providers,
        }
    }

    // ── Request handling ─────────────────────────────────────────────

    /// Handle an incoming MCP request.
    pub async fn handle_request(&self, request: McpRequest) -> McpResponse {
        match request {
            McpRequest::Initialize { params: _ } => self.handle_initialize(),
            McpRequest::ListTools => self.handle_list_tools(),
            McpRequest::CallTool { params } => self.handle_call_tool(params).await,
            McpRequest::ListResources => self.handle_list_resources(),
            McpRequest::ReadResource { params } => self.handle_read_resource(params).await,
            McpRequest::EnableNotifications => self.handle_initialize(),
        }
    }

    fn handle_initialize(&self) -> McpResponse {
        McpResponse::Initialize(InitializeResult {
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
        })
    }

    fn handle_list_tools(&self) -> McpResponse {
        let mut all_tools = Vec::new();
        for handler in &self.tool_handlers {
            all_tools.extend(handler.tools());
        }
        McpResponse::ToolsList(ToolsListResult { tools: all_tools })
    }

    async fn handle_call_tool(&self, params: CallToolParams) -> McpResponse {
        let result = self.execute_tool(&params.name, &params.arguments).await;

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

    fn handle_list_resources(&self) -> McpResponse {
        let mut all_resources = Vec::new();
        for provider in &self.resource_providers {
            all_resources.extend(provider.resources());
        }
        McpResponse::ResourcesList(ResourcesListResult {
            resources: all_resources,
        })
    }

    async fn handle_read_resource(&self, params: ReadResourceParams) -> McpResponse {
        match self.read_resource(&params.uri).await {
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

    // ── Tool execution ────────────────────────────────────────────────

    async fn execute_tool(&self, name: &str, args: &serde_json::Value) -> Result<String, String> {
        for handler in &self.tool_handlers {
            let tools = handler.tools();
            if tools.iter().any(|t| t.name == name) {
                return handler.execute(name, args).await;
            }
        }
        Err(format!("Unknown tool: {}", name))
    }

    // ── Resource reading ─────────────────────────────────────────────

    async fn read_resource(&self, uri: &str) -> Result<String, String> {
        for provider in &self.resource_providers {
            let resources = provider.resources();
            if resources.iter().any(|r| r.uri == uri) {
                return provider.read(uri).await;
            }
        }
        Err(format!("Unknown resource: {}", uri))
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::{
        block::BlockToolHandler, page::PageToolHandler, query::QueryToolHandler,
        resource::GraphResourceProvider, template::TemplateToolHandler,
    };
    use quilt_application::use_cases::{
        BlockUseCases, BlockUseCasesImpl, PageUseCases, PageUseCasesImpl, ResourceUseCases,
        ResourceUseCasesImpl, SearchUseCasesImpl, TemplateUseCases, TemplateUseCasesImpl,
    };
    use quilt_infrastructure::database::sqlite::connection;
    use quilt_infrastructure::database::sqlite::repositories::{
        SqliteBlockRepository, SqlitePageRepository, SqliteTagRepository,
    };
    use quilt_search::SearchService;
    use sqlx::SqlitePool;

    async fn setup_server() -> (McpServer, SqlitePool) {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory DB");
        connection::run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
        let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
        let tag_repo = Arc::new(SqliteTagRepository::new(pool.clone()));

        let block_use_cases: Arc<dyn BlockUseCases> = Arc::new(BlockUseCasesImpl::new(
            block_repo.clone(),
            page_repo.clone(),
        ));
        let page_use_cases: Arc<dyn PageUseCases> =
            Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
        let resource_use_cases: Arc<dyn ResourceUseCases> = Arc::new(ResourceUseCasesImpl::new(
            block_repo.clone(),
            page_repo.clone(),
            tag_repo.clone(),
        ));
        let template_use_cases: Arc<dyn TemplateUseCases> = Arc::new(TemplateUseCasesImpl::new(
            page_repo.clone(),
            block_repo.clone(),
        ));

        let block_handler = BlockToolHandler::new(block_use_cases.clone());
        let page_handler = PageToolHandler::new(page_use_cases.clone());
        let query_handler = QueryToolHandler::new(Arc::new(
            quilt_application::use_cases::SearchUseCasesImpl::new()
                .with_search_service(Arc::new(SearchService::new(Arc::new(pool.clone())))),
        ));
        let template_handler = TemplateToolHandler::new(template_use_cases.clone());
        let resource_provider = GraphResourceProvider::new(resource_use_cases);

        let server = McpServer::new(
            vec![
                Box::new(block_handler),
                Box::new(page_handler),
                Box::new(query_handler),
                Box::new(template_handler),
            ],
            vec![Box::new(resource_provider)],
        );
        (server, pool)
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
                // Core tools: 14 total
                // BlockToolHandler: quilt_create_block, quilt_delete_block, quilt_link_blocks,
                //                   quilt_get_block_tree, quilt_get_backlinks, quilt_create_task,
                //                   quilt_list_blocks_by_author (7) — ADR-0003 added the last
                // PageToolHandler: quilt_list_pages, quilt_get_page_blocks, quilt_get_journal (3)
                // QueryToolHandler: quilt_query, quilt_search (2)
                // TemplateToolHandler: quilt_list_templates, quilt_get_template_schema (2) — ADR-0007
                assert_eq!(result.tools.len(), 14);
                assert!(result.tools.iter().any(|t| t.name == "quilt_search"));
                assert!(result.tools.iter().any(|t| t.name == "quilt_create_block"));
                assert!(result.tools.iter().any(|t| t.name == "quilt_list_blocks_by_author"));
                assert!(result.tools.iter().any(|t| t.name == "quilt_list_templates"));
                assert!(result.tools.iter().any(|t| t.name == "quilt_get_template_schema"));
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
                assert!(result.resources.iter().any(|r| r.uri == "quilt://graph"));
                assert!(result.resources.iter().any(|r| r.uri == "quilt://tags"));
            }
            _ => panic!("Expected ResourcesList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_call_tool_list_pages() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::CallTool {
                params: CallToolParams {
                    name: "quilt_list_pages".to_string(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;

        match response {
            McpResponse::ToolsCall(result) => {
                assert!(!result.is_error.unwrap());
                let v: serde_json::Value = serde_json::from_str(&result.content[0].text()).unwrap();
                assert_eq!(v["count"], 0);
            }
            _ => panic!("Expected ToolsCall response"),
        }
    }

    // ADR-0007: quilt_list_templates returns the empty set when
    // there are no template pages. Full template discovery is
    // covered in crates/quilt-application/tests/template_use_cases_tests.rs.
    #[tokio::test]
    async fn test_handle_call_tool_list_templates_empty() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::CallTool {
                params: CallToolParams {
                    name: "quilt_list_templates".to_string(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;

        match response {
            McpResponse::ToolsCall(result) => {
                assert!(!result.is_error.unwrap());
                let v: serde_json::Value = serde_json::from_str(&result.content[0].text()).unwrap();
                assert_eq!(v["count"], 0);
                assert!(v["templates"].as_array().unwrap().is_empty());
            }
            _ => panic!("Expected ToolsCall response"),
        }
    }

    // ADR-0007: quilt_get_template_schema returns template_not_found
    // (not an error) when the requested template does not exist.
    #[tokio::test]
    async fn test_handle_call_tool_get_template_schema_not_found() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::CallTool {
                params: CallToolParams {
                    name: "quilt_get_template_schema".to_string(),
                    arguments: serde_json::json!({ "name": "does-not-exist" }),
                },
            })
            .await;

        match response {
            McpResponse::ToolsCall(result) => {
                assert!(!result.is_error.unwrap());
                let v: serde_json::Value = serde_json::from_str(&result.content[0].text()).unwrap();
                assert_eq!(v["error"], "template_not_found");
                assert_eq!(v["name"], "does-not-exist");
            }
            _ => panic!("Expected ToolsCall response"),
        }
    }

    // ADR-0007: missing 'name' parameter returns a clear error.
    #[tokio::test]
    async fn test_handle_call_tool_get_template_schema_missing_name() {
        let (server, _pool) = setup_server().await;

        let response = server
            .handle_request(McpRequest::CallTool {
                params: CallToolParams {
                    name: "quilt_get_template_schema".to_string(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;

        match response {
            McpResponse::ToolsCall(result) => {
                assert!(result.is_error.unwrap());
                // Errors are returned as plain text strings, not JSON.
                assert_eq!(result.content[0].text(), "Missing 'name'");
            }
            _ => panic!("Expected ToolsCall response"),
        }
    }

    // Make ContentBlock text accessible in tests
    impl ContentBlock {
        fn text(&self) -> &str {
            match self {
                ContentBlock::Text { text } => text,
                _ => panic!("Expected Text content"),
            }
        }
    }

    // ── quilt-test-helpers integration test ─────────────────────────────

    #[tokio::test]
    async fn test_page_with_blocks_fixture() {
        // Verify quilt-test-helpers::page_with_blocks works correctly
        use quilt_test_helpers::{page_with_blocks, InMemoryBlockRepo, InMemoryPageRepo};

        let (page, blocks) = page_with_blocks("Test Page", vec!["Block 1", "Block 2"])
            .expect("page_with_blocks should succeed");

        // Verify page and blocks are correctly aligned
        assert_eq!(page.name, "test page");
        assert_eq!(blocks.len(), 2);
        assert!(blocks.iter().all(|b| b.page_id == page.id));

        // Insert into in-memory repos using the new helpers
        let page_repo = InMemoryPageRepo::new().with_pages(vec![page.clone()]);
        let block_repo = InMemoryBlockRepo::new()
            .with_page(page.clone(), blocks)
            .expect("blocks should belong to the page");

        // Verify repos are functional by getting trait objects
        // (confirms the Arc<dyn BlockRepository/PageRepository> APIs work)
        let _page_trait = page_repo.as_trait();
        let _block_trait = block_repo.as_trait();
    }
}
