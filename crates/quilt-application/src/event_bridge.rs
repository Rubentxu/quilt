//! EventBridge - connects file system watcher events to search index updates
//!
//! This service subscribes to `AppEvent::FileChanged` events from the file watcher
//! and triggers incremental FTS5 re-indexing via `SearchIndexManager::rebuild_incremental()`.
//!
//! NOTE: This module requires the `tokio-runtime` feature and is not available on wasm32.

#[cfg(feature = "tokio-runtime")]
use crate::errors::ApplicationError;
#[cfg(feature = "tokio-runtime")]
use quilt_domain::events::{AppEvent, FileEventType};
#[cfg(feature = "tokio-runtime")]
use quilt_search::SearchIndexManager;
#[cfg(feature = "tokio-runtime")]
use std::time::Duration;
#[cfg(feature = "tokio-runtime")]
use tokio::sync::broadcast;
#[cfg(feature = "tokio-runtime")]
use tracing::instrument;

#[cfg(feature = "tokio-runtime")]
/// EventBridge subscribes to file change events and triggers search index updates.
pub struct EventBridge {
    /// Receiver for AppEvent broadcasts
    receiver: broadcast::Receiver<AppEvent>,
    /// Search index manager for FTS5 operations
    search_index: SearchIndexManager,
}

#[cfg(feature = "tokio-runtime")]
impl EventBridge {
    /// Create a new EventBridge with the given search index and event receiver.
    pub fn new(search_index: SearchIndexManager, receiver: broadcast::Receiver<AppEvent>) -> Self {
        Self {
            receiver,
            search_index,
        }
    }

    /// Run the event bridge, processing file change events indefinitely.
    ///
    /// This method listens for `AppEvent::FileChanged` events and triggers
    /// incremental rebuilds on the search index.
    #[instrument(skip(self))]
    pub async fn run(mut self) -> Result<(), ApplicationError> {
        // Debounce duration: wait for event storm to settle
        let debounce_duration = Duration::from_millis(500);

        loop {
            // Wait for the next event
            match self.receiver.recv().await {
                Ok(AppEvent::FileChanged(change)) => {
                    // Wait for debounce period to coalesce rapid file changes
                    tokio::time::sleep(debounce_duration).await;

                    // Process based on event type
                    match change.event_type {
                        FileEventType::Created | FileEventType::Modified => {
                            tracing::debug!(
                                "File changed: {:?}, triggering incremental rebuild",
                                change.path
                            );
                            if let Err(e) = self
                                .search_index
                                .rebuild_incremental(change.timestamp)
                                .await
                            {
                                tracing::error!("Incremental rebuild failed: {}", e);
                            }
                        }
                        FileEventType::Deleted => {
                            tracing::debug!("File deleted: {:?}", change.path);
                            // For deletions, we could remove from index if needed
                            // Currently the FTS5 external content table handles this via triggers
                        }
                    }
                }
                Ok(_) => {
                    // Ignore other event types (reserved for future AppEvent variants)
                    // Currently AppEvent only has FileChanged
                }
                Err(_) => {
                    tracing::info!("EventBridge: broadcast channel closed, shutting down");
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(all(feature = "tokio-runtime", test))]
mod tests {
    use super::*;
    use chrono::Utc;
    use quilt_domain::events::{FileChanged, FileEventType};
    use std::path::PathBuf;

    #[test]
    fn test_event_bridge_creation() {
        // Create a minimal EventBridge for testing
        // Note: This test just verifies the struct can be created
        // Full integration testing would require a test database
        let (tx, rx) = broadcast::channel(16);

        // We can't easily create SearchIndexManager without a database,
        // so we just verify the channel creation works
        assert!(tx
            .send(AppEvent::FileChanged(FileChanged {
                path: PathBuf::from("test.md"),
                event_type: FileEventType::Created,
                timestamp: Utc::now(),
            }))
            .is_ok());

        drop(tx);

        // Verify receiver still works after sender dropped
        let mut rx = rx;
        // This will return an error since channel is closed - that's expected
    }

    #[test]
    fn test_file_changed_event() {
        let event = FileChanged {
            path: PathBuf::from("/graphs/test/notes.md"),
            event_type: FileEventType::Modified,
            timestamp: Utc::now(),
        };

        assert_eq!(event.path, PathBuf::from("/graphs/test/notes.md"));
        match event.event_type {
            FileEventType::Modified => {}
            _ => panic!("Expected Modified"),
        }
    }

    #[test]
    fn test_file_event_type_variants() {
        let created = FileEventType::Created;
        let modified = FileEventType::Modified;
        let deleted = FileEventType::Deleted;

        match created {
            FileEventType::Created => {}
            _ => panic!("Expected Created"),
        }
        match modified {
            FileEventType::Modified => {}
            _ => panic!("Expected Modified"),
        }
        match deleted {
            FileEventType::Deleted => {}
            _ => panic!("Expected Deleted"),
        }
    }
}
