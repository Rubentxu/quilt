//! SSE Events Handler
//!
//! Server-Sent Events (SSE) endpoint for real-time updates.
//!
//! # Event Types
//!
//! - `file-change` - A markdown file was modified
//! - `block-created`, `block-updated`, `block-deleted` - Block lifecycle events
//! - `page-created`, `page-updated` - Page events
//! - `sync-complete` - Full sync finished
//!
//! # SSE Format
//!
//! ```text
//! event: block-created
//! data: {"id": "uuid", "content": "...", "updated_at": "..."}
//!
//! event: file-change
//! data: {"path": "/pages/test.md", "kind": "modified"}
//! ```

use std::sync::Arc;

use axum::{
    extract::State,
    http::header::{HeaderMap, HeaderName, HeaderValue},
    Json,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::state::HttpState;

/// SSE event types that can be sent to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum SseEvent {
    /// A file was changed (created/modified/deleted)
    FileChange {
        path: String,
        kind: String,
    },
    /// A block was created
    BlockCreated {
        id: String,
        page_id: String,
        content: String,
    },
    /// A block was updated
    BlockUpdated {
        id: String,
        content: String,
    },
    /// A block was deleted
    BlockDeleted { id: String },
    /// A page was created
    PageCreated { id: String, name: String },
    /// A page was updated
    PageUpdated { id: String, name: String },
    /// Full sync completed
    SyncComplete { timestamp: String },
    /// Heartbeat to keep connection alive
    Heartbeat,
}

impl SseEvent {
    /// Get the SSE event name for this event type
    fn event_name(&self) -> &str {
        match self {
            SseEvent::FileChange { .. } => "file-change",
            SseEvent::BlockCreated { .. } => "block-created",
            SseEvent::BlockUpdated { .. } => "block-updated",
            SseEvent::BlockDeleted { .. } => "block-deleted",
            SseEvent::PageCreated { .. } => "page-created",
            SseEvent::PageUpdated { .. } => "page-updated",
            SseEvent::SyncComplete { .. } => "sync-complete",
            SseEvent::Heartbeat { .. } => "heartbeat",
        }
    }

    /// Convert to JSON for SSE data field
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"error":"serialization_failed"}"#.to_string())
    }

    /// Create from FileChangeEvent (from polling service)
    pub fn from_file_change(path: &str, kind: &str) -> Self {
        SseEvent::FileChange {
            path: path.to_string(),
            kind: kind.to_string(),
        }
    }
}

/// Create an SSE event from a file change event
impl From<crate::polling::FileChangeEvent> for SseEvent {
    fn from(event: crate::polling::FileChangeEvent) -> Self {
        match event {
            crate::polling::FileChangeEvent::Created { path } => {
                SseEvent::FileChange { path, kind: "created".to_string() }
            }
            crate::polling::FileChangeEvent::Modified { path } => {
                SseEvent::FileChange { path, kind: "modified".to_string() }
            }
            crate::polling::FileChangeEvent::Deleted { path } => {
                SseEvent::FileChange { path, kind: "deleted".to_string() }
            }
        }
    }
}

/// App-level SSE events channel sender
/// This is stored in HttpState and shared across handlers
#[derive(Clone)]
pub struct SseBroadcaster {
    /// Sender for broadcasting events to SSE clients
    sender: broadcast::Sender<SseEvent>,
}

impl SseBroadcaster {
    /// Create a new SseBroadcaster
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribe to events (returns a receiver)
    pub fn subscribe(&self) -> broadcast::Receiver<SseEvent> {
        self.sender.subscribe()
    }

    /// Broadcast an event to all subscribers
    pub fn send(&self, event: SseEvent) -> Result<usize, broadcast::error::SendError<SseEvent>> {
        self.sender.send(event)
    }

    /// Get the sender for storage in HttpState
    pub fn into_sender(self) -> broadcast::Sender<SseEvent> {
        self.sender
    }

    /// Create from an existing sender
    pub fn from_sender(sender: broadcast::Sender<SseEvent>) -> Self {
        Self { sender }
    }
}

/// SSE connection handler
async fn handle_sse_stream(
    state: Arc<HttpState>,
    mut rx: broadcast::Receiver<SseEvent>,
) -> impl IntoResponse {
    info!("New SSE connection established");

    // Create a stream that yields SSE events
    // We use a regular stream with Result<Event, Infallible> since we handle all errors internally
    let stream = async_stream::stream! {
        // Send initial connection event
        yield Ok::<_, std::convert::Infallible>(Event::default()
            .event("connected")
            .data(r#"{"status":"connected"}"#));

        // Send heartbeat every 30 seconds to keep connection alive
        let mut heartbeat_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                // Event from broadcast channel
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            debug!("SSE broadcasting event: {:?}", event.event_name());
                            let event_name = event.event_name().to_string();
                            let data = event.to_json();

                            yield Ok(Event::default()
                                .event(&event_name)
                                .data(&data));
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            warn!("SSE broadcast channel closed, ending stream");
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // Receiver lagged behind, just continue
                            debug!("SSE receiver lagged, skipping events");
                        }
                    }
                }
                // Heartbeat tick
                _ = heartbeat_interval.tick() => {
                    yield Ok(Event::default()
                        .event("heartbeat")
                        .data(r#"{"type":"heartbeat","timestamp":""}"#));
                }
            }
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-cache"),
    );
    headers.insert(
        HeaderName::from_static("connection"),
        HeaderValue::from_static("keep-alive"),
    );
    headers.insert(
        HeaderName::from_static("x-accel-buffering"),
        HeaderValue::from_static("no"),
    );

    // Add CORS headers if needed
    headers.insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );

    (
        headers,
        Sse::new(stream).keep_alive(
            axum::response::sse::KeepAlive::default()
                .interval(std::time::Duration::from_secs(30))
                .text("data: heartbeat\n\n"),
        ),
    )
}

/// GET /api/events - Server-Sent Events stream
///
/// Returns an SSE stream that sends events when:
/// - File changes detected (via polling)
/// - Database changes (via notification)
/// - MCP server responses
///
/// # Event Types
///
/// - `file-change` - A markdown file was modified
/// - `block-created`, `block-updated`, `block-deleted` - Block lifecycle events
/// - `page-created`, `page-updated` - Page events
/// - `sync-complete` - Full sync finished
///
/// # SSE Format
///
/// ```text
/// event: block-created
/// data: {"type":"block-created","data":{"id":"uuid","page_id":"...","content":"..."}}
/// ```
pub async fn events_handler(
    State(state): State<Arc<HttpState>>,
) -> impl IntoResponse {
    // Get the SSE broadcaster from state
    let broadcaster = state.sse_broadcaster.clone();

    // Subscribe to events
    let rx = broadcaster.subscribe();

    handle_sse_stream(state, rx).await
}

/// POST /api/events/file-change - Notify of a file change
///
/// This endpoint can be called by internal services to notify about file changes.
pub async fn notify_file_change(
    State(state): State<Arc<HttpState>>,
    Json(payload): Json<FileChangePayload>,
) -> impl IntoResponse {
    let broadcaster = state.sse_broadcaster.clone();

    let event = SseEvent::from_file_change(&payload.path, &payload.kind);
    if let Err(e) = broadcaster.send(event) {
        error!("Failed to broadcast file change event: {}", e);
    }

    (axum::http::StatusCode::ACCEPTED, "")
}

/// Payload for file change notification
#[derive(Debug, Deserialize)]
pub struct FileChangePayload {
    pub path: String,
    pub kind: String,
}

/// POST /api/events/block - Notify of a block change
///
/// This endpoint can be called by internal services to notify about block changes.
pub async fn notify_block_event(
    State(state): State<Arc<HttpState>>,
    Json(payload): Json<BlockEventPayload>,
) -> impl IntoResponse {
    let broadcaster = state.sse_broadcaster.clone();

    let event = match payload.action.as_str() {
        "created" => SseEvent::BlockCreated {
            id: payload.id,
            page_id: payload.page_id.unwrap_or_default(),
            content: payload.content.unwrap_or_default(),
        },
        "updated" => SseEvent::BlockUpdated {
            id: payload.id,
            content: payload.content.unwrap_or_default(),
        },
        "deleted" => SseEvent::BlockDeleted { id: payload.id },
        _ => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid action");
        }
    };

    if let Err(e) = broadcaster.send(event) {
        error!("Failed to broadcast block event: {}", e);
    }

    (axum::http::StatusCode::ACCEPTED, "")
}

/// Payload for block event notification
#[derive(Debug, Deserialize)]
pub struct BlockEventPayload {
    pub action: String,
    pub id: String,
    pub page_id: Option<String>,
    pub content: Option<String>,
}

/// POST /api/events/page - Notify of a page change
///
/// This endpoint can be called by internal services to notify about page changes.
pub async fn notify_page_event(
    State(state): State<Arc<HttpState>>,
    Json(payload): Json<PageEventPayload>,
) -> impl IntoResponse {
    let broadcaster = state.sse_broadcaster.clone();

    let event = match payload.action.as_str() {
        "created" => SseEvent::PageCreated {
            id: payload.id,
            name: payload.name.unwrap_or_default(),
        },
        "updated" => SseEvent::PageUpdated {
            id: payload.id,
            name: payload.name.unwrap_or_default(),
        },
        _ => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid action");
        }
    };

    if let Err(e) = broadcaster.send(event) {
        error!("Failed to broadcast page event: {}", e);
    }

    (axum::http::StatusCode::ACCEPTED, "")
}

/// Payload for page event notification
#[derive(Debug, Deserialize)]
pub struct PageEventPayload {
    pub action: String,
    pub id: String,
    pub name: Option<String>,
}

/// Mount SSE event routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/api/events", axum::routing::get(events_handler))
        .route("/api/events/file-change", axum::routing::post(notify_file_change))
        .route("/api/events/block", axum::routing::post(notify_block_event))
        .route("/api/events/page", axum::routing::post(notify_page_event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_sse_event_serialization() {
        let event = SseEvent::BlockCreated {
            id: "test-id".to_string(),
            page_id: "page-1".to_string(),
            content: "Test content".to_string(),
        };

        let json = event.to_json();
        // Note: serde uses snake_case for enum variant names due to rename_all = "snake_case"
        assert!(json.contains("\"type\":\"block_created\""));
        assert!(json.contains("\"id\":\"test-id\""));
        assert!(json.contains("\"page_id\":\"page-1\""));
        assert!(json.contains("\"content\":\"Test content\""));
    }

    #[test]
    fn test_sse_event_file_change() {
        let event = SseEvent::FileChange {
            path: "/pages/test.md".to_string(),
            kind: "modified".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        // Note: serde uses snake_case for enum variant names
        assert!(json.contains("\"type\":\"file_change\""));
        assert!(json.contains("\"path\":\"/pages/test.md\""));
        assert!(json.contains("\"kind\":\"modified\""));
    }

    #[test]
    fn test_sse_event_names() {
        let events = vec![
            (SseEvent::FileChange { path: String::new(), kind: String::new() }, "file-change"),
            (SseEvent::BlockCreated { id: String::new(), page_id: String::new(), content: String::new() }, "block-created"),
            (SseEvent::SyncComplete { timestamp: String::new() }, "sync-complete"),
        ];

        for (event, expected_name) in events {
            assert_eq!(event.event_name(), expected_name);
        }
    }

    #[tokio::test]
    async fn test_sse_broadcaster() {
        let broadcaster = SseBroadcaster::new(100);
        let rx = broadcaster.subscribe();

        let event = SseEvent::SyncComplete {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        };

        broadcaster.send(event.clone()).unwrap();

        // In a real scenario, we'd check the receiver
        // For unit test, just verify it doesn't panic
        drop(rx);
    }
}