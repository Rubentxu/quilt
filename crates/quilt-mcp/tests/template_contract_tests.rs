//! Integration tests for the Template Contract MCP tools (Q030).
//!
//! Covers the 3 new MCP-only tools added by this change:
//! - `quilt_get_template_contract` — fetch the contract for one template
//! - `quilt_list_templates_with_contracts` — list templates + their contracts
//! - `quilt_apply_template_with_contract` — apply template respecting contract
//!
//! These tests run through the full MCP server with an in-memory
//! SQLite store so they exercise the wire format, the
//! `TemplateContract` validation, and the `TemplateUseCases` lookups
//! end-to-end.

use std::collections::HashMap;
use std::sync::Arc;

use quilt_application::templates::contract::{
    ApplyTemplateWithContractUseCase, ApplyTemplateWithContractUseCaseImpl,
};
use quilt_application::templates::reapply::{ReapplyTemplateUseCase, ReapplyTemplateUseCaseImpl};
use quilt_application::services::ref_service::{RefService, RefServiceTrait};
use quilt_application::use_cases::{
    BlockUseCases, BlockUseCasesImpl, PageUseCases, PageUseCasesImpl, SearchUseCasesImpl,
    TemplateUseCases, TemplateUseCasesImpl,
};
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository, RefRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};
use quilt_infrastructure::database::sqlite::connection;
use quilt_infrastructure::database::sqlite::repositories::{
    SqliteBlockRepository, SqlitePageRepository, SqliteRefRepository,
};
use quilt_mcp::McpServer;
use quilt_mcp::handlers::block::BlockToolHandler;
use quilt_mcp::handlers::page::PageToolHandler;
use quilt_mcp::handlers::template::TemplateToolHandler;
use quilt_mcp::protocol::{CallToolParams, McpRequest};
use quilt_search::SearchService;
use sqlx::SqlitePool;

// ── Helpers ────────────────────────────────────────────────────────

struct TestWorld {
    server: McpServer,
    #[allow(dead_code)]
    pool: SqlitePool,
    page_repo: Arc<SqlitePageRepository>,
    block_repo: Arc<SqliteBlockRepository>,
}

async fn setup_world() -> TestWorld {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory DB");
    connection::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let block_repo = Arc::new(SqliteBlockRepository::new(pool.clone()));
    let page_repo = Arc::new(SqlitePageRepository::new(pool.clone()));
    let ref_repo = Arc::new(SqliteRefRepository::new(pool.clone()));
    let ref_service: Arc<dyn RefServiceTrait> = Arc::new(RefService::new(ref_repo));

    let block_use_cases: Arc<dyn BlockUseCases> = Arc::new(BlockUseCasesImpl::new(
        block_repo.clone(),
        page_repo.clone(),
        ref_service,
    ));
    let page_use_cases: Arc<dyn PageUseCases> =
        Arc::new(PageUseCasesImpl::new(page_repo.clone(), block_repo.clone()));
    let template_use_cases: Arc<dyn TemplateUseCases> = Arc::new(TemplateUseCasesImpl::new(
        page_repo.clone(),
        block_repo.clone(),
    ));
    let concrete_tuc: TemplateUseCasesImpl<SqlitePageRepository, SqliteBlockRepository> =
        TemplateUseCasesImpl::new(page_repo.clone(), block_repo.clone());
    let reapply_use_cases: Arc<dyn ReapplyTemplateUseCase> = Arc::new(
        ReapplyTemplateUseCaseImpl::new(Arc::new(concrete_tuc), block_repo.clone()),
    );
    let apply_with_contract_use_cases: Arc<dyn ApplyTemplateWithContractUseCase> = Arc::new(
        ApplyTemplateWithContractUseCaseImpl::new(template_use_cases.clone(), block_repo.clone()),
    );

    let _search_use_cases = Arc::new(
        SearchUseCasesImpl::new()
            .with_search_service(Arc::new(SearchService::new(Arc::new(pool.clone())))),
    );

    let block_handler = BlockToolHandler::new(block_use_cases.clone());
    let page_handler = PageToolHandler::new(page_use_cases.clone());
    let template_handler = TemplateToolHandler::new(
        template_use_cases.clone(),
        reapply_use_cases.clone(),
        apply_with_contract_use_cases.clone(),
    );

    let server = McpServer::new(
        vec![
            Box::new(block_handler),
            Box::new(page_handler),
            Box::new(template_handler),
        ],
        vec![], // no resource providers needed for these tests
    );

    TestWorld {
        server,
        pool,
        page_repo,
        block_repo,
    }
}

async fn insert_template(world: &TestWorld, name: &str, block_props: Vec<(&str, &str)>) -> Uuid {
    let page = Page::new(PageCreate {
        name: format!("template/{}", name),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: HashMap::new(),
    })
    .unwrap();
    let page_id = page.id;
    world.page_repo.insert(&page).await.unwrap();

    let mut block = Block::new(BlockCreate {
        page_id,
        content: "Template content".to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: HashMap::new(),
    })
    .unwrap();
    for (k, v) in block_props {
        block
            .properties
            .insert(k.to_string(), PropertyValue::String(v.to_string()));
    }
    world.block_repo.insert(&block).await.unwrap();
    page_id
}

async fn insert_target_block(world: &TestWorld, content: &str) -> Uuid {
    let page = Page::new(PageCreate {
        name: "Some Page".to_string(),
        title: None,
        namespace_id: None,
        journal_day: None,
        format: BlockFormat::Markdown,
        file_id: None,
        properties: HashMap::new(),
    })
    .unwrap();
    let page_id = page.id;
    world.page_repo.insert(&page).await.unwrap();

    let block = Block::new(BlockCreate {
        page_id,
        content: content.to_string(),
        parent_id: None,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        block_type: BlockType::Paragraph,
        properties: HashMap::new(),
    })
    .unwrap();
    let block_id = block.id;
    world.block_repo.insert(&block).await.unwrap();
    block_id
}

async fn call_tool(server: &McpServer, name: &str, args: serde_json::Value) -> serde_json::Value {
    let response = server
        .handle_request(McpRequest::CallTool {
            params: CallToolParams {
                name: name.to_string(),
                arguments: args,
            },
        })
        .await;
    match response {
        quilt_mcp::protocol::McpResponse::ToolsCall(result) => {
            let text = match &result.content[0] {
                quilt_mcp::protocol::ContentBlock::Text { text } => text.clone(),
                _ => panic!("expected text content"),
            };
            serde_json::from_str(&text).expect("tool result should be JSON")
        }
        _ => panic!("expected ToolsCall response"),
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[tokio::test]
async fn list_tools_includes_three_contract_tools() {
    let world = setup_world().await;
    let response = world.server.handle_request(McpRequest::ListTools).await;
    match response {
        quilt_mcp::protocol::McpResponse::ToolsList(result) => {
            let names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
            assert!(
                names.contains(&"quilt_get_template_contract"),
                "missing quilt_get_template_contract"
            );
            assert!(
                names.contains(&"quilt_list_templates_with_contracts"),
                "missing quilt_list_templates_with_contracts"
            );
            assert!(
                names.contains(&"quilt_apply_template_with_contract"),
                "missing quilt_apply_template_with_contract"
            );
        }
        _ => panic!("expected ToolsList response"),
    }
}

#[tokio::test]
async fn get_template_contract_returns_contract() {
    let world = setup_world().await;
    let _tpl_id = insert_template(
        &world,
        "reference",
        vec![("title", "Untitled"), ("status", "todo")],
    )
    .await;

    let result = call_tool(
        &world.server,
        "quilt_get_template_contract",
        serde_json::json!({"template_id": _tpl_id.to_string()}),
    )
    .await;

    // The tool returns the contract as JSON.
    assert!(
        result.get("template_id").is_some(),
        "result missing template_id"
    );
    let required = result.get("required_properties").and_then(|v| v.as_array());
    assert!(required.is_some(), "missing required_properties");
    let reqs: Vec<String> = required
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(reqs.contains(&"title".to_string()));
    assert!(reqs.contains(&"status".to_string()));
}

#[tokio::test]
async fn get_template_contract_unknown_id_returns_error() {
    let world = setup_world().await;
    let result = call_tool(
        &world.server,
        "quilt_get_template_contract",
        serde_json::json!({"template_id": Uuid::new_v4().to_string()}),
    )
    .await;
    assert!(
        result.get("error").is_some(),
        "unknown template_id should return error: {result:?}"
    );
    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "template_not_found");
}

#[tokio::test]
async fn list_templates_with_contracts_returns_all() {
    let world = setup_world().await;
    insert_template(&world, "reference", vec![("title", "x")]).await;
    insert_template(&world, "documentation", vec![("body", "y")]).await;

    let result = call_tool(
        &world.server,
        "quilt_list_templates_with_contracts",
        serde_json::json!({}),
    )
    .await;

    let count = result.get("count").and_then(|v| v.as_u64()).unwrap();
    assert_eq!(count, 2, "expected 2 templates");

    let entries = result.get("templates").and_then(|v| v.as_array()).unwrap();
    assert_eq!(entries.len(), 2);
    // Each entry should have both summary and contract fields.
    for entry in entries {
        assert!(entry.get("name").is_some());
        assert!(entry.get("contract").is_some());
    }
}

#[tokio::test]
async fn apply_template_with_contract_success() {
    let world = setup_world().await;
    let tpl_id = insert_template(
        &world,
        "reference",
        vec![("title", "Untitled"), ("status", "todo")],
    )
    .await;
    let block_id = insert_target_block(&world, "Some content").await;

    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": block_id.to_string(),
            "template_id": tpl_id.to_string(),
            "proposed": {
                "title": "My Reference",
                "status": "in-progress"
            }
        }),
    )
    .await;

    assert!(
        result.get("error").is_none(),
        "happy path should not return error: {result:?}"
    );
    let applied = result.get("applied").and_then(|v| v.as_array()).unwrap();
    let applied_strs: Vec<String> = applied
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    assert!(applied_strs.contains(&"title".to_string()));
    assert!(applied_strs.contains(&"status".to_string()));

    // Verify the block was actually updated in storage.
    let updated = world.block_repo.get_by_id(block_id).await.unwrap().unwrap();
    assert_eq!(
        updated.properties.get("title").unwrap().as_display_string(),
        "My Reference"
    );
}

#[tokio::test]
async fn apply_template_with_contract_rejects_missing_required() {
    let world = setup_world().await;
    let tpl_id = insert_template(
        &world,
        "reference",
        vec![("title", "Untitled"), ("status", "todo")],
    )
    .await;
    let block_id = insert_target_block(&world, "x").await;

    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": block_id.to_string(),
            "template_id": tpl_id.to_string(),
            "proposed": {
                "title": "Only title, missing status"
            }
        }),
    )
    .await;

    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "missing_required_property");
}

#[tokio::test]
async fn apply_template_with_contract_rejects_locked_mutation() {
    let world = setup_world().await;
    // Template has "template" set to "reference" — if user tries to
    // change it, the locked check rejects.
    let tpl_id = insert_template(
        &world,
        "reference",
        vec![
            ("title", "Untitled"),
            ("status", "todo"),
            ("template", "reference"),
        ],
    )
    .await;
    let block_id = insert_target_block(&world, "x").await;

    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": block_id.to_string(),
            "template_id": tpl_id.to_string(),
            "proposed": {
                "title": "x",
                "status": "y",
                "template": "HACKED"
            }
        }),
    )
    .await;

    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert!(
        err == "locked_property_changed" || err == "missing_required_property",
        "expected locked-related error, got {err}"
    );
}

#[tokio::test]
async fn apply_template_with_contract_rejects_version_mismatch() {
    let world = setup_world().await;
    let tpl_id = insert_template(
        &world,
        "reference",
        vec![("title", "Untitled"), ("status", "todo")],
    )
    .await;
    let block_id = insert_target_block(&world, "x").await;

    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": block_id.to_string(),
            "template_id": tpl_id.to_string(),
            "proposed": {
                "title": "x",
                "status": "y"
            },
            "caller_version": 99
        }),
    )
    .await;

    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "version_mismatch");
}

#[tokio::test]
async fn apply_template_with_contract_invalid_uuid_returns_error() {
    let world = setup_world().await;
    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": "not-a-uuid",
            "template_id": Uuid::new_v4().to_string(),
            "proposed": {}
        }),
    )
    .await;
    // The tool returns the error in-band as a JSON object with `error` key.
    assert!(result.get("error").is_some());
    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "invalid_argument");
}

#[tokio::test]
async fn get_template_contract_invalid_uuid_returns_error() {
    let world = setup_world().await;
    let result = call_tool(
        &world.server,
        "quilt_get_template_contract",
        serde_json::json!({"template_id": "not-a-uuid"}),
    )
    .await;
    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "invalid_argument");
}

#[tokio::test]
async fn apply_template_with_contract_missing_args_returns_error() {
    let world = setup_world().await;
    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({}),
    )
    .await;
    // Should be an `invalid_argument` (missing field).
    let err = result.get("error").and_then(|v| v.as_str()).unwrap();
    assert_eq!(err, "invalid_argument");
}

#[tokio::test]
async fn apply_template_with_contract_template_id_inferred_from_name() {
    // The tool should also accept a `template_name` (the short name)
    // and resolve it to the template id internally.
    let world = setup_world().await;
    let _tpl_id = insert_template(
        &world,
        "reference",
        vec![("title", "Untitled"), ("status", "todo")],
    )
    .await;
    let block_id = insert_target_block(&world, "x").await;

    let result = call_tool(
        &world.server,
        "quilt_apply_template_with_contract",
        serde_json::json!({
            "block_id": block_id.to_string(),
            "template_name": "reference",
            "proposed": {
                "title": "x",
                "status": "y"
            }
        }),
    )
    .await;

    assert!(
        result.get("error").is_none(),
        "name-based lookup should succeed: {result:?}"
    );
}
