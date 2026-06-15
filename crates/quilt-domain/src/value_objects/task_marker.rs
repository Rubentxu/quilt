//! TaskMarker value object - task status markers

use crate::errors::DomainError;
use std::fmt;
use std::str::FromStr;

/// TaskMarker represents the status of a task block.
///
/// Tasks in Quilt have a lifecycle:
/// - NOW: Currently being worked on
/// - LATER: Planned for future
/// - TODO: Needs to be done
/// - DOING: In progress
/// - DONE: Completed
/// - CANCELLED: Cancelled/abandoned
/// - WAITING: Blocked on external dependency
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum TaskMarker {
    /// Currently being worked on
    Now,
    /// Planned for future
    Later,
    /// Needs to be done
    #[default]
    Todo,
    /// In progress
    Doing,
    /// Completed
    Done,
    /// Cancelled/abandoned
    Cancelled,
    /// Blocked on external dependency
    Waiting,
}

impl TaskMarker {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            TaskMarker::Now => "NOW",
            TaskMarker::Later => "LATER",
            TaskMarker::Todo => "TODO",
            TaskMarker::Doing => "DOING",
            TaskMarker::Done => "DONE",
            TaskMarker::Cancelled => "CANCELLED",
            TaskMarker::Waiting => "WAITING",
        }
    }

    /// Get the Quilt property value
    pub fn as_property_value(&self) -> &'static str {
        match self {
            TaskMarker::Now => "now",
            TaskMarker::Later => "later",
            TaskMarker::Todo => "todo",
            TaskMarker::Doing => "doing",
            TaskMarker::Done => "done",
            TaskMarker::Cancelled => "cancelled",
            TaskMarker::Waiting => "waiting",
        }
    }

    /// Check if this is a terminal state (DONE or CANCELLED)
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskMarker::Done | TaskMarker::Cancelled)
    }

    /// Check if this is an active state (NOW or DOING)
    pub fn is_active(&self) -> bool {
        matches!(self, TaskMarker::Now | TaskMarker::Doing)
    }

    /// Check if this is pending (TODO, LATER, or WAITING)
    pub fn is_pending(&self) -> bool {
        matches!(
            self,
            TaskMarker::Todo | TaskMarker::Later | TaskMarker::Waiting
        )
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Result<Self, DomainError> {
        s.parse()
            .map_err(|_| DomainError::ParseError(format!("Invalid task marker: {}", s)))
    }
}

impl FromStr for TaskMarker {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "now" => Ok(TaskMarker::Now),
            "later" => Ok(TaskMarker::Later),
            "todo" => Ok(TaskMarker::Todo),
            "doing" => Ok(TaskMarker::Doing),
            "done" => Ok(TaskMarker::Done),
            "cancelled" | "canceled" => Ok(TaskMarker::Cancelled),
            "waiting" => Ok(TaskMarker::Waiting),
            _ => Err(()),
        }
    }
}

impl TaskMarker {
    /// Get all valid markers in order
    pub fn all() -> &'static [TaskMarker] {
        &[
            TaskMarker::Now,
            TaskMarker::Later,
            TaskMarker::Todo,
            TaskMarker::Doing,
            TaskMarker::Done,
            TaskMarker::Cancelled,
            TaskMarker::Waiting,
        ]
    }
}

impl fmt::Display for TaskMarker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_properties() {
        assert_eq!(TaskMarker::Todo.as_property_value(), "todo");
        assert_eq!(TaskMarker::Done.as_property_value(), "done");
        // Waiting marker
        assert_eq!(TaskMarker::Waiting.as_property_value(), "waiting");
        assert_eq!(TaskMarker::Waiting.name(), "WAITING");
    }

    #[test]
    fn test_terminal_states() {
        assert!(TaskMarker::Done.is_terminal());
        assert!(TaskMarker::Cancelled.is_terminal());
        assert!(!TaskMarker::Todo.is_terminal());
        assert!(!TaskMarker::Waiting.is_terminal());
    }

    #[test]
    fn test_active_states() {
        // Active: Now and Doing
        assert!(TaskMarker::Now.is_active());
        assert!(TaskMarker::Doing.is_active());
        assert!(!TaskMarker::Todo.is_active());
        assert!(!TaskMarker::Waiting.is_active());
    }

    #[test]
    fn test_pending_states() {
        // Pending: Todo, Later, and Waiting
        assert!(TaskMarker::Todo.is_pending());
        assert!(TaskMarker::Later.is_pending());
        assert!(TaskMarker::Waiting.is_pending());
        assert!(!TaskMarker::Doing.is_pending());
        assert!(!TaskMarker::Done.is_pending());
    }

    #[test]
    fn test_from_str() {
        assert_eq!(TaskMarker::parse_str("todo"), Ok(TaskMarker::Todo));
        assert_eq!(TaskMarker::parse_str("DONE"), Ok(TaskMarker::Done));
        assert_eq!(TaskMarker::parse_str("canceled"), Ok(TaskMarker::Cancelled));
        assert!(TaskMarker::parse_str("unknown").is_err());
        // Waiting
        assert_eq!(TaskMarker::parse_str("waiting"), Ok(TaskMarker::Waiting));
        assert_eq!(TaskMarker::parse_str("WAITING"), Ok(TaskMarker::Waiting));
    }

    #[test]
    fn test_all_includes_waiting() {
        let all = TaskMarker::all();
        assert!(
            all.contains(&TaskMarker::Waiting),
            "all() should include Waiting"
        );
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(format!("{}", TaskMarker::Waiting), "WAITING");
        assert_eq!(format!("{}", TaskMarker::Todo), "TODO");
        assert_eq!(format!("{}", TaskMarker::Doing), "DOING");
    }
}
