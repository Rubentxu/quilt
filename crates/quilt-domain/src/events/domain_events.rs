//! DomainEvents - events emitted by domain entities

use crate::value_objects::Uuid;
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Type of file system event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEventType {
    /// A new file was created
    Created,
    /// An existing file was modified
    Modified,
    /// A file was deleted
    Deleted,
}

/// FileChanged event emitted when a watched file is created, modified, or deleted.
#[derive(Debug, Clone)]
pub struct FileChanged {
    /// Path to the changed file
    pub path: PathBuf,
    /// Type of change that occurred
    pub event_type: FileEventType,
    /// When the change was detected
    pub timestamp: DateTime<Utc>,
}

/// Application-level events broadcast via the event system.
///
/// This enum contains all domain events that can be published to the event bus.
/// Each variant carries the specific event data needed by subscribers.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// A watched file was changed
    FileChanged(FileChanged),
    /// A new block was created
    BlockCreated(BlockCreated),
    /// A block was updated
    BlockUpdated(BlockUpdated),
    /// A block was deleted
    BlockDeleted(BlockDeleted),
    /// A block was moved to a new parent or position
    BlockMoved(BlockMoved),
    /// A new page was created
    PageCreated(PageCreated),
    /// A page was renamed
    PageRenamed(PageRenamed),
    /// A page was deleted
    PageDeleted(PageDeleted),
}

/// DomainEvent is the base trait for all domain events.
pub trait DomainEvent: Send + Sync {
    /// Get the event name
    fn event_name(&self) -> &'static str;
    /// Get the entity ID that triggered this event
    fn entity_id(&self) -> Uuid;
}

/// BlockCreated is emitted when a new block is created.
#[derive(Debug, Clone)]
pub struct BlockCreated {
    pub block_id: Uuid,
    pub page_id: Uuid,
    pub parent_id: Option<Uuid>,
}

/// BlockUpdated is emitted when a block is modified.
#[derive(Debug, Clone)]
pub struct BlockUpdated {
    pub block_id: Uuid,
    pub changed_fields: Vec<&'static str>,
}

/// BlockDeleted is emitted when a block is deleted.
#[derive(Debug, Clone)]
pub struct BlockDeleted {
    pub block_id: Uuid,
    pub page_id: Uuid,
}

/// BlockMoved is emitted when a block changes parent or order.
#[derive(Debug, Clone)]
pub struct BlockMoved {
    pub block_id: Uuid,
    pub old_parent_id: Option<Uuid>,
    pub new_parent_id: Option<Uuid>,
    pub old_order: f64,
    pub new_order: f64,
}

/// PageCreated is emitted when a new page is created.
#[derive(Debug, Clone)]
pub struct PageCreated {
    pub page_id: Uuid,
    pub name: String,
    pub is_journal: bool,
}

/// PageRenamed is emitted when a page is renamed.
#[derive(Debug, Clone)]
pub struct PageRenamed {
    pub page_id: Uuid,
    pub old_name: String,
    pub new_name: String,
}

/// PageDeleted is emitted when a page is deleted.
#[derive(Debug, Clone)]
pub struct PageDeleted {
    pub page_id: Uuid,
    pub name: String,
}

// Implement DomainEvent for all event types

impl DomainEvent for BlockCreated {
    fn event_name(&self) -> &'static str {
        "block.created"
    }
    fn entity_id(&self) -> Uuid {
        self.block_id
    }
}

impl DomainEvent for BlockUpdated {
    fn event_name(&self) -> &'static str {
        "block.updated"
    }
    fn entity_id(&self) -> Uuid {
        self.block_id
    }
}

impl DomainEvent for BlockDeleted {
    fn event_name(&self) -> &'static str {
        "block.deleted"
    }
    fn entity_id(&self) -> Uuid {
        self.block_id
    }
}

impl DomainEvent for BlockMoved {
    fn event_name(&self) -> &'static str {
        "block.moved"
    }
    fn entity_id(&self) -> Uuid {
        self.block_id
    }
}

impl DomainEvent for PageCreated {
    fn event_name(&self) -> &'static str {
        "page.created"
    }
    fn entity_id(&self) -> Uuid {
        self.page_id
    }
}

impl DomainEvent for PageRenamed {
    fn event_name(&self) -> &'static str {
        "page.renamed"
    }
    fn entity_id(&self) -> Uuid {
        self.page_id
    }
}

impl DomainEvent for PageDeleted {
    fn event_name(&self) -> &'static str {
        "page.deleted"
    }
    fn entity_id(&self) -> Uuid {
        self.page_id
    }
}
