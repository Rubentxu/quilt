//! Domain DTOs — canonical types shared across frontends
//!
//! These are extracted from `crates/quilt-ui/src/bridge.rs`.
//! They use serde for serialization across the WASM boundary.

use serde::{Deserialize, Serialize};

/// A block in the outliner. The canonical representation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub parent_id: Option<String>,
    pub content: String,
    /// Order within siblings (fractional indexing)
    pub order: f64,
    /// Indentation level (1 = top-level)
    pub level: u8,
    /// Task marker: todo, doing, done, now, later, cancelled
    pub marker: Option<String>,
    /// Priority: A, B, C
    pub priority: Option<String>,
    /// Whether block is collapsed
    pub collapsed: bool,
    /// Block properties as JSON
    #[serde(default)]
    pub properties: serde_json::Value,
    /// References to other blocks/pages (UUIDs)
    #[serde(default)]
    pub refs: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
}

/// A page containing blocks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub namespace: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// A search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: Option<String>,
    pub rank: Option<f64>,
}

/// A backlink from one block to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkDto {
    pub source_block_id: String,
    pub source_page_name: String,
    pub content_preview: String,
}

/// WASM boundary command — dispatched from React to Rust outliner core.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum OutlinerCommand {
    /// Set block content
    SetContent { block_id: String, content: String },
    /// Split block at cursor
    SplitBlock { block_id: String, cursor_pos: usize },
    /// Merge with previous block
    MergePrev { block_id: String },
    /// Merge with next block
    MergeNext { block_id: String },
    /// Indent block
    Indent { block_id: String },
    /// Outdent block
    Outdent { block_id: String },
    /// Move block to new position (drag-and-drop)
    MoveBlock {
        block_id: String,
        new_parent_id: String,
        new_order: f64,
    },
    /// Cycle block marker
    CycleMarker { block_id: String },
    /// Cycle block priority
    CyclePriority { block_id: String },
}

/// WASM boundary state snapshot — returned from Rust to React.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlinerState {
    pub blocks: Vec<BlockDto>,
    pub page: Option<PageDto>,
    pub can_undo: bool,
    pub can_redo: bool,
    /// Fowler-Noll-Vo hash of state for cache invalidation
    pub state_hash: u64,
}

/// Response from a dispatched command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResponse {
    pub accepted: bool,
    pub state_hash: u64,
    pub error: Option<String>,
}

/// Parsed inline content segments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Segment {
    Text(String),
    PageRef(String),
    BlockRef(String),
    Tag(String),
    Property { key: String, value: String },
    Bold(String),
    Italic(String),
    Code(String),
    Link { text: String, url: String },
    Strikethrough(String),
    Highlight(String),
    BoldItalic(String),
    Header { level: u8, text: String },
}

/// Result of parsing a block's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedContent {
    pub segments: Vec<Segment>,
    pub normalized: String,
}

/// Core error type for outliner operations.
#[derive(Debug, Clone)]
pub enum CoreError {
    NotFound(String),
    InvalidOperation(String),
    ValidationError(String),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::NotFound(msg) => write!(f, "Not found: {}", msg),
            CoreError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            CoreError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}
