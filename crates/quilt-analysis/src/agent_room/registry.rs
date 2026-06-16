//! `AgentExecutor` trait + `AgentRegistry`.
//!
//! The trait is the abstraction that lets new agent types be
//! plugged in without changing the dispatcher. The registry
//! is the lookup table the dispatcher and the `POST /agents`
//! handler both consult.
//!
//! V1 ships ONE executor: `decay-annotator`. Adding a new
//! type is a single `registry.register(Arc::new(MyExecutor))`
//! call at startup — no handler changes.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;
use thiserror::Error;
use tokio::sync::watch;

/// Result of one `run()` call. The `summary` becomes the
/// `summary` field on the `AgentDto`; `blocks_modified` is
/// the count of new blocks the executor wrote to the graph.
#[derive(Debug, Clone)]
pub struct AgentRunOutcome {
    pub summary: String,
    pub blocks_modified: u32,
}

/// Context passed to every executor. Carries the
/// user-supplied `context_page` and `model` (informational),
/// the run id, and a clone of the repositories.
#[derive(Clone)]
pub struct RunContext {
    pub run_id: Uuid,
    pub context_page: Option<String>,
    pub model: Option<String>,
    pub block_repo: Arc<dyn BlockRepository>,
    pub page_repo: Arc<dyn PageRepository>,
}

/// Errors an executor can return. The `Internal` variant is
/// the catch-all — the lifecycle will surface a short
/// message on the AgentDto and log the full chain.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Repository error: {0}")]
    Repository(String),
    #[error("Internal: {0}")]
    Internal(String),
}

/// The trait every concrete agent type implements. One
/// instance per type, registered in the `AgentRegistry`.
#[async_trait]
pub trait AgentExecutor: Send + Sync {
    /// Stable string id the registry uses as a key. Must
    /// match the value the client sends in `agentType`.
    fn agent_type(&self) -> &'static str;

    /// Run the agent. The `cancel` receiver is a
    /// `watch::Receiver<bool>` — the executor SHOULD poll
    /// `cancel.has_changed()` (or `cancel.changed().await`)
    /// at every checkpoint and return early if `true`. The
    /// lifecycle interprets an early return (or any error)
    /// after a cancel as `Cancelled`; an error before a
    /// cancel is interpreted as `Failed`.
    async fn run(
        &self,
        ctx: RunContext,
        cancel: watch::Receiver<bool>,
    ) -> Result<AgentRunOutcome, AgentError>;
}

/// Registry of available agent types. Cloneable and
/// thread-safe — the worker holds an `Arc<AgentRegistry>`.
#[derive(Clone, Default)]
pub struct AgentRegistry {
    executors: HashMap<String, Arc<dyn AgentExecutor>>,
}

impl std::fmt::Debug for AgentRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRegistry")
            .field("types", &self.executors.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl AgentRegistry {
    /// Empty registry. Use `register` to populate.
    pub fn new() -> Self {
        Self::default()
    }

    /// Convenience: build a registry that contains every
    /// built-in V1 executor. The caller does NOT have to
    /// register them manually.
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        r.register(Arc::new(
            crate::agent_room::agents::decay_annotator::DecayAnnotatorExecutor::new(),
        ));
        r
    }

    /// Register a new executor. Overwrites any existing
    /// entry under the same `agent_type` — useful in tests
    /// for swapping in a fake.
    pub fn register(&mut self, e: Arc<dyn AgentExecutor>) {
        self.executors.insert(e.agent_type().to_string(), e);
    }

    /// Look up an executor by type. Returns `None` for
    /// unknown types.
    pub fn get(&self, agent_type: &str) -> Option<Arc<dyn AgentExecutor>> {
        self.executors.get(agent_type).cloned()
    }

    /// List the registered type ids (sorted ASC for stable
    /// error messages).
    pub fn list_types(&self) -> Vec<String> {
        let mut v: Vec<String> = self.executors.keys().cloned().collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests_disabled {
    // Tests moved to `tests/agent_room_integration.rs`.
}
