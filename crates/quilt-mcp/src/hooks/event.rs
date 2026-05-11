//! Hook event types and payloads
//!
//! Defines all hook event types that plugins can subscribe to, along with
//! their corresponding payloads for passing data to plugin handlers.

use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// Priority level for hook execution ordering.
///
/// Higher priority values are executed first. Plugins with the same
/// priority are executed in registration order (FIFO).
///
/// # Priority Guidelines
///
/// - **100-999**: System/reserved priorities (e.g., audit logging)
/// - **50-99**: High-priority plugins (e.g., search indexing)
/// - **1-49**: Normal plugins (default)
/// - **0**: Default priority (used when not specified)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Priority(pub u8);

impl Priority {
    /// System priority for critical operations like audit logging.
    pub const SYSTEM: Priority = Priority(100);
    /// High priority for operations like search indexing.
    pub const HIGH: Priority = Priority(75);
    /// Normal/default priority for most plugins.
    pub const NORMAL: Priority = Priority(50);
    /// Low priority for non-critical plugins.
    pub const LOW: Priority = Priority(25);
    /// Default priority when not specified.
    pub const DEFAULT: Priority = Priority(0);

    /// Creates a priority with an explicit value (0-255).
    ///
    /// Values outside this range are clamped.
    pub fn new(value: u8) -> Self {
        Priority(value)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::DEFAULT
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Priority({})", self.0)
    }
}

/// A subscription declaration for a hook event type.
///
/// Returned by [`Plugin::subscribed_hooks()`](crate::plugin::Plugin::subscribed_hooks)
/// to declare which hook events the plugin wants to receive.
#[derive(Debug, Clone)]
pub struct HookSubscription {
    /// The kind of event to subscribe to
    pub event: HookEventKind,
    /// Priority for dispatch ordering (higher = earlier)
    pub priority: Priority,
    /// Optional filter to further restrict when the hook fires
    pub filter: Option<HookFilter>,
}

/// Optional filter to restrict when a hook fires.
///
/// This allows plugins to subscribe to a subset of events
/// rather than all events of a given kind.
#[derive(Debug, Clone, Default)]
pub struct HookFilter {
    /// Match only if block IDs contain any of these
    pub block_ids: Option<Vec<String>>,
    /// Match only if page IDs contain any of these
    pub page_ids: Option<Vec<String>>,
    /// Match only if content contains any of these substrings
    pub content_contains: Option<Vec<String>>,
}

/// Represents the different kinds of hook events that plugins can subscribe to.
///
/// This is used for registration and dispatch routing — plugins declare
/// which event kinds they want to receive via [`crate::plugin::Plugin::subscribed_hooks()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEventKind {
    /// Block was created, updated, or deleted
    BlockChanged,
    /// Page was created, updated, deleted, or renamed
    PageChanged,
    /// Database transaction was committed
    DbTransaction,
    /// Search index was updated
    SearchIndexUpdated,
}

impl HookEventKind {
    /// Returns the name of this event kind for debugging/logging.
    pub fn name(&self) -> &'static str {
        match self {
            HookEventKind::BlockChanged => "block_changed",
            HookEventKind::PageChanged => "page_changed",
            HookEventKind::DbTransaction => "db_transaction",
            HookEventKind::SearchIndexUpdated => "search_index_updated",
        }
    }
}

/// All hook events that can be dispatched to plugins.
///
/// Each variant carries its associated payload with the event data.
/// Events are cloned when dispatched to multiple plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookEvent {
    /// Block was created, updated, or deleted
    BlockChanged(BlockPayload),
    /// Page was created, updated, deleted, or renamed
    PageChanged(PagePayload),
    /// Database transaction was committed
    DbTransaction(TransactionPayload),
    /// Search index was updated
    SearchIndexUpdated(SearchIndexPayload),
}

impl HookEvent {
    /// Returns the [`HookEventKind`] for this event.
    pub fn kind(&self) -> HookEventKind {
        match self {
            HookEvent::BlockChanged(_) => HookEventKind::BlockChanged,
            HookEvent::PageChanged(_) => HookEventKind::PageChanged,
            HookEvent::DbTransaction(_) => HookEventKind::DbTransaction,
            HookEvent::SearchIndexUpdated(_) => HookEventKind::SearchIndexUpdated,
        }
    }
}

/// Payload for block change events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPayload {
    /// Unique identifier of the block
    pub id: String,
    /// ID of the page containing the block
    pub page_id: String,
    /// Type of change that occurred
    pub change_type: ChangeType,
    /// Block content (None if deleted)
    pub content: Option<String>,
}

/// Payload for page change events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePayload {
    /// Unique identifier of the page
    pub id: String,
    /// Page name (title)
    pub name: String,
    /// Type of change that occurred
    pub change_type: ChangeType,
}

/// Payload for transaction events.
///
/// Emitted after a database transaction is committed, containing
/// all block and page mutations that were part of the transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionPayload {
    /// Unique transaction identifier
    pub tx_id: String,
    /// All block changes in this transaction
    pub block_changes: Vec<BlockMutation>,
    /// All page changes in this transaction
    pub page_changes: Vec<PageMutation>,
    /// Transaction commit timestamp (ISO 8601)
    pub committed_at: String,
}

/// A single block mutation within a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockMutation {
    /// Block identifier
    pub id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Content if created/updated
    pub content: Option<String>,
}

/// A single page mutation within a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMutation {
    /// Page identifier
    pub id: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Page name if created/renamed
    pub name: Option<String>,
}

/// Payload for search index update events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexPayload {
    /// Type of index update
    pub update_type: SearchIndexUpdateType,
    /// Number of blocks affected (if applicable)
    pub blocks_affected: Option<u32>,
    /// Number of pages affected (if applicable)
    pub pages_affected: Option<u32>,
}

/// Type of search index update.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchIndexUpdateType {
    /// Full index rebuild
    FullRebuild,
    /// Incremental block index update
    BlocksUpdated,
    /// Incremental page index update
    PagesUpdated,
}

/// Type of change that occurred to a block or page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Entity was created
    Created,
    /// Entity was updated
    Updated,
    /// Entity was deleted
    Deleted,
}

impl ChangeType {
    /// Returns the name of this change type for debugging/logging.
    pub fn name(&self) -> &'static str {
        match self {
            ChangeType::Created => "created",
            ChangeType::Updated => "updated",
            ChangeType::Deleted => "deleted",
        }
    }
}

/// Result of a single plugin's hook execution.
///
/// Contains the plugin name and whether execution succeeded or failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Name of the plugin that handled the hook
    pub plugin_name: String,
    /// Whether the hook handler succeeded
    pub success: bool,
    /// Error message if execution failed (None if success)
    pub error: Option<String>,
}

impl HookResult {
    /// Creates a successful hook result.
    pub fn success(plugin_name: impl Into<String>) -> Self {
        Self {
            plugin_name: plugin_name.into(),
            success: true,
            error: None,
        }
    }

    /// Creates a failed hook result.
    pub fn failure(plugin_name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            plugin_name: plugin_name.into(),
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Hook payload trait for extracting event-specific data.
///
/// This trait is implemented by each payload type to allow generic
/// handling of hook data across different event types.
pub trait HookPayload: Debug + Clone + Serialize + for<'de> Deserialize<'de> {
    /// Returns the event kind this payload belongs to.
    fn event_kind() -> HookEventKind;
}

impl HookPayload for BlockPayload {
    fn event_kind() -> HookEventKind {
        HookEventKind::BlockChanged
    }
}

impl HookPayload for PagePayload {
    fn event_kind() -> HookEventKind {
        HookEventKind::PageChanged
    }
}

impl HookPayload for TransactionPayload {
    fn event_kind() -> HookEventKind {
        HookEventKind::DbTransaction
    }
}

impl HookPayload for SearchIndexPayload {
    fn event_kind() -> HookEventKind {
        HookEventKind::SearchIndexUpdated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_kind_name() {
        assert_eq!(HookEventKind::BlockChanged.name(), "block_changed");
        assert_eq!(HookEventKind::PageChanged.name(), "page_changed");
        assert_eq!(HookEventKind::DbTransaction.name(), "db_transaction");
        assert_eq!(
            HookEventKind::SearchIndexUpdated.name(),
            "search_index_updated"
        );
    }

    #[test]
    fn test_hook_event_kind_from_event() {
        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Hello".to_string()),
        });
        assert_eq!(event.kind(), HookEventKind::BlockChanged);
    }

    #[test]
    fn test_change_type_name() {
        assert_eq!(ChangeType::Created.name(), "created");
        assert_eq!(ChangeType::Updated.name(), "updated");
        assert_eq!(ChangeType::Deleted.name(), "deleted");
    }

    #[test]
    fn test_hook_result_success() {
        let result = HookResult::success("my_plugin");
        assert!(result.success);
        assert_eq!(result.plugin_name, "my_plugin");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_hook_result_failure() {
        let result = HookResult::failure("my_plugin", "Something went wrong");
        assert!(!result.success);
        assert_eq!(result.plugin_name, "my_plugin");
        assert_eq!(result.error.as_ref().unwrap(), "Something went wrong");
    }

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::BlockChanged(BlockPayload {
            id: "block-1".to_string(),
            page_id: "page-1".to_string(),
            change_type: ChangeType::Created,
            content: Some("Hello".to_string()),
        });

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"block_changed\""));

        let deserialized: HookEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, HookEvent::BlockChanged(_)));
    }

    #[test]
    fn test_transaction_payload_serialization() {
        let payload = TransactionPayload {
            tx_id: "tx-1".to_string(),
            block_changes: vec![BlockMutation {
                id: "block-1".to_string(),
                change_type: ChangeType::Created,
                content: Some("New block".to_string()),
            }],
            page_changes: vec![],
            committed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        let deserialized: TransactionPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tx_id, "tx-1");
        assert_eq!(deserialized.block_changes.len(), 1);
    }
}
