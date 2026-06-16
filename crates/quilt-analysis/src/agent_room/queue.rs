//! The Tokio worker that drives runs to completion.
//!
//! V1: ONE worker, sequential queue. The worker:
//! 1. Sleeps 100 ms while `is_any_active()` returns false.
//! 2. Pops the next id from the queue (FIFO).
//! 3. Skips ids whose record is no longer `Queued` (e.g.
//!    cancelled while waiting).
//! 4. Calls `try_promote_to_running` to advance the
//!    state machine.
//! 5. Looks up the executor in the registry; on
//!    miss, transitions to `Failed`.
//! 6. Awaits the executor's `run()`; maps the outcome to
//!    `Completed` (with summary + blocks_modified) or
//!    `Failed` (with error message).
//! 7. If the user cancelled, transitions to `Cancelled`.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use quilt_domain::value_objects::Uuid;

use super::lifecycle::AgentLifecycle;
use super::registry::{AgentError, AgentRegistry};

/// Cheap, cloneable handle. The worker is owned by the
/// caller (the HTTP server's startup code).
#[derive(Clone)]
pub struct AgentQueue {
    pub lifecycle: AgentLifecycle,
    pub registry: Arc<AgentRegistry>,
}

impl AgentQueue {
    pub fn new(lifecycle: AgentLifecycle, registry: Arc<AgentRegistry>) -> Self {
        Self {
            lifecycle,
            registry,
        }
    }
}

/// Spawn the worker. Returns a `JoinHandle<()>` the caller
/// may keep around (V1 does not abort it — the worker lives
/// for the lifetime of the process).
pub fn spawn_worker(queue: AgentQueue) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        run_loop(queue).await;
    })
}

/// One pass of the worker loop. Exposed as a free function
/// (rather than only through `spawn_worker`) so tests can
/// drive it directly with a `tokio::time::pause()` /
/// `advance()` pair.
pub async fn run_loop(queue: AgentQueue) {
    let mut backoff = Duration::from_millis(100);
    loop {
        if !queue.lifecycle.is_any_active() {
            tokio::time::sleep(backoff).await;
            continue;
        }
        // We have something to do. Reset backoff to the
        // minimum — the next idle period will raise it
        // again if applicable.
        backoff = Duration::from_millis(50);

        // Pop the next run.
        let Some(id) = queue.lifecycle.pop_next() else {
            // Race: the record was cancelled and removed.
            // Loop again.
            continue;
        };

        if let Err(e) = process_one(&queue, id).await {
            // Defensive: an error here is a bug, not a
            // normal failure. Log and continue so the
            // worker does not die.
            eprintln!("agent_room: worker error processing {id}: {e}");
        }
    }
}

/// Process one run. Public for tests.
pub async fn process_one(queue: &AgentQueue, id: Uuid) -> Result<(), String> {
    // 1. Skip if the record is no longer Queued.
    let promoted = queue.lifecycle.try_promote_to_running(id);
    if promoted.is_none() {
        return Ok(()); // cancelled while queued — nothing to do
    }
    // Persist the new state (the worker writes the block
    // even on the Queued→Running transition so the
    // AgentActivityFeed sees the live status).
    if let Err(e) = queue.lifecycle.persist_status_to_block(id).await {
        eprintln!("agent_room: persist Running failed: {e}");
    }

    // 2. Find the executor.
    let Some(executor) = pick_executor(queue, id) else {
        // No executor for this type — fail the run.
        let _ = queue
            .lifecycle
            .fail(id, "Agent executor not registered".to_string())
            .await;
        return Ok(());
    };

    // 3. Build the RunContext.
    let ctx = match build_context(queue, id) {
        Ok(c) => c,
        Err(e) => {
            let _ = queue.lifecycle.fail(id, e).await;
            return Ok(());
        }
    };

    // 4. Wire the cancel receiver from the record.
    let cancel_rx = cancel_receiver_for(queue, id);

    // 5. Run.
    let outcome = executor.run(ctx, cancel_rx).await;

    // 6. Decide terminal state. If the executor returned
    //    Err or the cancel was observed, the user wins.
    let final_status = queue.lifecycle.get(id).map(|d| d.status);
    match (final_status.as_deref(), outcome) {
        (Some("Cancelled"), _) => {
            // User-side cancel already won; do nothing.
        }
        (_, Ok(out)) => {
            let _ = queue
                .lifecycle
                .complete(id, out.summary, out.blocks_modified)
                .await;
        }
        (_, Err(AgentError::Repository(msg))) => {
            let _ = queue
                .lifecycle
                .fail(id, format!("Repository error: {msg}"))
                .await;
        }
        (_, Err(e)) => {
            let _ = queue.lifecycle.fail(id, format!("Agent error: {e}")).await;
        }
    }
    Ok(())
}

fn pick_executor(queue: &AgentQueue, id: Uuid) -> Option<Arc<dyn super::registry::AgentExecutor>> {
    let dto = queue.lifecycle.get(id)?;
    queue.registry.get(&dto.agent_type)
}

fn build_context(queue: &AgentQueue, id: Uuid) -> Result<super::registry::RunContext, String> {
    let dto = queue
        .lifecycle
        .get(id)
        .ok_or_else(|| "lost record".to_string())?;
    Ok(super::registry::RunContext {
        run_id: id,
        context_page: dto.context_page.clone(),
        model: dto.model.clone(),
        block_repo: queue.lifecycle.block_repo_clone(),
        page_repo: queue.lifecycle.page_repo_clone(),
    })
}

fn cancel_receiver_for(queue: &AgentQueue, id: Uuid) -> watch::Receiver<bool> {
    queue.lifecycle.cancel_receiver_clone(id)
}

// ── test helpers on AgentLifecycle ─────────────────────────────────

impl AgentLifecycle {
    /// Clone of the `block_repo` for executor use. The
    /// field is `pub(crate)`; this getter exposes it
    /// without making the field public.
    pub fn block_repo_clone(&self) -> Arc<dyn quilt_domain::repositories::BlockRepository> {
        self.block_repo.clone()
    }

    /// Clone of the `page_repo` for executor use.
    pub fn page_repo_clone(&self) -> Arc<dyn quilt_domain::repositories::PageRepository> {
        self.page_repo.clone()
    }

    /// Build a `watch::Receiver<bool>` that observes the
    /// cancel signal of the given run. Used by the worker
    /// to wire cooperative cancel into the executor.
    pub fn cancel_receiver_clone(&self, id: Uuid) -> watch::Receiver<bool> {
        let recs = self.records.read();
        recs.get(&id)
            .map(|r| r.cancel_tx.subscribe())
            .expect("record exists")
    }
}

#[cfg(test)]
mod tests_disabled {
    // Tests moved to `tests/agent_room_integration.rs` (the
    // FIFO ordering test using a fake `OrderRecorder`).
}
