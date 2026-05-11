//! ScheduledTask entity - recurring task managed by the TaskScheduler

use crate::value_objects::Uuid;
use serde::{Deserialize, Serialize};

/// A scheduled task that runs periodically via the TaskScheduler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Unique identifier
    pub id: Uuid,
    /// Human-readable name (e.g., "tree_rag_rebuild_index")
    pub name: String,
    /// Cron expression (5 fields: min hour day-month month day-week)
    pub cron_expr: String,
    /// Type of task to execute
    pub task_type: TaskType,
    /// Whether this task is currently enabled
    pub enabled: bool,
    /// Last execution timestamp (None if never run)
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    /// Next scheduled execution timestamp
    pub next_run: chrono::DateTime<chrono::Utc>,
    /// When this task was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// The type of work a scheduled task performs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskType {
    /// Rebuild the TreeRAG block summary index
    RebuildIndex,
    /// Clean up stale summaries (blocks deleted or content changed)
    CleanStaleSummaries,
    /// Report on index health
    HealthCheck,
}

impl ScheduledTask {
    pub fn new(
        name: impl Into<String>,
        cron_expr: impl Into<String>,
        task_type: TaskType,
        next_run: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            cron_expr: cron_expr.into(),
            task_type,
            enabled: true,
            last_run: None,
            next_run,
            created_at: now,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
