//! TaskState value object - represents task block state
//!
//! This value object encapsulates the task-related state of a block:
/// marker (NOW, LATER, TODO, DONE, CANCELLED), priority, and logbook.

use chrono::{DateTime, Utc};

use super::{Priority, TaskMarker};

/// TaskState represents the task-related state of a block.
///
/// It encapsulates:
/// - `marker`: The task status marker (NOW, LATER, TODO, DONE, CANCELLED)
/// - `priority`: The task priority level (A, B, C)
/// - `logbook`: Timestamp when task was completed/cancelled (None if not done)
///
/// # Invariants
///
/// - If `marker` is `None`, then `priority` and `logbook` should also be `None`
///   (a block without a marker is not a task)
/// - `logbook` is only meaningful when `marker` is DONE or CANCELLED
#[derive(Debug, Clone, PartialEq)]
pub struct TaskState {
    /// Task marker (if this block is a task)
    pub marker: Option<TaskMarker>,
    /// Priority level (A, B, C)
    pub priority: Option<Priority>,
    /// Logbook state (CLOSED timestamp if done)
    pub logbook: Option<DateTime<Utc>>,
}

impl TaskState {
    /// Create a new task state from a marker.
    ///
    /// Priority and logbook are initially None.
    pub fn with_marker(marker: TaskMarker) -> Self {
        Self {
            marker: Some(marker),
            priority: None,
            logbook: None,
        }
    }

    /// Create a task state with marker and priority.
    pub fn with_priority(marker: TaskMarker, priority: Priority) -> Self {
        Self {
            marker: Some(marker),
            priority: Some(priority),
            logbook: None,
        }
    }

    /// Create an empty task state (no task).
    pub fn none() -> Self {
        Self {
            marker: None,
            priority: None,
            logbook: None,
        }
    }

    /// Check if this block is a task (has a marker).
    pub fn is_task(&self) -> bool {
        self.marker.is_some()
    }

    /// Check if this block is done (DONE or CANCELLED).
    pub fn is_done(&self) -> bool {
        self.marker == Some(TaskMarker::Done) || self.marker == Some(TaskMarker::Cancelled)
    }

    /// Check if this block is in progress (NOW or DOING marker).
    pub fn is_in_progress(&self) -> bool {
        self.marker == Some(TaskMarker::Now) || self.marker == Some(TaskMarker::Doing)
    }

    /// Check if this block is pending (TODO or LATER).
    pub fn is_pending(&self) -> bool {
        self.marker == Some(TaskMarker::Todo) || self.marker == Some(TaskMarker::Later)
    }

    /// Mark the task as done, setting the logbook timestamp.
    ///
    /// Returns a new TaskState with `marker = Done` and `logbook = Some(now)`.
    pub fn mark_done(&self, now: DateTime<Utc>) -> Self {
        Self {
            marker: Some(TaskMarker::Done),
            priority: self.priority,
            logbook: Some(now),
        }
    }

    /// Mark the task as cancelled, setting the logbook timestamp.
    ///
    /// Returns a new TaskState with `marker = Cancelled` and `logbook = Some(now)`.
    pub fn mark_cancelled(&self, now: DateTime<Utc>) -> Self {
        Self {
            marker: Some(TaskMarker::Cancelled),
            priority: self.priority,
            logbook: Some(now),
        }
    }

    /// Clear the done/cancelled state (reopen task).
    ///
    /// Returns a new TaskState with `marker = Todo` and `logbook = None`.
    pub fn reopen(&self) -> Self {
        Self {
            marker: Some(TaskMarker::Todo),
            priority: self.priority,
            logbook: None,
        }
    }

    /// Check if this task has a terminal marker (DONE or CANCELLED).
    pub fn has_terminal_marker(&self) -> bool {
        self.marker
            .map(|m| m.is_terminal())
            .unwrap_or(false)
    }
}

impl Default for TaskState {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_state_none() {
        let state = TaskState::none();
        assert!(!state.is_task());
        assert!(!state.is_done());
    }

    #[test]
    fn test_task_state_with_marker() {
        let state = TaskState::with_marker(TaskMarker::Todo);
        assert!(state.is_task());
        assert!(!state.is_done());
        assert!(state.is_pending());
    }

    #[test]
    fn test_task_state_done() {
        let state = TaskState::with_marker(TaskMarker::Done);
        assert!(state.is_done());
        assert!(state.has_terminal_marker());
    }

    #[test]
    fn test_task_state_cancelled() {
        let state = TaskState::with_marker(TaskMarker::Cancelled);
        assert!(state.is_done());
        assert!(state.has_terminal_marker());
    }

    #[test]
    fn test_task_state_now_in_progress() {
        let state = TaskState::with_marker(TaskMarker::Now);
        assert!(state.is_in_progress());
        assert!(!state.is_done());
    }

    #[test]
    fn test_mark_done_sets_logbook() {
        let state = TaskState::with_marker(TaskMarker::Todo);
        let now = Utc::now();
        let done_state = state.mark_done(now);
        
        assert_eq!(done_state.marker, Some(TaskMarker::Done));
        assert!(done_state.logbook.is_some());
    }

    #[test]
    fn test_reopen_clears_logbook() {
        let state = TaskState::with_marker(TaskMarker::Done);
        let now = Utc::now();
        let done_state = state.mark_done(now);
        let reopened = done_state.reopen();
        
        assert_eq!(reopened.marker, Some(TaskMarker::Todo));
        assert!(reopened.logbook.is_none());
    }
}
