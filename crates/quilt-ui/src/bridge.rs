//! Bridge to Tauri backend commands
//!
//! These are typed wrappers around Tauri's `invoke` IPC.
//! In development (non-Tauri browser via Trunk), they return mock data.

use serde::{Deserialize, Serialize};

/// Invoke a Tauri command via window.__TAURI__.invoke (JavaScript interop).
///
/// In development (non-Tauri browser via Trunk), the __TAURI__ global doesn't exist,
/// so we return BridgeError::Unavailable which triggers mock data fallbacks.
#[cfg(target_arch = "wasm32")]
pub async fn invoke<T: for<'de> Deserialize<'de>>(
    cmd: &str,
    args: &serde_json::Value,
) -> Result<T, BridgeError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use web_sys::{js_sys::Reflect, Window};

    let window: Window =
        web_sys::window().ok_or_else(|| BridgeError::Unavailable("No window".into()))?;

    // Get window.__TAURI__
    let tauri = Reflect::get(&window, &JsValue::from_str("__TAURI__"))
        .map_err(|_| BridgeError::Unavailable("__TAURI__ not available".into()))?;
    if tauri.is_undefined() || tauri.is_null() {
        return Err(BridgeError::Unavailable(
            "Tauri not available (dev mode)".into(),
        ));
    }

    // Get window.__TAURI__.invoke as a JS Function
    let invoke_fn_val = Reflect::get(&tauri, &JsValue::from_str("invoke"))
        .map_err(|e| BridgeError::TauriError(format!("invoke not found: {:?}", e)))?;
    let invoke_fn: js_sys::Function = invoke_fn_val.unchecked_into();

    // Build args array: [cmd, args]
    let args_arr = js_sys::Array::new();
    args_arr.push(&JsValue::from_str(cmd));
    args_arr.push(&serde_wasm_bindgen::to_value(args).unwrap_or(JsValue::NULL));

    // Call: window.__TAURI__.invoke(cmd, args) -> Promise
    let promise_val = invoke_fn
        .apply(&JsValue::NULL, &args_arr)
        .map_err(|e| BridgeError::TauriError(format!("invoke call failed: {:?}", e)))?;
    let promise: js_sys::Promise = promise_val.unchecked_into();

    // Await the Promise
    let js_future = wasm_bindgen_futures::JsFuture::from(promise);
    let result: JsValue = js_future
        .await
        .map_err(|e| BridgeError::TauriError(format!("invoke await failed: {:?}", e)))?;

    // Deserialize result
    serde_wasm_bindgen::from_value(result)
        .map_err(|e| BridgeError::TauriError(format!("deserialization failed: {}", e)))
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke<T: for<'de> Deserialize<'de>>(
    _cmd: &str,
    _args: &serde_json::Value,
) -> Result<T, BridgeError> {
    Err(BridgeError::Unavailable("Not running in WASM".into()))
}

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
    /// Parent block ID (None for top-level blocks)
    pub parent_id: Option<String>,
    /// Lexicographic order among siblings (fractional indexing)
    pub order: f64,
    /// Indentation level (1-indexed)
    pub level: u8,
    /// Whether this block is collapsed in the outliner
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

/// Error from the bridge
#[derive(Debug, Clone)]
pub enum BridgeError {
    TauriError(String),
    JsonError(String),
    Unavailable(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::TauriError(s) => write!(f, "TauriError: {}", s),
            BridgeError::JsonError(s) => write!(f, "JsonError: {}", s),
            BridgeError::Unavailable(s) => write!(f, "Unavailable: {}", s),
        }
    }
}

impl From<serde_json::Error> for BridgeError {
    fn from(e: serde_json::Error) -> Self {
        BridgeError::JsonError(e.to_string())
    }
}

// ── Mock data helpers ──────────────────────────────────────────────────────────

/// Mock blocks for dev mode
fn mock_todays_blocks() -> Vec<BlockDto> {
    vec![
        BlockDto {
            id: "mock-1".into(),
            page_id: "mock-page".into(),
            page_name: Some("Welcome".into()),
            content: "Welcome to Quilt! Start journaling today.".into(),
            marker: None,
            priority: None,
            parent_id: None,
            order: 100.0,
            level: 1,
            collapsed: false,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        },
        BlockDto {
            id: "mock-2".into(),
            page_id: "mock-page".into(),
            page_name: Some("Welcome".into()),
            content: "Install cargo-tauri and trunk to enable the full experience.".into(),
            marker: Some("todo".into()),
            priority: Some("a".into()),
            parent_id: None,
            order: 200.0,
            level: 1,
            collapsed: false,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        },
        BlockDto {
            id: "mock-3".into(),
            page_id: "mock-page".into(),
            page_name: Some("Welcome".into()),
            content: "This is a nested task".into(),
            marker: Some("doing".into()),
            priority: Some("b".into()),
            parent_id: Some("mock-2".into()),
            order: 100.0,
            level: 2,
            collapsed: false,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        },
        BlockDto {
            id: "mock-4".into(),
            page_id: "mock-page".into(),
            page_name: Some("Welcome".into()),
            content: "Another top-level note".into(),
            marker: Some("done".into()),
            priority: None,
            parent_id: None,
            order: 300.0,
            level: 1,
            collapsed: false,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        },
    ]
}

/// Mock pages for dev mode
fn mock_pages() -> Vec<PageDto> {
    vec![
        PageDto {
            id: "mock-1".into(),
            name: "welcome".into(),
            title: Some("Welcome to Quilt".into()),
            journal: false,
            journal_day: None,
            created_at: "2024-01-01T00:00:00Z".into(),
        },
        PageDto {
            id: "mock-2".into(),
            name: "journal-2024-01-15".into(),
            title: Some("January 15, 2024".into()),
            journal: true,
            journal_day: Some(20240115),
            created_at: "2024-01-15T00:00:00Z".into(),
        },
    ]
}

/// Mock query block result
fn mock_query_result(dsl: &str, limit: usize) -> Vec<BlockDto> {
    vec![BlockDto {
        id: "mock-block-1".into(),
        page_id: "mock-page".into(),
        page_name: Some("Mock Page".into()),
        content: format!("Query result for: {} (limit: {})", dsl, limit),
        marker: None,
        priority: None,
        parent_id: None,
        order: 100.0,
        level: 1,
        collapsed: false,
        created_at: "2024-01-01T00:00:00Z".into(),
        updated_at: "2024-01-01T00:00:00Z".into(),
    }]
}

/// Mock search result
fn mock_search_result(query: &str, _limit: usize) -> Vec<SearchResultDto> {
    vec![SearchResultDto {
        block_id: "mock-search-1".into(),
        page_id: "mock-page".into(),
        page_name: "Mock Page".into(),
        content: format!("Search result for: {}", query),
        snippet: Some(format!("...{}...", query)),
        score: 0.95,
    }]
}

/// Mock cognitive map for dev mode
fn mock_cognitive_map() -> CognitiveMapDto {
    CognitiveMapDto {
        total_clusters: 12,
        total_frontiers: 5,
        total_gaps: 3,
        pages_analyzed: 8,
        available: false,
    }
}

// ── Morning Briefing DTOs ───────────────────────────────────────────────────────

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

/// Knowledge evolution insight from tracked topics
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

/// Mock morning briefing for dev mode
fn mock_morning_briefing() -> MorningBriefingDto {
    MorningBriefingDto {
        cognitive_pulse: CognitivePulseDto {
            total_pages: 42,
            total_blocks: 187,
            clusters: 12,
            frontiers: 5,
            gaps: 3,
        },
        serendipity_highlights: vec![
            SerendipityHighlightDto {
                from_page: "Rust Async Patterns".to_string(),
                to_page: "Tokio Internals".to_string(),
                connection_type: "temporal".to_string(),
                confidence: 0.87,
            },
            SerendipityHighlightDto {
                from_page: "Memory Models".to_string(),
                to_page: "Concurrent Algorithms".to_string(),
                connection_type: "semantic".to_string(),
                confidence: 0.76,
            },
        ],
        decay_alerts: vec![
            DecayAlertDto {
                page_name: "Old Project Notes".to_string(),
                last_modified: "2024-01-15T10:30:00Z".to_string(),
                days_stale: 30,
            },
            DecayAlertDto {
                page_name: "Deprecated API Guide".to_string(),
                last_modified: "2024-02-01T14:00:00Z".to_string(),
                days_stale: 14,
            },
        ],
        stats: BriefingStatsDto {
            pages_created_today: 3,
            blocks_created_today: 15,
            queries_run_today: 8,
        },
        knowledge_evolution: vec![
            KnowledgeEvolutionDto {
                topic: "Rust async programming".to_string(),
                belief_changes: 3,
                reinforced_count: 2,
                abandoned_count: 1,
            },
            KnowledgeEvolutionDto {
                topic: "Distributed systems".to_string(),
                belief_changes: 1,
                reinforced_count: 1,
                abandoned_count: 0,
            },
        ],
        generated_at: "2024-03-20T08:00:00Z".to_string(),
        degraded: false,
    }
}

// ── Cognitive IPC commands ────────────────────────────────────────────────────

/// Response for cognitive commands indicating availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityDto {
    pub available: bool,
    pub message: Option<String>,
}

/// Get cognitive map - wired to `cognitive_mirror` Tauri command
pub async fn get_cognitive_map(page_name: &str) -> Result<CognitiveMapDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "page_name": page_name });
        match invoke::<CognitiveMapDto>("cognitive_mirror", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(mock_cognitive_map()),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(mock_cognitive_map())
    }
}

/// Check if cognitive engine is available
pub async fn cognitive_available() -> Result<AvailabilityDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({});
        match invoke::<AvailabilityDto>("cognitive_available", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(AvailabilityDto {
                available: false,
                message: Some("Cognitive engine not configured (dev mode)".to_string()),
            }),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(AvailabilityDto {
            available: false,
            message: Some("Cognitive engine not configured (dev mode)".to_string()),
        })
    }
}

/// Get serendipity connections - wired to `serendipity` Tauri command
pub async fn get_serendipity(
    since: Option<&str>,
    limit: Option<usize>,
    min_confidence: Option<f32>,
) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({
            "since": since,
            "limit": limit.unwrap_or(20),
            "min_confidence": min_confidence.unwrap_or(0.3)
        });
        match invoke::<serde_json::Value>("serendipity", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(serde_json::json!({
                "available": false,
                "message": "Serendipity engine not configured (dev mode)"
            })),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (since, limit, min_confidence);
        Ok(serde_json::json!({
            "available": false,
            "message": "Serendipity engine not configured (dev mode)"
        }))
    }
}

/// Get argument map for a page - wired to `argument_map` Tauri command
pub async fn get_argument_map(page_name: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "page_name": page_name });
        match invoke::<serde_json::Value>("argument_map", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(serde_json::json!({
                "available": false,
                "message": "Argument cartographer not configured (dev mode)"
            })),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(serde_json::json!({
            "available": false,
            "message": "Argument cartographer not configured (dev mode)"
        }))
    }
}

/// Get mental model for an agent - wired to `mental_model` Tauri command
pub async fn get_mental_model(agent_id: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "agent_id": agent_id });
        match invoke::<serde_json::Value>("mental_model", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(serde_json::json!({
                "available": false,
                "message": "Mental model gardener not configured (dev mode)"
            })),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = agent_id;
        Ok(serde_json::json!({
            "available": false,
            "message": "Mental model gardener not configured (dev mode)"
        }))
    }
}

/// Get morning briefing - wired to `morning_briefing` Tauri command
pub async fn get_morning_briefing() -> Result<MorningBriefingDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({});
        match invoke::<MorningBriefingDto>("morning_briefing", &args).await {
            Ok(dto) => Ok(dto),
            Err(BridgeError::Unavailable(_)) => Ok(mock_morning_briefing()),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(mock_morning_briefing())
    }
}

// ── Typed wrapper functions ────────────────────────────────────────────────────

/// Query blocks with DSL string - wired to `query_blocks` Tauri command
pub async fn query_blocks(dsl: &str, limit: usize) -> Result<Vec<BlockDto>, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "dsl": dsl, "limit": limit });
        match invoke::<Vec<BlockDto>>("query_blocks", &args).await {
            Ok(blocks) => Ok(blocks),
            Err(BridgeError::Unavailable(_)) => Ok(mock_query_result(dsl, limit)),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(mock_query_result(dsl, limit))
    }
}

/// Create a new block - wired to `create_block` Tauri command
pub async fn create_block(page_name: &str, content: &str) -> Result<BlockDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "page_name": page_name, "content": content });
        match invoke::<BlockDto>("create_block", &args).await {
            Ok(block) => Ok(block),
            Err(BridgeError::Unavailable(_)) => {
                // Mock response for dev mode
                Ok(BlockDto {
                    id: format!("mock-{}-{}", page_name, content.len()),
                    page_id: page_name.into(),
                    page_name: Some(page_name.into()),
                    content: content.into(),
                    marker: None,
                    priority: None,
                    parent_id: None,
                    order: 100.0,
                    level: 1,
                    collapsed: false,
                    created_at: "2024-01-01T00:00:00Z".into(),
                    updated_at: "2024-01-01T00:00:00Z".into(),
                })
            }
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (page_name, content);
        Ok(BlockDto {
            id: format!("mock-{}-{}", page_name, content.len()),
            page_id: page_name.into(),
            page_name: Some(page_name.into()),
            content: content.into(),
            marker: None,
            priority: None,
            parent_id: None,
            order: 100.0,
            level: 1,
            collapsed: false,
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        })
    }
}

/// Update an existing block - wired to `update_block` Tauri command
pub async fn update_block(
    id: &str,
    content: &str,
    parent_id: Option<&str>,
    order: Option<f64>,
    level: Option<u8>,
    collapsed: Option<bool>,
) -> Result<BlockDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({
            "id": id,
            "content": content,
            "parentId": parent_id,
            "order": order,
            "level": level,
            "collapsed": collapsed,
        });
        match invoke::<BlockDto>("update_block", &args).await {
            Ok(block) => Ok(block),
            Err(BridgeError::Unavailable(_)) => {
                // Mock response for dev mode
                Ok(BlockDto {
                    id: id.into(),
                    page_id: "mock-page".into(),
                    page_name: Some("mock".into()),
                    content: content.into(),
                    marker: None,
                    priority: None,
                    parent_id: parent_id.map(String::from),
                    order: order.unwrap_or(100.0),
                    level: level.unwrap_or(1),
                    collapsed: collapsed.unwrap_or(false),
                    created_at: "2024-01-01T00:00:00Z".into(),
                    updated_at: "2024-01-01T00:00:00Z".into(),
                })
            }
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (id, content, parent_id, order, level, collapsed);
        Ok(BlockDto {
            id: id.into(),
            page_id: "mock-page".into(),
            page_name: Some("mock".into()),
            content: content.into(),
            marker: None,
            priority: None,
            parent_id: parent_id.map(String::from),
            order: order.unwrap_or(100.0),
            level: level.unwrap_or(1),
            collapsed: collapsed.unwrap_or(false),
            created_at: "2024-01-01T00:00:00Z".into(),
            updated_at: "2024-01-01T00:00:00Z".into(),
        })
    }
}

/// Search blocks in the knowledge graph - wired to `search_blocks` Tauri command
pub async fn search_blocks(query: &str, limit: usize) -> Result<Vec<SearchResultDto>, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "query": query, "limit": limit });
        match invoke::<Vec<SearchResultDto>>("search_blocks", &args).await {
            Ok(results) => Ok(results),
            Err(BridgeError::Unavailable(_)) => Ok(mock_search_result(query, limit)),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(mock_search_result(query, limit))
    }
}

/// List all pages - wired to `list_pages` Tauri command
pub async fn list_pages() -> Result<Vec<PageDto>, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({});
        match invoke::<Vec<PageDto>>("list_pages", &args).await {
            Ok(pages) => Ok(pages),
            Err(BridgeError::Unavailable(_)) => Ok(mock_pages()),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(mock_pages())
    }
}

/// Get journal page for a specific date - wired to `get_journal` Tauri command
pub async fn get_journal(date: &str) -> Result<PageDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "date": date });
        match invoke::<PageDto>("get_journal", &args).await {
            Ok(page) => Ok(page),
            Err(BridgeError::Unavailable(_)) => {
                // Mock response for dev mode
                Ok(PageDto {
                    id: format!("mock-journal-{}", date),
                    name: format!("journal-{}", date),
                    title: Some(date.to_string()),
                    journal: true,
                    journal_day: Some(date.replace("-", "").parse().unwrap_or(0)),
                    created_at: "2024-01-01T00:00:00Z".into(),
                })
            }
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(PageDto {
            id: format!("mock-journal-{}", date),
            name: format!("journal-{}", date),
            title: Some(date.to_string()),
            journal: true,
            journal_day: Some(date.replace("-", "").parse().unwrap_or(0)),
            created_at: "2024-01-01T00:00:00Z".into(),
        })
    }
}

/// Get today's blocks - TODO: no corresponding Tauri command yet
///
/// This function queries for blocks created or modified today.
/// Currently returns mock data - needs a Tauri command implementation.
pub async fn get_todays_blocks() -> Result<Vec<BlockDto>, BridgeError> {
    // TODO: Wire to real Tauri backend (no command exists yet for "today's blocks")
    Ok(mock_todays_blocks())
}

/// Get all pages (alias for list_pages for backwards compatibility)
///
/// DEPRECATED: Use `list_pages()` instead. This function exists for
/// backwards compatibility with existing UI code.
pub async fn get_pages() -> Result<Vec<PageDto>, BridgeError> {
    list_pages().await
}

/// Query the agent for a specific page - wired to `query_agent` Tauri command
pub async fn query_agent(page_name: &str) -> Result<serde_json::Value, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({ "page_name": page_name });
        match invoke::<serde_json::Value>("query_agent", &args).await {
            Ok(result) => Ok(result),
            Err(BridgeError::Unavailable(_)) => Ok(serde_json::json!({
                "available": false,
                "message": "Agent not configured (dev mode)",
                "page_name": page_name
            })),
            Err(e) => Err(e),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = page_name;
        Ok(serde_json::json!({
            "available": false,
            "message": "Agent not configured (dev mode)",
            "page_name": page_name
        }))
    }
}

/// Graph View DTOs (mirrors backend types)
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

/// Get graph data - wired to `resource_graph` MCP via Tauri command
pub async fn get_graph_data() -> Result<GraphDataDto, BridgeError> {
    #[cfg(target_arch = "wasm32")]
    {
        let args = serde_json::json!({});
        invoke::<serde_json::Value>("resource_graph", &args)
            .await
            .map(|v| {
                // The MCP returns a JSON string, parse it
                serde_json::from_str(v.as_str().unwrap_or("{}")).unwrap_or(GraphDataDto {
                    nodes: vec![],
                    edges: vec![],
                    last_updated: String::new(),
                })
            })
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
