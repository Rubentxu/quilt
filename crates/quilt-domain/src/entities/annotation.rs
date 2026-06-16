//! Annotation entity - represents a comment/mark on a block
//!
//! Annotations are separate entities (Google Docs comments style) that allow
//! humans to request agent action on specific content without modifying it.
//! Agents read pending annotations via MCP, process them, and mark as resolved.

use crate::errors::DomainError;
use crate::value_objects::Uuid;
use chrono::{DateTime, Utc};

/// Author type - who created the annotation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorType {
    /// Human user
    Human,
    /// AI agent
    Agent,
}

impl AuthorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthorType::Human => "human",
            AuthorType::Agent => "agent",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "human" => Some(AuthorType::Human),
            "agent" => Some(AuthorType::Agent),
            _ => None,
        }
    }
}

/// Annotation status - lifecycle state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationStatus {
    /// Awaiting agent action
    Pending,
    /// Agent is actively working on this
    InProgress,
    /// Agent has resolved this annotation
    Resolved,
    /// Human dismissed this annotation
    Dismissed,
}

impl AnnotationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnnotationStatus::Pending => "pending",
            AnnotationStatus::InProgress => "in_progress",
            AnnotationStatus::Resolved => "resolved",
            AnnotationStatus::Dismissed => "dismissed",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(AnnotationStatus::Pending),
            "in_progress" => Some(AnnotationStatus::InProgress),
            "resolved" => Some(AnnotationStatus::Resolved),
            "dismissed" => Some(AnnotationStatus::Dismissed),
            _ => None,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AnnotationStatus::Resolved | AnnotationStatus::Dismissed
        )
    }
}

/// Annotation scope - what the annotation is attached to
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationScope {
    /// Annotation attached to the entire block
    Block,
    /// Annotation attached to a specific range within the block content
    Inline,
}

impl AnnotationScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            AnnotationScope::Block => "block",
            AnnotationScope::Inline => "inline",
        }
    }

    pub fn try_from_str(s: &str) -> Option<Self> {
        match s {
            "block" => Some(AnnotationScope::Block),
            "inline" => Some(AnnotationScope::Inline),
            _ => None,
        }
    }
}

/// Annotation represents a comment or mark on a block, directed at an agent
/// for review, correction, or enrichment. Like Google Docs comments.
///
/// # Fields
/// - `id`: Unique identifier
/// - `block_id`: The target block this annotation refers to
/// - `author_type`: Whether the author is a human or agent
/// - `author_name`: Name of the author (human username or agent ID)
/// - `content`: Annotation text in markdown
/// - `status`: Current lifecycle state
/// - `parent_annotation_id`: Optional parent for threaded replies
/// - `created_at`: When the annotation was created
/// - `resolved_at`: When the annotation was resolved (if applicable)
/// - `resolved_by`: Name of who resolved it (if applicable)
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub id: Uuid,
    pub block_id: Uuid,
    pub author_type: AuthorType,
    pub author_name: String,
    pub content: String,
    pub status: AnnotationStatus,
    pub parent_annotation_id: Option<Uuid>,
    pub scope: AnnotationScope,
    pub highlight_start: Option<u32>,
    pub highlight_end: Option<u32>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

/// Data required to create a new annotation
#[derive(Debug, Clone)]
pub struct AnnotationCreate {
    pub block_id: Uuid,
    pub author_type: AuthorType,
    pub author_name: String,
    pub content: String,
    pub parent_annotation_id: Option<Uuid>,
    pub scope: AnnotationScope,
    pub highlight_start: Option<u32>,
    pub highlight_end: Option<u32>,
}

impl Annotation {
    pub fn new(create: AnnotationCreate) -> Result<Self, DomainError> {
        if create.content.trim().is_empty() {
            return Err(DomainError::InvalidData(
                "Annotation content cannot be empty".to_string(),
            ));
        }

        // Inline scope requires both highlight offsets to be specified
        if create.scope == AnnotationScope::Inline {
            match (create.highlight_start, create.highlight_end) {
                (Some(start), Some(end)) if start < end => {}
                _ => {
                    return Err(DomainError::InvalidData(
                        "Inline annotation requires both highlightStart and highlightEnd offsets"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(Self {
            id: Uuid::new_v4(),
            block_id: create.block_id,
            author_type: create.author_type,
            author_name: create.author_name,
            content: create.content,
            status: AnnotationStatus::Pending,
            parent_annotation_id: create.parent_annotation_id,
            scope: create.scope,
            highlight_start: create.highlight_start,
            highlight_end: create.highlight_end,
            created_at: Utc::now(),
            resolved_at: None,
            resolved_by: None,
        })
    }

    pub fn resolve(&mut self, resolved_by: String) {
        self.status = AnnotationStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.resolved_by = Some(resolved_by);
    }

    pub fn set_in_progress(&mut self) {
        if self.status == AnnotationStatus::Pending {
            self.status = AnnotationStatus::InProgress;
        }
    }

    pub fn dismiss(&mut self) {
        self.status = AnnotationStatus::Dismissed;
    }

    pub fn is_pending(&self) -> bool {
        self.status == AnnotationStatus::Pending
    }

    pub fn is_resolved(&self) -> bool {
        self.status == AnnotationStatus::Resolved
    }

    pub fn belongs_to_block(&self, block_id: Uuid) -> bool {
        self.block_id == block_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_annotation(
        author_type: AuthorType,
        content: &str,
    ) -> Result<Annotation, DomainError> {
        Annotation::new(AnnotationCreate {
            block_id: Uuid::new_v4(),
            author_type,
            author_name: "test_user".to_string(),
            content: content.to_string(),
            parent_annotation_id: None,
            scope: AnnotationScope::Block,
            highlight_start: None,
            highlight_end: None,
        })
    }

    #[test]
    fn test_annotation_creation() {
        let annotation =
            create_test_annotation(AuthorType::Human, "Please expand this section").unwrap();
        assert!(annotation.id != Uuid::nil());
        assert_eq!(annotation.status, AnnotationStatus::Pending);
        assert_eq!(annotation.author_type, AuthorType::Human);
        assert_eq!(annotation.resolved_at, None);
        assert_eq!(annotation.resolved_by, None);
    }

    #[test]
    fn test_annotation_empty_content_rejected() {
        let result = create_test_annotation(AuthorType::Human, "   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_annotation_resolve() {
        let mut annotation = create_test_annotation(AuthorType::Agent, "Fixed the typo").unwrap();
        annotation.resolve("claude-desktop".to_string());

        assert_eq!(annotation.status, AnnotationStatus::Resolved);
        assert!(annotation.resolved_at.is_some());
        assert_eq!(annotation.resolved_by, Some("claude-desktop".to_string()));
    }

    #[test]
    fn test_annotation_set_in_progress() {
        let mut annotation = create_test_annotation(AuthorType::Human, "Please review").unwrap();
        assert!(annotation.is_pending());

        annotation.set_in_progress();
        assert_eq!(annotation.status, AnnotationStatus::InProgress);
    }

    #[test]
    fn test_annotation_dismiss() {
        let mut annotation = create_test_annotation(AuthorType::Human, "Not needed").unwrap();
        annotation.dismiss();
        assert_eq!(annotation.status, AnnotationStatus::Dismissed);
    }

    #[test]
    fn test_annotation_belongs_to_block() {
        let block_id = Uuid::new_v4();
        let annotation = Annotation::new(AnnotationCreate {
            block_id,
            author_type: AuthorType::Human,
            author_name: "ruben".to_string(),
            content: "Test".to_string(),
            parent_annotation_id: None,
            scope: AnnotationScope::Block,
            highlight_start: None,
            highlight_end: None,
        })
        .unwrap();

        assert!(annotation.belongs_to_block(block_id));
        assert!(!annotation.belongs_to_block(Uuid::new_v4()));
    }

    #[test]
    fn test_author_type_conversion() {
        assert_eq!(AuthorType::Human.as_str(), "human");
        assert_eq!(AuthorType::Agent.as_str(), "agent");
        assert_eq!(AuthorType::try_from_str("human"), Some(AuthorType::Human));
        assert_eq!(AuthorType::try_from_str("agent"), Some(AuthorType::Agent));
        assert_eq!(AuthorType::try_from_str("unknown"), None);
    }

    #[test]
    fn test_annotation_status_conversion() {
        assert_eq!(AnnotationStatus::Pending.as_str(), "pending");
        assert_eq!(AnnotationStatus::InProgress.as_str(), "in_progress");
        assert_eq!(AnnotationStatus::Resolved.as_str(), "resolved");
        assert_eq!(AnnotationStatus::Dismissed.as_str(), "dismissed");

        assert!(!AnnotationStatus::Pending.is_terminal());
        assert!(!AnnotationStatus::InProgress.is_terminal());
        assert!(AnnotationStatus::Resolved.is_terminal());
        assert!(AnnotationStatus::Dismissed.is_terminal());
    }
}
