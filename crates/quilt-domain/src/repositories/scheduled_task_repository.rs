//! ScheduledTaskRepository trait - persistence for scheduled tasks

use crate::entities::ScheduledTask;
use crate::errors::DomainError;
use async_trait::async_trait;

/// Repository for ScheduledTask persistence.
///
/// Used by the TaskScheduler to persist recurring tasks across restarts.
#[async_trait]
pub trait ScheduledTaskRepository: Send + Sync {
    /// Get a task by name
    async fn get_by_name(&self, name: &str) -> Result<Option<ScheduledTask>, DomainError>;

    /// Get all enabled tasks due for execution (next_run <= now)
    async fn list_due(&self) -> Result<Vec<ScheduledTask>, DomainError>;

    /// List all tasks
    async fn list_all(&self) -> Result<Vec<ScheduledTask>, DomainError>;

    /// Insert or update a task
    async fn upsert(&self, task: &ScheduledTask) -> Result<(), DomainError>;

    /// Delete a task by name
    async fn delete(&self, name: &str) -> Result<(), DomainError>;

    /// Update last_run and next_run after execution
    async fn mark_executed(
        &self,
        name: &str,
        next_run: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DomainError>;
}
