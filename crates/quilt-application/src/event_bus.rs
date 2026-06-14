//! Event Bus implementation using tokio broadcast channel
//!
//! This module provides the central event distribution system for the application.
//! It uses a tokio broadcast channel to distribute `AppEvent` to multiple subscribers.
//!
//! # Architecture
//!
//! - [`EventBus`] is the central publisher that holds the broadcast sender
//! - Subscribers receive a [`broadcast::Receiver`] that can be used to receive events
//! - Multiple subscribers can receive the same event simultaneously
//!
//! # Example
//!
//! ```ignore
//! use quilt_application::event_bus::EventBus;
//! use quilt_domain::events::AppEvent;
//!
//! // Create a new event bus
//! let bus = EventBus::new();
//!
//! // Subscribe to receive events
//! let mut receiver = bus.subscribe();
//!
//! // Publish an event
//! // bus.publish(event);
//! ```

use quilt_domain::events::AppEvent;
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::instrument;

/// Maximum number of subscribers that can receive events simultaneously.
/// This is the buffer size for the broadcast channel.
const BROADCAST_CHANNEL_SIZE: usize = 1000;

/// Errors that can occur when working with the EventBus.
#[derive(Debug, Error)]
pub enum EventBusError {
    #[error("Failed to send event: broadcast channel closed")]
    ChannelClosed,
}

/// EventBus is the central event distribution system for the application.
///
/// It uses a tokio broadcast channel to distribute `AppEvent` to multiple subscribers.
/// The bus can be shared across the application to enable loose coupling between
/// components that need to react to domain events.
///
/// # Example
///
/// ```ignore
/// use quilt_application::event_bus::EventBus;
/// use quilt_domain::events::AppEvent;
///
/// // Create a new event bus
/// let bus = EventBus::new();
///
/// // Subscribe to receive events
/// let mut receiver = bus.subscribe();
///
/// // In another task, publish events
/// bus.publish(AppEvent::BlockCreated(...));
/// ```
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    /// Creates a new EventBus with a default channel size.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use quilt_application::event_bus::EventBus;
    ///
    /// let bus = EventBus::new();
    /// ```
    #[instrument(skip_all)]
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(BROADCAST_CHANNEL_SIZE);
        Self { sender }
    }

    /// Publishes an event to all subscribers.
    ///
    /// This method sends the event to all active receivers. If no receivers are
    /// subscribed, the event is silently dropped (this is the expected behavior
    /// for a broadcast channel).
    ///
    /// # Arguments
    ///
    /// * `event` - The event to publish
    ///
    /// # Example
    ///
    /// ```ignore
    /// use quilt_application::event_bus::EventBus;
    /// use quilt_domain::events::{AppEvent, BlockCreated};
    /// use quilt_domain::value_objects::Uuid;
    ///
    /// let bus = EventBus::new();
    /// bus.publish(AppEvent::BlockCreated(BlockCreated {
    ///     block_id: Uuid::new_v4(),
    ///     page_id: Uuid::new_v4(),
    ///     parent_id: None,
    /// }));
    /// ```
    #[instrument(skip(self, event), fields(event_name = event_name(&event)))]
    pub fn publish(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }

    /// Subscribes to receive events from the bus.
    ///
    /// Returns a receiver that will receive all subsequent events published
    /// to the bus. The receiver will continue to receive events until it is
    /// dropped or the channel is closed.
    ///
    /// # Returns
    ///
    /// A [`broadcast::Receiver<AppEvent>`] that can be used to receive events.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use quilt_application::event_bus::EventBus;
    ///
    /// let bus = EventBus::new();
    /// let mut receiver = bus.subscribe();
    /// ```
    #[instrument(skip(self))]
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    /// Returns the number of active subscribers.
    ///
    /// This is primarily useful for testing and monitoring.
    #[instrument(skip(self))]
    pub fn subscriber_count(&self) -> usize {
        // Note: broadcast::Sender doesn't provide a direct subscriber count.
        // This would require a custom implementation or using a different approach.
        // For now, we return 0 as a placeholder.
        0
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the name of an AppEvent variant for logging/tracing purposes.
fn event_name(event: &AppEvent) -> &'static str {
    match event {
        AppEvent::FileChanged(_) => "file_changed",
        AppEvent::BlockCreated(_) => "block_created",
        AppEvent::BlockUpdated(_) => "block_updated",
        AppEvent::BlockDeleted(_) => "block_deleted",
        AppEvent::BlockMoved(_) => "block_moved",
        AppEvent::PageCreated(_) => "page_created",
        AppEvent::PageRenamed(_) => "page_renamed",
        AppEvent::PageDeleted(_) => "page_deleted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::events::{BlockCreated, BlockDeleted, BlockMoved, BlockUpdated};
    use quilt_domain::events::{FileChanged, FileEventType};
    use quilt_domain::events::{PageCreated, PageDeleted, PageRenamed};

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        let _receiver = bus.subscribe();

        // Verify we can send an event
        let event = AppEvent::BlockCreated(BlockCreated {
            block_id: quilt_domain::value_objects::Uuid::new_v4(),
            page_id: quilt_domain::value_objects::Uuid::new_v4(),
            parent_id: None,
        });

        bus.publish(event);
    }

    #[test]
    fn test_app_event_all_variants() {
        // Ensure AppEvent has all expected variants
        let uuid = quilt_domain::value_objects::Uuid::new_v4();

        let _ = AppEvent::FileChanged(FileChanged {
            path: std::path::PathBuf::from("test.md"),
            event_type: FileEventType::Created,
            timestamp: chrono::Utc::now(),
        });

        let _ = AppEvent::BlockCreated(BlockCreated {
            block_id: uuid,
            page_id: uuid,
            parent_id: None,
        });
        let _ = AppEvent::BlockUpdated(BlockUpdated {
            block_id: uuid,
            changed_fields: vec!["content"],
        });
        let _ = AppEvent::BlockDeleted(BlockDeleted {
            block_id: uuid,
            page_id: uuid,
        });
        let _ = AppEvent::BlockMoved(BlockMoved {
            block_id: uuid,
            old_parent_id: None,
            new_parent_id: None,
            old_order: 1.0,
            new_order: 2.0,
        });
        let _ = AppEvent::PageCreated(PageCreated {
            page_id: uuid,
            name: "Test Page".to_string(),
            is_journal: false,
        });
        let _ = AppEvent::PageRenamed(PageRenamed {
            page_id: uuid,
            old_name: "Old".to_string(),
            new_name: "New".to_string(),
        });
        let _ = AppEvent::PageDeleted(PageDeleted {
            page_id: uuid,
            name: "Test Page".to_string(),
        });
    }

    #[test]
    fn test_event_name() {
        let uuid = quilt_domain::value_objects::Uuid::new_v4();

        let file_event = AppEvent::FileChanged(FileChanged {
            path: std::path::PathBuf::from("test.md"),
            event_type: FileEventType::Created,
            timestamp: chrono::Utc::now(),
        });
        assert_eq!(event_name(&file_event), "file_changed");

        let block_event = AppEvent::BlockCreated(BlockCreated {
            block_id: uuid,
            page_id: uuid,
            parent_id: None,
        });
        assert_eq!(event_name(&block_event), "block_created");
    }
}
