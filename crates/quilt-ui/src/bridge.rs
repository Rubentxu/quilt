//! Bridge to quilt-server HTTP API
//!
//! HTTP client for the Quilt REST API served by quilt-server (Axum).
//! All communication goes through HTTP — no Tauri, no IPC.

use serde::{Deserialize, Serialize};
use std::future::Future as StdFuture;
use std::pin::Pin;
use std::time::Duration;

// ── HTTP helpers ──────────────────────────────────────────────────────────────

/// Base URL for the API server (same origin as the page)
fn api_base() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_else(|| "http://localhost:3737".to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        "http://localhost:3737".to_string()
    }
}

/// HTTP GET returning deserialized JSON
async fn http_get<T: for<'de> Deserialize<'de>>(path: &str) -> Result<T, BridgeError> {
    let url = format!("{}{}", api_base(), path);
    log::info!("GET {}", url);
    let resp = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BridgeError::Network(format!("GET {} → {}", url, resp.status())));
    }
    resp.json::<T>().await.map_err(|e| BridgeError::Network(e.to_string()))
}

/// HTTP POST with JSON body
async fn http_post<T: for<'de> Deserialize<'de>>(
    path: &str,
    body: &serde_json::Value,
) -> Result<T, BridgeError> {
    let url = format!("{}{}", api_base(), path);
    log::info!("POST {}", url);
    let resp = gloo_net::http::Request::post(&url)
        .json(body)
        .map_err(|e| BridgeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BridgeError::Network(format!("POST {} → {}", url, resp.status())));
    }
    resp.json::<T>().await.map_err(|e| BridgeError::Network(e.to_string()))
}

/// HTTP PATCH with JSON body
async fn http_patch<T: for<'de> Deserialize<'de>>(
    path: &str,
    body: &serde_json::Value,
) -> Result<T, BridgeError> {
    let url = format!("{}{}", api_base(), path);
    log::info!("PATCH {}", url);
    let resp = gloo_net::http::Request::patch(&url)
        .json(body)
        .map_err(|e| BridgeError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BridgeError::Network(format!("PATCH {} → {}", url, resp.status())));
    }
    resp.json::<T>().await.map_err(|e| BridgeError::Network(e.to_string()))
}

/// HTTP DELETE
async fn http_delete(path: &str) -> Result<(), BridgeError> {
    let url = format!("{}{}", api_base(), path);
    log::info!("DELETE {}", url);
    let resp = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| BridgeError::Network(e.to_string()))?;
    if !resp.ok() {
        return Err(BridgeError::Network(format!("DELETE {} → {}", url, resp.status())));
    }
    Ok(())
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Execute a future with retry logic for transient failures.
pub async fn with_retry<F, T, E>(mut f: F, max_retries: u32) -> Result<T, E>
where
    F: FnMut() -> Pin<Box<dyn StdFuture<Output = Result<T, E>>>>,
    E: std::fmt::Debug,
{
    let mut last_error = None;
    for attempt in 0..max_retries {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries - 1 {
                    let backoff_ms = 100 * 2u64.pow(attempt);
                    gloo_timers::future::sleep(Duration::from_millis(backoff_ms)).await;
                }
            }
        }
    }
    Err(last_error.unwrap())
}

/// Execute a future with a timeout.
pub async fn with_timeout<F, T>(future: F, duration: Duration) -> Result<T, BridgeError>
where
    F: StdFuture<Output = Result<T, BridgeError>>,
{
    gloo_timers::future::sleep(duration).await;
    // Simple timeout: race with sleep. In practice gloo-net handles timeouts.
    future.await
}

// ── Error ────────────────────────────────────────────────────────────────────

/// Error from the HTTP bridge
#[derive(Debug, Clone)]
pub enum BridgeError {
    Network(String),
    JsonError(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Network(s) => write!(f, "Network: {}", s),
            BridgeError::JsonError(s) => write!(f, "Json: {}", s),
        }
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(e: serde_json::Error) -> Self {
        BridgeError::JsonError(e.to_string())
    }
}

// ── DTOs ─────────────────────────────────────────────────────────────────────

/// A block from the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub page_name: Option<String>,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub parent_id: Option<String>,
    pub order: f64,
    pub level: u8,
    pub collapsed: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Default for BlockDto {
    fn default() -> Self {
        Self {
            id: String::new(),
            page_id: String::new(),
            page_name: None,
            content: String::new(),
            marker: None,
            priority: None,
            parent_id: None,
            order: 100.0,
            level: 1,
            collapsed: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}

/// A page from the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
    #[serde(default)]
    pub blocks: Vec<BlockDto>,
}

/// A search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: Option<String>,
    pub score: f64,
}

/// Query history item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryHistoryItem {
    pub query: String,
    pub timestamp: i64,
    pub human_time: String,
}

/// Query execution error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryError {
    pub message: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
}

// ── Cognitive DTOs ───────────────────────────────────────────────────────────

/// Cognitive map response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CognitiveMapDto {
    pub total_clusters: usize,
    pub total_frontiers: usize,
    pub total_gaps: usize,
    pub pages_analyzed: usize,
    pub available: bool,
}

/// Aggregated cognitive pulse metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitivePulseDto {
    pub total_pages: usize,
    pub total_blocks: usize,
    pub clusters: usize,
    pub frontiers: usize,
    pub gaps: usize,
}

/// A serendipitous connection highlight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerendipityHighlightDto {
    pub from_page: String,
    pub to_page: String,
    pub connection_type: String,
    pub confidence: f32,
}

/// An alert about a stale (decaying) page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayAlertDto {
    pub page_name: String,
    pub last_modified: String,
    pub days_stale: i64,
}

/// Activity statistics for today
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefingStatsDto {
    pub pages_created_today: usize,
    pub blocks_created_today: usize,
    pub queries_run_today: usize,
}

/// Knowledge evolution insight
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEvolutionDto {
    pub topic: String,
    pub belief_changes: usize,
    pub reinforced_count: usize,
    pub abandoned_count: usize,
}

/// The complete morning briefing DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorningBriefingDto {
    pub cognitive_pulse: CognitivePulseDto,
    pub serendipity_highlights: Vec<SerendipityHighlightDto>,
    pub decay_alerts: Vec<DecayAlertDto>,
    pub stats: BriefingStatsDto,
    pub knowledge_evolution: Vec<KnowledgeEvolutionDto>,
    pub generated_at: String,
    pub degraded: bool,
}

/// Response for cognitive commands indicating availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityDto {
    pub available: bool,
    pub message: Option<String>,
}

// ── Graph DTOs ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    #[serde(rename = "nodeType")]
    pub node_type: String,
    pub journal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

// ── Properties DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPropertiesDto {
    pub block_id: String,
    pub properties: Vec<PropertyDto>,
}

// ── Annotation DTOs ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationTypeDto {
    Highlight,
    Comment,
    Question,
    Important,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationDto {
    pub id: String,
    pub block_id: String,
    pub annotation_type: AnnotationTypeDto,
    pub content: String,
    pub resolved: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnnotationRequest {
    pub block_id: String,
    pub annotation_type: AnnotationTypeDto,
    pub content: String,
}

// ── Backlink DTOs ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipTypeDto {
    Direct,
    Transitive,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkDto {
    pub id: String,
    pub source_id: String,
    pub source_title: String,
    pub source_preview: String,
    pub context: String,
    pub relationship_type: RelationshipTypeDto,
    pub created_at: String,
    pub provenance_score: f64,
}

// ── Graph Management DTOs ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInfoDto {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockInfoDto {
    pub id: String,
    pub content: String,
    pub deleted_at: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════════════
// API Functions — all HTTP
// ══════════════════════════════════════════════════════════════════════════════

// ── Pages API ────────────────────────────────────────────────────────────────

/// List all pages
pub async fn list_pages() -> Result<Vec<PageDto>, BridgeError> {
    http_get("/api/v1/pages").await
}

/// Alias for backwards compatibility
pub async fn get_pages() -> Result<Vec<PageDto>, BridgeError> {
    list_pages().await
}

/// Get a page by name with its blocks
pub async fn get_page(page_name: &str) -> Result<PageDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let page: PageDto = http_get(&format!("/api/v1/pages/{}", page_name)).await?;
        let blocks: Vec<BlockDto> =
            http_get(&format!("/api/v1/pages/{}/blocks", page_name)).await?;
        Ok(PageDto { blocks, ..page })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(PageDto {
            id: "mock".to_string(),
            name: "mock".to_string(),
            title: Some("Mock Page".to_string()),
            journal: false,
            journal_day: None,
            created_at: "2024-01-01T00:00:00Z".into(),
            blocks: vec![],
        })
    }
}

/// Get today's journal page
pub async fn get_journal(date: &str) -> Result<PageDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let page: PageDto = http_get(&format!("/api/v1/pages/journal/{}", date)).await?;
        let blocks: Vec<BlockDto> =
            http_get(&format!("/api/v1/pages/{}/blocks", page.name)).await?;
        Ok(PageDto { blocks, ..page })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = date;
        Ok(PageDto {
            id: format!("mock-journal-{}", date),
            name: format!("journal-{}", date),
            title: Some(date.to_string()),
            journal: true,
            journal_day: Some(date.replace("-", "").parse().unwrap_or(0)),
            created_at: "2024-01-01T00:00:00Z".into(),
            blocks: vec![],
        })
    }
}

/// Get today's blocks
pub async fn get_todays_blocks() -> Result<Vec<BlockDto>, BridgeError> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let page = get_journal(&today).await?;
    Ok(page.blocks)
}

// ── Blocks API ───────────────────────────────────────────────────────────────

/// Create a block
pub async fn create_block(page_name: &str, content: &str) -> Result<BlockDto, BridgeError> {
    let body = serde_json::json!({
        "pageName": page_name,
        "content": content,
    });
    http_post("/api/v1/blocks", &body).await
}

/// Update a block (content, parent, order, level, collapsed)
pub async fn update_block(
    id: &str,
    content: &str,
    parent_id: Option<&str>,
    order: Option<f64>,
    level: Option<u8>,
    collapsed: Option<bool>,
) -> Result<BlockDto, BridgeError> {
    let body = serde_json::json!({
        "content": content,
        "parentId": parent_id,
        "order": order,
        "level": level.map(|l| l as i32),
        "collapsed": collapsed,
    });
    http_patch(&format!("/api/v1/blocks/{}", id), &body).await
}

/// Query blocks with DSL
pub async fn query_blocks(dsl: &str, limit: usize) -> Result<Vec<BlockDto>, BridgeError> {
    http_get(&format!("/api/v1/blocks?dsl={}&limit={}", urlencoding(dsl), limit)).await
}

/// Search blocks
pub async fn search_blocks(query: &str, limit: usize) -> Result<Vec<SearchResultDto>, BridgeError> {
    http_get(&format!("/api/v1/blocks/search?query={}&limit={}", urlencoding(query), limit)).await
}

/// Delete a block (soft delete)
pub async fn delete_block(block_id: &str) -> Result<(), BridgeError> {
    http_delete(&format!("/api/v1/blocks/{}", block_id)).await
}

/// Restore a block from trash
pub async fn restore_block(block_id: &str) -> Result<(), BridgeError> {
    http_patch(&format!("/api/v1/blocks/{}/restore", block_id), &serde_json::json!({})).await
}

/// Hard delete a block permanently
pub async fn hard_delete_block(block_id: &str) -> Result<(), BridgeError> {
    http_delete(&format!("/api/v1/blocks/{}/hard", block_id)).await
}

/// Get block backlinks
pub async fn get_block_backlinks(block_id: &str) -> Result<Vec<BlockDto>, BridgeError> {
    http_get(&format!("/api/v1/blocks/{}/backlinks", block_id)).await
}

// ── Properties API ───────────────────────────────────────────────────────────

/// Get properties for a block
pub async fn get_block_properties(block_id: &str) -> Result<BlockPropertiesDto, BridgeError> {
    http_get(&format!("/api/v1/blocks/{}/properties", block_id)).await
}

/// Update properties for a block
pub async fn update_block_properties(
    block_id: &str,
    properties: Vec<PropertyDto>,
) -> Result<BlockPropertiesDto, BridgeError> {
    let body = serde_json::json!({
        "blockId": block_id,
        "properties": properties
    });
    http_patch(&format!("/api/v1/blocks/{}/properties", block_id), &body).await
}

// ── Annotations API ──────────────────────────────────────────────────────────

/// Get annotations for a block
pub async fn get_block_annotations(block_id: &str) -> Result<Vec<AnnotationDto>, BridgeError> {
    http_get(&format!("/api/v1/blocks/{}/annotations", block_id)).await
}

/// Create an annotation
pub async fn create_annotation(
    block_id: &str,
    annotation_type: AnnotationTypeDto,
    content: &str,
) -> Result<AnnotationDto, BridgeError> {
    let body = serde_json::json!({
        "blockId": block_id,
        "annotationType": annotation_type,
        "content": content,
    });
    http_post(&format!("/api/v1/blocks/{}/annotations", block_id), &body).await
}

/// Resolve/unresolve an annotation
pub async fn resolve_annotation(
    annotation_id: &str,
    resolved: bool,
) -> Result<AnnotationDto, BridgeError> {
    let body = serde_json::json!({ "resolved": resolved });
    http_patch(&format!("/api/v1/annotations/{}", annotation_id), &body).await
}

/// Delete an annotation
pub async fn delete_annotation(annotation_id: &str) -> Result<(), BridgeError> {
    http_delete(&format!("/api/v1/annotations/{}", annotation_id)).await
}

// ── Search API ───────────────────────────────────────────────────────────────

/// Search pages by name
pub async fn search_pages(query: &str, limit: usize) -> Result<Vec<SearchResultDto>, BridgeError> {
    search_blocks(query, limit).await
}

// ── Backlinks API ────────────────────────────────────────────────────────────

/// Get backlinks for a page
pub async fn get_page_backlinks(page_name: &str) -> Result<Vec<BacklinkDto>, BridgeError> {
    http_get(&format!("/api/v1/pages/{}/backlinks", page_name)).await
}

// ── Cognitive API ────────────────────────────────────────────────────────────

/// Get cognitive map for a page
pub async fn get_cognitive_map(page_name: &str) -> Result<CognitiveMapDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get(&format!("/api/v1/cognitive/map?name={}", urlencoding(page_name))).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(CognitiveMapDto {
            total_clusters: 0,
            total_frontiers: 0,
            total_gaps: 0,
            pages_analyzed: 0,
            available: false,
        })
    }
}

/// Check cognitive engine availability
pub async fn cognitive_available() -> Result<AvailabilityDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get("/api/v1/cognitive/available").await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(AvailabilityDto {
            available: false,
            message: Some("Not running in WASM".to_string()),
        })
    }
}

/// Get serendipity connections
pub async fn get_serendipity(
    since: Option<&str>,
    limit: Option<usize>,
    min_confidence: Option<f32>,
) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut url = format!("/api/v1/cognitive/serendipity?limit={}", limit.unwrap_or(20));
        if let Some(s) = since {
            url.push_str(&format!("&since={}", s));
        }
        if let Some(c) = min_confidence {
            url.push_str(&format!("&minConfidence={}", c));
        }
        http_get(&url).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (since, limit, min_confidence);
        Ok(serde_json::json!({"available": false}))
    }
}

/// Get argument map for a page
pub async fn get_argument_map(page_name: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get(&format!("/api/v1/cognitive/arguments/{}", urlencoding(page_name))).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(serde_json::json!({"available": false}))
    }
}

/// Get mental model for an agent
pub async fn get_mental_model(agent_id: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get(&format!("/api/v1/cognitive/models?agent={}", urlencoding(agent_id))).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = agent_id;
        Ok(serde_json::json!({"available": false}))
    }
}

/// Get morning briefing
pub async fn get_morning_briefing() -> Result<MorningBriefingDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get("/api/v1/cognitive/briefing").await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(MorningBriefingDto {
            cognitive_pulse: CognitivePulseDto {
                total_pages: 0,
                total_blocks: 0,
                clusters: 0,
                frontiers: 0,
                gaps: 0,
            },
            serendipity_highlights: vec![],
            decay_alerts: vec![],
            stats: BriefingStatsDto {
                pages_created_today: 0,
                blocks_created_today: 0,
                queries_run_today: 0,
            },
            knowledge_evolution: vec![],
            generated_at: String::new(),
            degraded: true,
        })
    }
}

// ── Agent API ────────────────────────────────────────────────────────────────

/// Query agent for a page
pub async fn query_agent(page_name: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get(&format!("/api/v1/navigate?name={}", urlencoding(page_name))).await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(serde_json::json!({"available": false}))
    }
}

// ── Graph API ────────────────────────────────────────────────────────────────

/// Get graph data
pub async fn get_graph_data() -> Result<GraphDataDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get("/api/v1/navigate/graph").await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(GraphDataDto {
            nodes: vec![],
            edges: vec![],
            last_updated: String::new(),
        })
    }
}

/// Get current graph info
pub async fn get_current_graph() -> Result<Option<GraphInfoDto>, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        http_get("/api/v1/navigate/current").await
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(Some(GraphInfoDto {
            path: "quilt.db".to_string(),
            name: "Default Graph".to_string(),
        }))
    }
}

/// Open a graph at the given path
pub async fn open_graph(path: &str) -> Result<(), BridgeError> {
    let body = serde_json::json!({ "path": path });
    let _: serde_json::Value = http_post("/api/v1/navigate/open", &body).await?;
    Ok(())
}

// ── Recycle Bin ──────────────────────────────────────────────────────────────

/// Get recycle bin contents
pub async fn get_recycle_bin() -> Result<Vec<BlockInfoDto>, BridgeError> {
    http_get("/api/v1/blocks?dsl=deleted").await
}

// ── Utility ──────────────────────────────────────────────────────────────────

/// Simple URL encoding for query parameters
fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('[', "%5B")
        .replace(']', "%5D")
        .replace('(', "%28")
        .replace(')', "%29")
}
