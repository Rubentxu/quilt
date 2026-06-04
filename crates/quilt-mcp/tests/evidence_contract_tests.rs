//! Integration tests for the Evidence Contract v1.
//!
//! These tests enforce the contract that:
//! 1. **No handler serializes a top-level `evidence` key** in its
//!    returned JSON. The `evidence` key is reserved at the server
//!    layer (`server.rs::handle_call_tool` / `handle_read_resource`).
//! 2. **Every MCP tool call returns a `ToolsCallResult` with
//!    `_meta.evidence` populated** — either via the handler's
//!    `tool_evidence` override or via the universal fallback.
//! 3. **Every MCP resource read returns a `ResourceReadResult` with
//!    `_meta.evidence` populated** — same pattern.
//! 4. **Error responses carry `is_error: true` in evidence.**
//!
//! The test iterates all 17 live tools via the server's `ListTools`
//! and `ListResources` (per design auto-grill #4: 17 tools).
//! The spec's aspirational fallback tier for `settings_*` and
//! `quilt_*_cognitive` is documented but not exercised.

use std::sync::Arc;

use quilt_application::templates::reapply::{ReapplyTemplateUseCase, ReapplyTemplateUseCaseImpl};
use quilt_application::use_cases::{
    BlockUseCases, BlockUseCasesImpl, PageUseCases, PageUseCasesImpl, ResourceUseCases,
    ResourceUseCasesImpl, SearchUseCasesImpl, TemplateUseCases, TemplateUseCasesImpl,
};
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqliteTagRepository,
};
use quilt_mcp::McpServer;
use quilt_mcp::handlers::block::BlockToolHandler;
use quilt_mcp::handlers::graph::GraphToolHandler;
use quilt_mcp::handlers::page::PageToolHandler;
use quilt_mcp::handlers::query::QueryToolHandler;
use quilt_mcp::handlers::resource::GraphResourceProvider;
use quilt_mcp::handlers::retrieval::RetrievalToolHandler;
use quilt_mcp::handlers::template::TemplateToolHandler;
use quilt_mcp::handlers::temporal::TemporalToolHandler;
use quilt_mcp::protocol::McpRequest;
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
    let reapply_use_cases: Arc<dyn ReapplyTemplateUseCase> = Arc::new(
        ReapplyTemplateUseCaseImpl::new(template_use_cases.clone(), block_repo.clone()),
    );

    let search_use_cases = Arc::new(SearchUseCasesImpl::new().with_search_service(Arc::new(
        quilt_search::SearchService::new(Arc::new(pool.clone())),
    )));

    let block_handler = BlockToolHandler::new(block_use_cases.clone());
    let page_handler = PageToolHandler::new(page_use_cases.clone());
    let query_handler = QueryToolHandler::new(search_use_cases.clone());
    let retrieval_handler = RetrievalToolHandler::new(search_use_cases.clone());
    let temporal_handler = TemporalToolHandler::new(search_use_cases.clone());
    let graph_handler = GraphToolHandler::new(block_use_cases.clone());
    let template_handler =
        TemplateToolHandler::new(template_use_cases.clone(), reapply_use_cases.clone());
    let resource_provider = GraphResourceProvider::new(resource_use_cases);

    let server = McpServer::new(
        vec![
            Box::new(block_handler),
            Box::new(page_handler),
            Box::new(query_handler),
            Box::new(retrieval_handler),
            Box::new(temporal_handler),
            Box::new(graph_handler),
            Box::new(template_handler),
        ],
        vec![Box::new(resource_provider)],
    );
    (server, pool)
}

/// Contract sub-test A: no handler serializes a top-level `evidence`
/// key in its returned string. We iterate all 17 live tools and assert
/// the JSON text inside the `Text` content block does NOT contain a
/// top-level `evidence` key.
#[tokio::test]
async fn test_no_handler_serializes_top_level_evidence_key() {
    let (server, _pool) = setup_server().await;

    // Get the list of registered tools (17 live tools per design).
    let tools_response = server.handle_request(McpRequest::ListTools).await;
    let tools = match tools_response {
        quilt_mcp::protocol::McpResponse::ToolsList(r) => r.tools,
        _ => panic!("Expected ToolsList response"),
    };
    assert_eq!(tools.len(), 19, "Expected exactly 19 live tools (14 base + 3 retrieval-graph + 2 template)");

    // For each tool, call it (with safe empty args) and inspect the
    // JSON text. Failures are aggregated.
    let mut violations = Vec::new();
    for tool in &tools {
        let response = server
            .handle_request(McpRequest::CallTool {
                params: quilt_mcp::protocol::CallToolParams {
                    name: tool.name.clone(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;
        let result = match response {
            quilt_mcp::protocol::McpResponse::ToolsCall(r) => r,
            _ => {
                violations.push(format!("{}: non-ToolsCall response", tool.name));
                continue;
            }
        };
        // Extract the JSON text from the first Text content block.
        let text = match result.content.first() {
            Some(quilt_mcp::protocol::ContentBlock::Text { text }) => text,
            _ => {
                violations.push(format!("{}: no text content", tool.name));
                continue;
            }
        };
        // Parse the JSON text and assert it does NOT have top-level
        // `evidence` key. (It MAY have `_meta.evidence` — that's
        // server-injected and is fine.)
        let parsed: serde_json::Result<serde_json::Value> = serde_json::from_str(text);
        match parsed {
            Ok(v) => {
                if let Some(obj) = v.as_object() {
                    if obj.contains_key("evidence") {
                        violations.push(format!(
                            "{}: serializes top-level 'evidence' key",
                            tool.name
                        ));
                    }
                }
            }
            Err(_) => {
                // Non-JSON text is fine (errors are returned as plain
                // strings like "Missing 'page_name'"). We only check
                // JSON outputs.
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Handler(s) serialize top-level 'evidence' key (contract violation):\n  - {}",
        violations.join("\n  - ")
    );
}

/// Contract sub-test B: every response carries `_meta.evidence`.
/// We assert the `_meta` envelope is `Some` and `evidence` is `Some`
/// for every tool call.
#[tokio::test]
async fn test_every_response_carries_meta_evidence() {
    let (server, _pool) = setup_server().await;

    let tools_response = server.handle_request(McpRequest::ListTools).await;
    let tools = match tools_response {
        quilt_mcp::protocol::McpResponse::ToolsList(r) => r.tools,
        _ => panic!("Expected ToolsList response"),
    };
    assert_eq!(tools.len(), 19);

    let mut missing = Vec::new();
    for tool in &tools {
        let response = server
            .handle_request(McpRequest::CallTool {
                params: quilt_mcp::protocol::CallToolParams {
                    name: tool.name.clone(),
                    arguments: serde_json::json!({}),
                },
            })
            .await;
        let result = match response {
            quilt_mcp::protocol::McpResponse::ToolsCall(r) => r,
            _ => {
                missing.push(format!("{}: non-ToolsCall response", tool.name));
                continue;
            }
        };
        let meta = match result._meta {
            Some(m) => m,
            None => {
                missing.push(format!("{}: missing _meta envelope", tool.name));
                continue;
            }
        };
        let ev = match meta.evidence {
            Some(e) => e,
            None => {
                missing.push(format!("{}: missing _meta.evidence", tool.name));
                continue;
            }
        };
        // tool_name must match the called tool.
        if ev.tool_name != tool.name {
            missing.push(format!(
                "{}: evidence.tool_name = {} (expected {})",
                tool.name, ev.tool_name, tool.name
            ));
        }
        // Timestamp must be present.
        if ev.timestamp.timestamp() <= 0 {
            missing.push(format!("{}: evidence.timestamp is zero", tool.name));
        }
    }

    assert!(
        missing.is_empty(),
        "Tool(s) missing _meta.evidence:\n  - {}",
        missing.join("\n  - ")
    );
}

/// Contract sub-test C: error responses carry `is_error: true`.
/// We pick a tool that fails on bad args (quilt_get_page_blocks
/// without `page_name`) and assert the evidence has `is_error: true`.
#[tokio::test]
async fn test_error_responses_have_is_error_true() {
    let (server, _pool) = setup_server().await;

    // Call quilt_get_page_blocks without page_name → Err.
    let response = server
        .handle_request(McpRequest::CallTool {
            params: quilt_mcp::protocol::CallToolParams {
                name: "quilt_get_page_blocks".to_string(),
                arguments: serde_json::json!({}),
            },
        })
        .await;
    let result = match response {
        quilt_mcp::protocol::McpResponse::ToolsCall(r) => r,
        _ => panic!("Expected ToolsCall response"),
    };

    // is_error on the wire (ToolsCallResult) must be true.
    assert!(
        result.is_error.unwrap(),
        "is_error must be true on error path"
    );

    // Evidence.is_error must be true.
    let meta = result._meta.expect("error path must carry _meta");
    let ev = meta.evidence.expect("error path must carry _meta.evidence");
    assert!(ev.is_error, "evidence.is_error must be true on error path");
    assert_eq!(ev.tool_name, "quilt_get_page_blocks");
}

/// Contract sub-test D: resource reads also carry evidence.
#[tokio::test]
async fn test_resource_reads_carry_evidence() {
    let (server, _pool) = setup_server().await;

    let resources_response = server.handle_request(McpRequest::ListResources).await;
    let resources = match resources_response {
        quilt_mcp::protocol::McpResponse::ResourcesList(r) => r.resources,
        _ => panic!("Expected ResourcesList response"),
    };
    assert_eq!(resources.len(), 4);

    let mut missing = Vec::new();
    for res in &resources {
        let response = server
            .handle_request(McpRequest::ReadResource {
                params: quilt_mcp::protocol::ReadResourceParams {
                    uri: res.uri.clone(),
                },
            })
            .await;
        let result = match response {
            quilt_mcp::protocol::McpResponse::ResourcesRead(r) => r,
            _ => {
                missing.push(format!("{}: non-ResourcesRead response", res.uri));
                continue;
            }
        };
        let meta = match result._meta {
            Some(m) => m,
            None => {
                missing.push(format!("{}: missing _meta", res.uri));
                continue;
            }
        };
        let ev = match meta.evidence {
            Some(e) => e,
            None => {
                missing.push(format!("{}: missing _meta.evidence", res.uri));
                continue;
            }
        };
        // For resources, tool_name is the URI.
        if ev.tool_name != res.uri {
            missing.push(format!(
                "{}: evidence.tool_name = {} (expected {})",
                res.uri, ev.tool_name, res.uri
            ));
        }
    }

    assert!(
        missing.is_empty(),
        "Resource(s) missing _meta.evidence:\n  - {}",
        missing.join("\n  - ")
    );
}
