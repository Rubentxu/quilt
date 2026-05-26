//! HTTP bridge to Quilt backend
//!
//! Communicates with the Quilt server via REST API.

use gloo::net::http::Request;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub order: f64,
    pub level: u8,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub collapsed: bool,
    pub properties: serde_json::Value,
    pub refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub namespace: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: Option<String>,
    pub rank: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum BridgeError {
    Network(String),
    Parse(String),
    Server(u16, String),
    BlockNotFound(String),
    BlockHasChildren(String),
    ConcurrentEdit(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Network(e) => write!(f, "Network error: {}", e),
            BridgeError::Parse(e) => write!(f, "Parse error: {}", e),
            BridgeError::Server(code, msg) => write!(f, "Server error {}: {}", code, msg),
            BridgeError::BlockNotFound(id) => write!(f, "Block not found: {}", id),
            BridgeError::BlockHasChildren(id) => write!(f, "Block has children: {}", id),
            BridgeError::ConcurrentEdit(id) => write!(f, "Concurrent edit conflict: {}", id),
        }
    }
}

impl std::error::Error for BridgeError {}

const BASE_URL: &str = "http://127.0.0.1:3737/api/v1";

pub async fn get_page_blocks(page_name: &str) -> Result<Vec<BlockDto>, BridgeError> {
    let url = format!("{}/pages/{}/blocks", BASE_URL, page_name);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let blocks: Vec<BlockDto> = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(blocks)
}

pub async fn get_journal(day: i64) -> Result<PageDto, BridgeError> {
    let url = format!("{}/journal/{}", BASE_URL, day);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let page: PageDto = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(page)
}

pub async fn search(query: &str) -> Result<Vec<SearchResultDto>, BridgeError> {
    let url = format!("{}/search?q={}", BASE_URL, query);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let results: Vec<SearchResultDto> = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(results)
}

pub async fn create_block(
    page_name: &str,
    content: &str,
    parent_id: Option<&str>,
) -> Result<BlockDto, BridgeError> {
    let url = format!("{}/blocks", BASE_URL);
    let body = serde_json::json!({
        "page_name": page_name,
        "content": content,
        "parent_id": parent_id,
    });
    let resp = Request::post(&url)
        .json(&body)
        .map_err(|e| BridgeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let block: BlockDto = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(block)
}

pub async fn update_block(block_id: &str, content: &str) -> Result<BlockDto, BridgeError> {
    let url = format!("{}/blocks/{}", BASE_URL, block_id);
    let body = serde_json::json!({ "content": content });
    let resp = Request::patch(&url)
        .json(&body)
        .map_err(|e| BridgeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let block: BlockDto = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(block)
}

pub async fn delete_block(block_id: &str) -> Result<(), BridgeError> {
    let url = format!("{}/blocks/{}", BASE_URL, block_id);
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        let msg = resp.text().await.unwrap_or_default();
        if resp.status() == 409 {
            return Err(BridgeError::BlockHasChildren(block_id.to_string()));
        }
        return Err(BridgeError::Server(resp.status(), msg));
    }
    Ok(())
}

pub async fn move_block(
    block_id: &str,
    new_parent_id: Option<&str>,
    new_order: f64,
) -> Result<BlockDto, BridgeError> {
    let url = format!("{}/blocks/{}/move", BASE_URL, block_id);
    let body = serde_json::json!({
        "new_parent_id": new_parent_id,
        "order": new_order,
    });
    let resp = Request::put(&url)
        .json(&body)
        .map_err(|e| BridgeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BridgeError::Server(
            resp.status(),
            resp.text().await.unwrap_or_default(),
        ));
    }
    let block: BlockDto = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(block)
}

pub async fn list_pages() -> Result<Vec<PageDto>, BridgeError> {
    let url = format!("{}/pages", BASE_URL);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    let pages: Vec<PageDto> = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(pages)
}

/// A backlink DTO returned by the page backlinks endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkDto {
    pub source_block_id: String,
    pub source_page_name: String,
    pub content_preview: String,
}

/// Get all backlinks for a page (blocks that reference this page)
pub async fn get_page_backlinks(page_name: &str) -> Result<Vec<BacklinkDto>, BridgeError> {
    let url = format!("{}/pages/{}/backlinks", BASE_URL, page_name);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(BridgeError::Server(resp.status(), msg));
    }
    let backlinks: Vec<BacklinkDto> = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(backlinks)
}

/// Get all unlinked references for a page
///
/// Returns blocks whose content text mentions the page name but do NOT
/// have an explicit `[[page]]` link.
pub async fn get_page_unlinked_references(page_name: &str) -> Result<Vec<BacklinkDto>, BridgeError> {
    let url = format!("{}/pages/{}/unlinked-references", BASE_URL, page_name);
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(BridgeError::Server(resp.status(), msg));
    }
    let refs: Vec<BacklinkDto> = resp
        .json()
        .await
        .map_err(|e| BridgeError::Parse(e.to_string()))?;
    Ok(refs)
}
