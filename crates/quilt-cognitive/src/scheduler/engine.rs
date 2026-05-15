//! TaskScheduler — integrated cron-like scheduler for background tasks
//!
//! Runs TreeRAG index rebuilds, health checks, and cleanup tasks.

use crate::scheduler::cron::{next_run, parse_cron};
use crate::tree_rag::TreeRagEngine;
use quilt_domain::entities::{ScheduledTask, TaskType};
use quilt_domain::repositories::ScheduledTaskRepository;
use std::sync::Arc;
use tracing::{error, info};

/// The TaskScheduler manages recurring tasks and executes them on schedule.
pub struct TaskScheduler {
    task_repo: Arc<dyn ScheduledTaskRepository>,
    tree_rag: Arc<TreeRagEngine>,
}

impl TaskScheduler {
    pub fn new(task_repo: Arc<dyn ScheduledTaskRepository>, tree_rag: Arc<TreeRagEngine>) -> Self {
        Self {
            task_repo,
            tree_rag,
        }
    }

    /// Start the scheduler worker loop (spawns a tokio task).
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                if let Err(e) = self.tick().await {
                    error!("Scheduler tick error: {}", e);
                }
            }
        })
    }

    /// Run one tick: check for due tasks and execute them.
    async fn tick(&self) -> Result<(), String> {
        let due_tasks = self
            .task_repo
            .list_due()
            .await
            .map_err(|e| format!("list_due: {}", e))?;

        for task in &due_tasks {
            if !task.enabled {
                continue;
            }

            let result = self.execute_task(task).await;
            match result {
                Ok(_) => {
                    // Compute next run
                    let schedule = parse_cron(&task.cron_expr)
                        .ok_or_else(|| format!("Invalid cron: {}", task.cron_expr))?;
                    let next = next_run(&schedule, chrono::Utc::now())
                        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24));

                    self.task_repo
                        .mark_executed(&task.name, next)
                        .await
                        .map_err(|e| format!("mark_executed: {}", e))?;
                }
                Err(e) => {
                    error!("Task {} failed: {}", task.name, e);
                }
            }
        }
        Ok(())
    }

    async fn execute_task(&self, task: &ScheduledTask) -> Result<(), String> {
        match &task.task_type {
            TaskType::RebuildIndex => {
                info!("Rebuilding TreeRAG index...");
                let count = self
                    .tree_rag
                    .rebuild_index(None)
                    .await
                    .map_err(|e| e.to_string())?;
                info!("TreeRAG index: {} stale blocks need summarization", count);
            }
            TaskType::CleanStaleSummaries => {
                info!("Cleaning stale summaries...");
                let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
                let stale_ids = self
                    .tree_rag
                    .summary_repo
                    .list_stale(cutoff)
                    .await
                    .map_err(|e| e.to_string())?;
                for block_id in &stale_ids {
                    self.tree_rag
                        .summary_repo
                        .delete(*block_id)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                info!("Cleaned {} stale summaries", stale_ids.len());
            }
            TaskType::HealthCheck => {
                let status = self.tree_rag.status().await.map_err(|e| e.to_string())?;
                info!(
                    "TreeRAG health: {}/{} blocks indexed ({} pending)",
                    status.indexed_blocks, status.total_blocks, status.pending_blocks
                );
            }
        }
        Ok(())
    }

    /// Schedule a new recurring task.
    pub async fn schedule_task(
        &self,
        name: &str,
        cron_expr: &str,
        task_type: TaskType,
    ) -> Result<(), String> {
        let schedule =
            parse_cron(cron_expr).ok_or_else(|| "Invalid cron expression".to_string())?;
        let next = next_run(&schedule, chrono::Utc::now())
            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24));

        let task = ScheduledTask::new(name, cron_expr, task_type, next);
        self.task_repo
            .upsert(&task)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// List all scheduled tasks.
    pub async fn list_tasks(&self) -> Result<Vec<ScheduledTask>, String> {
        self.task_repo.list_all().await.map_err(|e| e.to_string())
    }

    /// Delete a scheduled task by name.
    pub async fn delete_task(&self, name: &str) -> Result<(), String> {
        self.task_repo.delete(name).await.map_err(|e| e.to_string())
    }

    /// Run a specific task immediately.
    pub async fn run_now(&self, name: &str) -> Result<(), String> {
        let task = self
            .task_repo
            .get_by_name(name)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Task not found: {}", name))?;

        self.execute_task(&task).await
    }
}
