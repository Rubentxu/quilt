//! MCP Notifications definitions

use serde::{Deserialize, Serialize};

/// A notification from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub method: String,
    pub params: NotificationParams,
}

/// Notification params
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationParams {
    pub event: NotificationEvent,
}

/// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationEvent {
    #[serde(rename = "block_changed")]
    BlockChanged(BlockChangedEvent),
    #[serde(rename = "page_created")]
    PageCreated(PageCreatedEvent),
    #[serde(rename = "backlinks_changed")]
    BacklinksChanged(BacklinksChangedEvent),
}

/// Block changed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockChangedEvent {
    pub block_id: String,
    pub change_type: ChangeType,
}

/// Change type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Created,
    Updated,
    Deleted,
}

/// Page created event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageCreatedEvent {
    pub page_id: String,
    pub page_name: String,
}

/// Backlinks changed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinksChangedEvent {
    pub block_id: String,
}
