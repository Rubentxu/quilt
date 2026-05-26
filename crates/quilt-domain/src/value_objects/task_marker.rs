//! TaskMarker value object - task status markers

use std::fmt;
use std::str::FromStr;

/// TaskMarker represents the status of a task block.
///
/// Tasks in Logseq have a lifecycle:
/// - NOW: Currently being worked on
/// - LATER: Planned for future
/// - TODO: Needs to be done
/// - DONE: Completed
/// - CANCELLED: Cancelled/abandoned
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
    /// Completed
    Done,
    /// Cancelled/abandoned
    Cancelled,
}

impl TaskMarker {
    /// Get the display name
    pub fn name(&self) -> &'static str {
        match self {
            TaskMarker::Now => "NOW",
            TaskMarker::Later => "LATER",
            TaskMarker::Todo => "TODO",
            TaskMarker::Done => "DONE",
            TaskMarker::Cancelled => "CANCELLED",
        }
    }

    /// Get the Logseq property value
    pub fn as_property_value(&self) -> &'static str {
        match self {
            TaskMarker::Now => "now",
            TaskMarker::Later => "later",
            TaskMarker::Todo => "todo",
            TaskMarker::Done => "done",
            TaskMarker::Cancelled => "cancelled",
        }
    }

    /// Check if this is a terminal state (DONE or CANCELLED)
    pub fn is_terminal(&self) -> bool {
        matches!(self, TaskMarker::Done | TaskMarker::Cancelled)
    }

    /// Check if this is an active state (NOW or LATER)
    pub fn is_active(&self) -> bool {
        matches!(self, TaskMarker::Now | TaskMarker::Later)
    }

    /// Check if this is pending (TODO or LATER)
    pub fn is_pending(&self) -> bool {
        matches!(self, TaskMarker::Todo | TaskMarker::Later)
    }

    /// Parse from string
    pub fn parse_str(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl FromStr for TaskMarker {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "now" => Ok(TaskMarker::Now),
            "later" => Ok(TaskMarker::Later),
            "todo" => Ok(TaskMarker::Todo),
            "done" => Ok(TaskMarker::Done),
            "cancelled" | "canceled" => Ok(TaskMarker::Cancelled),
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
            TaskMarker::Done,
            TaskMarker::Cancelled,
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
    }

    #[test]
    fn test_terminal_states() {
        assert!(TaskMarker::Done.is_terminal());
        assert!(TaskMarker::Cancelled.is_terminal());
        assert!(!TaskMarker::Todo.is_terminal());
    }

    #[test]
    fn test_from_str() {
        assert_eq!(TaskMarker::parse_str("todo"), Some(TaskMarker::Todo));
        assert_eq!(TaskMarker::parse_str("DONE"), Some(TaskMarker::Done));
        assert_eq!(
            TaskMarker::parse_str("canceled"),
            Some(TaskMarker::Cancelled)
        );
        assert_eq!(TaskMarker::parse_str("unknown"), None);
    }
}
