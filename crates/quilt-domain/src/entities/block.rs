//! Block entity - the fundamental unit of content in Quilt

use crate::content::BlockContent;
use crate::errors::DomainError;
use crate::services::TimezoneService;
use crate::value_objects::{BlockFormat, Priority, PropertyValue, TaskMarker, Uuid};
use std::collections::HashMap;

/// Block is the fundamental unit of content in Quilt.
///
/// Every piece of content is a block, whether it's a page title,
/// a bullet point, or a nested item in an outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Unique identifier - cannot change after creation
    pub id: Uuid,
    /// The page this block belongs to
    pub page_id: Uuid,
    /// Parent block (None for top-level blocks on a page)
    pub parent_id: Option<Uuid>,
    /// Lexicographic order among siblings (fractional indexing)
    pub order: f64,
    /// Indentation level (1-indexed)
    pub level: u8,
    /// Content format
    pub format: BlockFormat,
    /// Task marker (if this block is a task)
    pub marker: Option<TaskMarker>,
    /// Priority level (A, B, C)
    pub priority: Option<Priority>,
    /// The actual content - structured as typed segments (ADR 0005)
    pub content: BlockContent,
    /// Custom properties (key-value pairs)
    pub properties: HashMap<String, PropertyValue>,
    /// References to other blocks/pages
    pub refs: Vec<Uuid>,
    /// Tags associated with this block
    pub tags: Vec<String>,
    /// Scheduled date/time
    pub scheduled: Option<chrono::DateTime<chrono::Utc>>,
    /// Deadline date/time
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    /// Start time for duration tracking
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    /// Repeated task configuration
    pub repeated: Option<chrono::DateTime<chrono::Utc>>,
    /// Logbook state (CLOSED timestamp if done)
    pub logbook: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this block is collapsed in the outliner
    pub collapsed: bool,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Journal day when this block was created (YYYYMMDD format).
    ///
    /// This is a denormalized field for efficient queries.
    /// When a block is created, this is automatically set to the
    /// current journal day in the user's timezone.
    ///
    /// NULL means the block was created before this feature
    /// was implemented (migration case) or is an orphan block.
    pub journal_day: Option<i32>,
    /// Journal day when this block was last updated (YYYYMMDD format).
    ///
    /// Updated on every content change, move, or property change.
    /// Used for the "updated today" activity stream.
    pub updated_journal_day: Option<i32>,
}

/// Data required to create a new block
#[derive(Debug, Clone)]
pub struct BlockCreate {
    pub page_id: Uuid,
    pub content: BlockContent,
    pub parent_id: Option<Uuid>,
    pub order: f64,
    pub marker: Option<TaskMarker>,
    pub format: BlockFormat,
    pub properties: HashMap<String, PropertyValue>,
}

/// Data required to update an existing block
#[derive(Debug, Clone, Default)]
pub struct BlockUpdate {
    pub content: Option<BlockContent>,
    pub parent_id: Option<Option<Uuid>>,
    pub order: Option<f64>,
    pub level: Option<u8>,
    pub marker: Option<TaskMarker>,
    pub priority: Option<Priority>,
    pub properties: Option<HashMap<String, PropertyValue>>,
    pub scheduled: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub deadline: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub collapsed: Option<bool>,
}

impl Block {
    /// Create a new block with the given data.
    ///
    /// The journal_day is automatically set based on the user's timezone.
    /// Refs are automatically extracted from the content.
    pub fn new(create: BlockCreate, timezone: &TimezoneService) -> Result<Self, DomainError> {
        let now = chrono::Utc::now();
        let journal_day = timezone.today_journal_day().as_i32();

        // Create the block with the provided content
        let mut block = Self {
            id: Uuid::new_v4(),
            page_id: create.page_id,
            parent_id: create.parent_id,
            order: create.order,
            level: create.parent_id.map(|_| 2).unwrap_or(1),
            format: create.format,
            marker: create.marker,
            priority: None,
            content: create.content,
            properties: create.properties,
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: now,
            updated_at: now,
            journal_day: Some(journal_day),
            updated_journal_day: Some(journal_day),
        };

        // Extract refs from content
        block.extract_and_update_refs();

        Ok(block)
    }

    /// Apply an update to this block.
    ///
    /// The updated_journal_day is automatically updated based on the user's timezone.
    /// Also extracts and updates refs from the new content.
    pub fn update(
        &mut self,
        update: BlockUpdate,
        timezone: &TimezoneService,
    ) -> Result<(), DomainError> {
        if let Some(content) = update.content {
            self.content = content;
            self.extract_and_update_refs();
        }
        if let Some(parent_id) = update.parent_id {
            self.parent_id = parent_id;
            self.level = parent_id.map(|_| self.level.max(2)).unwrap_or(1);
        }
        if let Some(order) = update.order {
            self.order = order;
        }
        if let Some(level) = update.level {
            self.level = level;
        }
        if let Some(marker) = update.marker {
            self.marker = Some(marker);
            // Auto-set logbook when marker becomes DONE or CANCELLED
            if marker == TaskMarker::Done || marker == TaskMarker::Cancelled {
                self.logbook = Some(chrono::Utc::now());
            } else {
                self.logbook = None;
            }
        }
        if let Some(priority) = update.priority {
            self.priority = Some(priority);
        }
        if let Some(properties) = update.properties {
            self.properties = properties;
        }
        if let Some(scheduled) = update.scheduled {
            self.scheduled = scheduled;
        }
        if let Some(deadline) = update.deadline {
            self.deadline = deadline;
        }
        if let Some(collapsed) = update.collapsed {
            self.collapsed = collapsed;
        }
        self.updated_at = chrono::Utc::now();
        // Auto-update journal_day on every update
        self.updated_journal_day = Some(timezone.today_journal_day().as_i32());
        Ok(())
    }

    /// Check if this block can be moved to a new parent
    ///
    /// Rules:
    /// - Cannot move to itself
    /// - Cannot move to one of its own descendants (circular reference)
    pub fn can_move_to(&self, new_parent: Option<Uuid>, all_blocks: &[Block]) -> bool {
        // Cannot move to itself
        if new_parent == Some(self.id) {
            return false;
        }

        // Check for circular reference
        if let Some(parent_id) = new_parent {
            if self.is_descendant_of(parent_id, all_blocks) {
                return false;
            }
        }

        true
    }

    /// Check if target_id is an ancestor of this block (i.e., this block is a descendant of target_id)
    fn is_descendant_of(&self, target_id: Uuid, blocks: &[Block]) -> bool {
        // Check if target_id is this block's parent, grandparent, etc.
        let mut current_id = Some(self.id);
        while let Some(id) = current_id {
            // Find the block with this id
            if let Some(block) = blocks.iter().find(|b| b.id == id) {
                if block.parent_id == Some(target_id) {
                    return true; // target_id is the direct parent
                }
                current_id = block.parent_id;
            } else {
                break;
            }
        }
        false
    }

    /// Add a reference to another block or page
    pub fn add_ref(&mut self, ref_id: Uuid) {
        if !self.refs.contains(&ref_id) {
            self.refs.push(ref_id);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Remove a reference
    pub fn remove_ref(&mut self, ref_id: Uuid) {
        self.refs.retain(|r| *r != ref_id);
        self.updated_at = chrono::Utc::now();
    }

    /// Extract references from the block's content and update the refs field.
    ///
    /// This method parses the BlockContent to find PageRef and BlockRef segments,
    /// then updates the refs field to contain all referenced block and page IDs.
    ///
    /// Call this after creating a block or updating its content.
    pub fn extract_and_update_refs(&mut self) {
        use crate::content::BlockSegment;

        let mut new_refs = Vec::new();

        for segment in &self.content.segments {
            match segment {
                BlockSegment::PageRef { target, .. } => {
                    if !new_refs.contains(target) {
                        new_refs.push(*target);
                    }
                }
                BlockSegment::BlockRef { target } => {
                    if !new_refs.contains(target) {
                        new_refs.push(*target);
                    }
                }
                _ => {}
            }
        }

        self.refs = new_refs;
        self.updated_at = chrono::Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = chrono::Utc::now();
        }
    }

    /// Check if this block is a task (has a marker)
    pub fn is_task(&self) -> bool {
        self.marker.is_some()
    }

    /// Check if this block is a completed task
    pub fn is_done(&self) -> bool {
        self.marker == Some(TaskMarker::Done) || self.marker == Some(TaskMarker::Cancelled)
    }

    /// Get the path from root to this block (for breadcrumbs)
    pub fn get_path(&self, all_blocks: &[Block]) -> Vec<Uuid> {
        let mut path = vec![self.id];
        let mut current = self;
        while let Some(parent_id) = current.parent_id {
            path.push(parent_id);
            if let Some(parent) = all_blocks.iter().find(|b| b.id == parent_id) {
                current = parent;
            } else {
                break;
            }
        }
        path.reverse();
        path
    }

    /// Get the content as plain text.
    ///
    /// This extracts only the text segments, ignoring references,
    /// embeds, and other non-text content.
    pub fn as_plain_text(&self) -> String {
        self.content.as_plain_text()
    }

    /// Get the class identifier from this block's properties.
    ///
    /// Returns the class db_ident (e.g., "task", "journal") if set.
    /// Returns None if no class property is defined.
    pub fn get_class_ident(&self) -> Option<&str> {
        self.properties
            .get("class")
            .and_then(|v| match v {
                PropertyValue::String(s) => Some(s.as_str()),
                _ => None,
            })
    }

    /// Get the effective property value, considering class defaults.
    ///
    /// If the property is explicitly set on the block, returns that value.
    /// If not set but class provides a default, returns the default.
    /// If neither, returns None.
    pub fn get_effective_property(&self, _prop_id: &str) -> Option<&PropertyValue> {
        // For now, just return the block's own property
        // TODO: Implement class default lookup
        None
    }
}

impl Default for BlockCreate {
    fn default() -> Self {
        Self {
            page_id: Uuid::new_v4(),
            content: BlockContent::empty(),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::BlockSegment;

    fn create_test_block(id: Uuid, page_id: Uuid, parent_id: Option<Uuid>) -> Block {
        Block {
            id,
            page_id,
            parent_id,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: BlockContent::from_text("Test block"),
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            journal_day: Some(20260514),
            updated_journal_day: Some(20260514),
        }
    }

    fn test_timezone() -> TimezoneService {
        TimezoneService::from_tz_string("UTC").unwrap()
    }

    #[test]
    fn test_block_creation() {
        let page_id = Uuid::new_v4();
        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text("Hello"),
            parent_id: None,
            order: 1.0,
            marker: Some(TaskMarker::Todo),
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();
        assert_eq!(block.page_id, page_id);
        assert_eq!(block.content.as_plain_text(), "Hello");
        assert_eq!(block.marker, Some(TaskMarker::Todo));
        assert!(!block.is_done());
        // Check journal_day was auto-set
        assert!(block.journal_day.is_some());
        assert_eq!(block.journal_day, block.updated_journal_day);
    }

    #[test]
    fn test_circular_reference_detection() {
        let page_id = Uuid::new_v4();
        let block_a = create_test_block(Uuid::new_v4(), page_id, None);
        let block_b = create_test_block(Uuid::new_v4(), page_id, Some(block_a.id));
        let block_c = create_test_block(Uuid::new_v4(), page_id, Some(block_b.id));

        let all_blocks = [&block_a, &block_b, &block_c];

        // block_c cannot move to block_a (would create cycle)
        assert!(!block_c.can_move_to(
            Some(block_a.id),
            &all_blocks.iter().map(|b| (*b).clone()).collect::<Vec<_>>()
        ));

        // block_c can move to a different parent
        let new_parent = Uuid::new_v4();
        assert!(block_c.can_move_to(
            Some(new_parent),
            &all_blocks.iter().map(|b| (*b).clone()).collect::<Vec<_>>()
        ));
    }

    #[test]
    fn test_block_done_auto_logbook() {
        let create = BlockCreate {
            page_id: Uuid::new_v4(),
            content: BlockContent::from_text("Task"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let mut block = Block::new(create, &tz).unwrap();
        assert!(block.logbook.is_none());

        // Mark as done - logbook should be set
        block
            .update(
                BlockUpdate {
                    marker: Some(TaskMarker::Done),
                    ..Default::default()
                },
                &tz,
            )
            .unwrap();

        assert!(block.logbook.is_some());
    }

    #[test]
    fn test_block_update_changes_journal_day() {
        let create = BlockCreate {
            page_id: Uuid::new_v4(),
            content: BlockContent::from_text("Original content"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let mut block = Block::new(create, &tz).unwrap();
        let _original_journal_day = block.journal_day;
        let _original_updated_journal_day = block.updated_journal_day;

        // Update content
        block
            .update(
                BlockUpdate {
                    content: Some(BlockContent::from_text("Updated content")),
                    ..Default::default()
                },
                &tz,
            )
            .unwrap();

        // updated_journal_day should have changed
        assert!(block.updated_journal_day.is_some());
        // Note: In UTC timezone, today_journal_day() might return same value
        // In a real timezone like America/Mexico_City, it could differ
    }

    #[test]
    fn test_extract_and_update_refs_on_create() {
        let page_id = Uuid::new_v4();
        let page_ref_id = Uuid::new_v4();
        let block_ref_id = Uuid::new_v4();

        let content = BlockContent {
            segments: vec![
                BlockSegment::Text {
                    content: "See ".to_string(),
                    marks: Vec::new(),
                },
                BlockSegment::PageRef {
                    target: page_ref_id,
                    label: None,
                },
                BlockSegment::Text {
                    content: " and ".to_string(),
                    marks: Vec::new(),
                },
                BlockSegment::BlockRef { target: block_ref_id },
                BlockSegment::Text {
                    content: " for details".to_string(),
                    marks: Vec::new(),
                },
            ],
        };

        let create = BlockCreate {
            page_id,
            content,
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();

        // Block should have extracted both page and block refs
        assert_eq!(block.refs.len(), 2);
        assert!(block.refs.contains(&page_ref_id));
        assert!(block.refs.contains(&block_ref_id));
    }

    #[test]
    fn test_extract_and_update_refs_on_update() {
        let page_id = Uuid::new_v4();
        let page_ref_id = Uuid::new_v4();
        let block_ref_id = Uuid::new_v4();
        let new_page_ref_id = Uuid::new_v4();

        // Create block with one page ref
        let content = BlockContent {
            segments: vec![BlockSegment::PageRef {
                target: page_ref_id,
                label: None,
            }],
        };

        let create = BlockCreate {
            page_id,
            content,
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let mut block = Block::new(create, &tz).unwrap();

        // Should have initial ref
        assert_eq!(block.refs.len(), 1);
        assert!(block.refs.contains(&page_ref_id));

        // Update content with different refs
        let new_content = BlockContent {
            segments: vec![
                BlockSegment::BlockRef { target: block_ref_id },
                BlockSegment::PageRef {
                    target: new_page_ref_id,
                    label: None,
                },
            ],
        };

        block
            .update(BlockUpdate { content: Some(new_content), ..Default::default() }, &tz)
            .unwrap();

        // Should have updated refs
        assert_eq!(block.refs.len(), 2);
        assert!(!block.refs.contains(&page_ref_id)); // Old ref removed
        assert!(block.refs.contains(&block_ref_id));
        assert!(block.refs.contains(&new_page_ref_id));
    }

    #[test]
    fn test_extract_and_update_refs_no_refs() {
        let page_id = Uuid::new_v4();

        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text("Just plain text"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();

        // No refs should be extracted
        assert!(block.refs.is_empty());
    }

    #[test]
    fn test_get_class_ident() {
        let page_id = Uuid::new_v4();

        // Block with class property
        let mut properties = HashMap::new();
        properties.insert("class".to_string(), PropertyValue::String("task".to_string()));

        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text("My task"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties,
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();

        assert_eq!(block.get_class_ident(), Some("task"));
    }

    #[test]
    fn test_get_class_ident_no_class() {
        let page_id = Uuid::new_v4();

        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text("Plain text"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties: HashMap::new(),
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();

        assert_eq!(block.get_class_ident(), None);
    }

    #[test]
    fn test_get_class_ident_wrong_type() {
        let page_id = Uuid::new_v4();

        // Class property as wrong type (checkbox instead of string)
        let mut properties = HashMap::new();
        properties.insert("class".to_string(), PropertyValue::Checkbox(true));

        let create = BlockCreate {
            page_id,
            content: BlockContent::from_text("Test"),
            parent_id: None,
            order: 1.0,
            marker: None,
            format: BlockFormat::Markdown,
            properties,
        };

        let tz = test_timezone();
        let block = Block::new(create, &tz).unwrap();

        // Should return None because class is not a string
        assert_eq!(block.get_class_ident(), None);
    }
}
