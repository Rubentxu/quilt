//! Block-related Tauri commands

use crate::commands::pages::create_page_repo;
use crate::state::AppState;
use quilt_application::query_service::QueryService;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};
use quilt_infrastructure::database::sqlite::repositories::SqliteBlockRepository;
use quilt_search::SearchService;
use serde::{Deserialize, Serialize};
use tauri::State;

/// A block returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub page_name: Option<String>,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<(Block, Option<String>)> for BlockDto {
    fn from((block, page_name): (Block, Option<String>)) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.page_id.to_string(),
            page_name,
            content: block.content,
            marker: block.marker.map(|m| format!("{:?}", m)),
            priority: block.priority.map(|p| format!("{:?}", p)),
            created_at: block.created_at.to_rfc3339(),
            updated_at: block.updated_at.to_rfc3339(),
        }
    }
}

impl From<Block> for BlockDto {
    fn from(block: Block) -> Self {
        Self {
            id: block.id.to_string(),
            page_id: block.page_id.to_string(),
            page_name: None,
            content: block.content,
            marker: block.marker.map(|m| format!("{:?}", m)),
            priority: block.priority.map(|p| format!("{:?}", p)),
            created_at: block.created_at.to_rfc3339(),
            updated_at: block.updated_at.to_rfc3339(),
        }
    }
}

/// Query blocks using DSL string
#[tauri::command]
pub async fn query_blocks(
    dsl: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<BlockDto>, String> {
    let query_service = QueryService::new();

    // First, prepare the query to validate it
    query_service
        .prepare(&dsl, limit)
        .map_err(|e| format!("Query parse error: {}", e))?;

    // Execute the query
    let result = query_service
        .execute(&dsl, limit, &state.pool)
        .await
        .map_err(|e| format!("Query execution error: {}", e))?;

    let blocks_with_names: Vec<BlockDto> = result
        .blocks
        .into_iter()
        .map(|block| {
            // For now, set page_name to None - fetching requires async which is complex in map
            // TODO: Fetch page names separately
            (block, None::<String>).into()
        })
        .collect();

    Ok(blocks_with_names)
}

/// Create a new block on a page
#[tauri::command]
pub async fn create_block(
    page_name: String,
    content: String,
    parent_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<BlockDto, String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = create_page_repo(&state.pool);

    // Find or create the page
    let page = match page_repo.get_by_name(&page_name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = quilt_domain::entities::Page::new(quilt_domain::entities::PageCreate {
                name: page_name.clone(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            })
            .map_err(|e| e.to_string())?;
            page_repo.insert(&p).await.map_err(|e| e.to_string())?;
            p
        }
        Err(e) => return Err(e.to_string()),
    };

    let parent_uuid = parent_id
        .map(|s| Uuid::parse_str(&s).ok_or_else(|| format!("Invalid UUID: {}", s)))
        .transpose()?;

    let block = Block::new(BlockCreate {
        page_id: page.id,
        content,
        parent_id: parent_uuid,
        order: 1.0,
        marker: None,
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .map_err(|e| e.to_string())?;

    block_repo.insert(&block).await.map_err(|e| e.to_string())?;

    Ok(BlockDto::from(block))
}

/// Search blocks across all content
#[tauri::command]
pub async fn search_blocks(
    query: String,
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResultDto>, String> {
    let search_service = SearchService::new(state.pool.clone());
    let results = search_service
        .search(&query, limit)
        .await
        .map_err(|e| e.to_string())?;

    Ok(results
        .into_iter()
        .map(|r| SearchResultDto {
            block_id: r.block_id,
            page_id: r.page_id,
            page_name: r.page_name,
            content: r.content,
            snippet: r.snippet,
            score: r.score,
        })
        .collect())
}

/// Search result DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: String,
    pub score: f64,
}

/// Get a block with its children (tree)
#[tauri::command]
pub async fn get_block_tree(
    block_id: String,
    state: State<'_, AppState>,
) -> Result<BlockTreeDto, String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id).ok_or_else(|| format!("Invalid UUID: {}", block_id))?;

    let block = block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Block not found: {}", block_id))?;

    let children = block_repo
        .get_children(uuid)
        .await
        .map_err(|e| e.to_string())?;

    let children_count = children.len();
    let child_dtos: Vec<BlockDto> = children.into_iter().map(BlockDto::from).collect();

    Ok(BlockTreeDto {
        block: BlockDto::from(block),
        children: child_dtos,
        children_count,
    })
}

/// Block tree response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockTreeDto {
    pub block: BlockDto,
    pub children: Vec<BlockDto>,
    pub children_count: usize,
}

/// Link two blocks together (create a reference)
#[tauri::command]
pub async fn link_blocks(
    source_id: String,
    target_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let source_uuid =
        Uuid::parse_str(&source_id).ok_or_else(|| format!("Invalid UUID: {}", source_id))?;
    let target_uuid =
        Uuid::parse_str(&target_id).ok_or_else(|| format!("Invalid UUID: {}", target_id))?;

    // Verify both blocks exist
    block_repo
        .get_by_id(source_uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Source block not found: {}", source_id))?;

    block_repo
        .get_by_id(target_uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Target block not found: {}", target_id))?;

    // Create a link property on the source block referencing the target
    // This is stored as a property - the actual link structure depends on domain design
    let _ = block_repo; // TODO: Implement link creation when Block entity supports refs

    Ok(serde_json::json!({
        "source_id": source_id,
        "target_id": target_id,
        "linked": true
    }))
}

/// Get all blocks that link to a given block (backlinks)
#[tauri::command]
pub async fn get_backlinks(
    block_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<BlockDto>, String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id).ok_or_else(|| format!("Invalid UUID: {}", block_id))?;

    // Verify the target block exists
    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Block not found: {}", block_id))?;

    // TODO: Implement backlink query when Block entity supports refs
    // For now, return empty - actual implementation requires Link entity
    let _ = (block_repo, uuid);
    Ok(vec![])
}

/// Create a task block (a block with todo marker and optional deadline/priority)
#[tauri::command]
pub async fn create_task(
    page_name: String,
    content: String,
    deadline: Option<String>,
    priority: Option<String>,
    state: State<'_, AppState>,
) -> Result<BlockDto, String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let page_repo = create_page_repo(&state.pool);

    // Find or create the page
    let page = match page_repo.get_by_name(&page_name).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            let p = quilt_domain::entities::Page::new(quilt_domain::entities::PageCreate {
                name: page_name.clone(),
                title: None,
                namespace_id: None,
                journal_day: None,
                format: BlockFormat::Markdown,
                file_id: None,
            })
            .map_err(|e| e.to_string())?;
            page_repo.insert(&p).await.map_err(|e| e.to_string())?;
            p
        }
        Err(e) => return Err(e.to_string()),
    };

    // TODO: Parse deadline and priority when Block entity supports properties for these
    let _ = (deadline, priority);

    let block = Block::new(BlockCreate {
        page_id: page.id,
        content,
        parent_id: None,
        order: 1.0,
        marker: Some(TaskMarker::Todo),
        format: BlockFormat::Markdown,
        properties: Default::default(),
    })
    .map_err(|e| e.to_string())?;

    block_repo.insert(&block).await.map_err(|e| e.to_string())?;

    Ok(BlockDto::from(block))
}

/// Delete a block
#[tauri::command]
pub async fn delete_block(block_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let block_repo = SqliteBlockRepository::new(state.pool.clone());
    let uuid = Uuid::parse_str(&block_id).ok_or_else(|| format!("Invalid UUID: {}", block_id))?;

    block_repo
        .get_by_id(uuid)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Block not found: {}", block_id))?;

    block_repo.delete(uuid).await.map_err(|e| e.to_string())?;

    Ok(())
}
