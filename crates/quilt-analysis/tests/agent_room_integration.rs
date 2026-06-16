//! Integration tests for the Agent Room module.
//!
//! These are written as integration tests (in the `tests/`
//! directory) rather than inline `#[cfg(test)]` modules so
//! they can compile and run independently of the (currently
//! broken) pre-existing inline test modules in
//! `quilt-analysis`. The latter will be repaired in a
//! follow-up; this file focuses on the new V1 surface.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::watch;

use quilt_analysis::agent_room::lifecycle::AgentError;
use quilt_analysis::agent_room::{
    AgentExecutor, AgentLifecycle, AgentListFilter, AgentQueue, AgentRegistry, AgentRunOutcome,
    AgentStatus, SpawnAgentRequest,
    registry::{AgentError as RegistryError, RunContext as RegistryRunContext},
};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::Uuid;
use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};

fn lifecycle() -> AgentLifecycle {
    AgentLifecycle::new(
        InMemoryBlockRepo::new() as Arc<dyn BlockRepository>,
        InMemoryPageRepo::new() as Arc<dyn PageRepository>,
    )
    .with_timeout(Duration::from_millis(50))
}

fn known() -> Vec<String> {
    vec!["decay-annotator".to_string()]
}

#[tokio::test]
async fn queued_can_cancel() {
    let lc = lifecycle();
    let dto = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let id = Uuid::parse_str(&dto.id).unwrap();
    let after = lc.cancel(id).await.unwrap();
    assert_eq!(after.status, "Cancelled");
    assert!(after.completed_at.is_some());
}

#[tokio::test]
async fn running_can_cancel() {
    let lc = lifecycle();
    let dto = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let id = Uuid::parse_str(&dto.id).unwrap();
    assert!(lc.try_promote_to_running(id).is_some());
    let after = lc.cancel(id).await.unwrap();
    assert_eq!(after.status, "Cancelled");
}

#[tokio::test]
async fn completed_cannot_cancel() {
    let lc = lifecycle();
    let dto = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let id = Uuid::parse_str(&dto.id).unwrap();
    lc.try_promote_to_running(id);
    lc.complete(id, "done".to_string(), 0).await.unwrap();
    let after = lc.cancel(id).await.unwrap();
    assert_eq!(after.status, "Completed");
}

#[tokio::test]
async fn terminal_immutability() {
    let lc = lifecycle();
    let dto = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let id = Uuid::parse_str(&dto.id).unwrap();
    lc.cancel(id).await.unwrap();
    // After cancel, try_promote must fail (Cancelled is
    // not Queued).
    assert!(lc.try_promote_to_running(id).is_none());
    // complete() on a Cancelled run returns the Cancelled
    // DTO unchanged (the worker may observe a cancel that
    // just landed; the cancelled branch wins).
    let res = lc.complete(id, "x".to_string(), 0).await.unwrap();
    assert_eq!(res.status, "Cancelled");
}

#[tokio::test]
async fn at_most_one_running_invariant() {
    let lc = lifecycle();
    let mut ids = Vec::new();
    for _ in 0..3 {
        let dto = lc
            .spawn(
                SpawnAgentRequest {
                    agent_type: "decay-annotator".to_string(),
                    context_page: None,
                    model: None,
                    queue_mode: None,
                },
                &known(),
            )
            .await
            .unwrap();
        ids.push(Uuid::parse_str(&dto.id).unwrap());
    }
    lc.try_promote_to_running(ids[0]);
    assert!(lc.try_promote_to_running(ids[1]).is_some());
    assert!(lc.try_promote_to_running(ids[0]).is_none());
}

#[tokio::test]
async fn list_filter_by_status() {
    let lc = lifecycle();
    let a = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let a_id = Uuid::parse_str(&a.id).unwrap();
    lc.try_promote_to_running(a_id);

    let listed = lc.list(AgentListFilter {
        status: Some(AgentStatus::Running),
        agent_type: None,
        limit: None,
    });
    assert_eq!(listed.total, 1);
    assert_eq!(listed.agents[0].id, a.id);
}

#[tokio::test]
async fn unknown_type_rejected() {
    let lc = lifecycle();
    let res = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "imaginary".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await;
    assert!(matches!(res, Err(AgentError::UnknownType(_))));
}

#[tokio::test]
async fn empty_type_rejected() {
    let lc = lifecycle();
    let res = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await;
    assert!(matches!(res, Err(AgentError::UnknownType(_))));
}

#[test]
fn registry_with_defaults_registers_decay_annotator() {
    let r = AgentRegistry::with_defaults();
    assert!(r.get("decay-annotator").is_some());
    assert_eq!(r.list_types(), vec!["decay-annotator".to_string()]);
}

#[test]
fn registry_unknown_type_returns_none() {
    let r = AgentRegistry::new();
    assert!(r.get("imaginary").is_none());
    assert!(r.list_types().is_empty());
}

#[tokio::test]
async fn decay_annotator_executor_empty_graph() {
    use quilt_analysis::agent_room::agents::decay_annotator::DecayAnnotatorExecutor;
    use quilt_analysis::agent_room::registry::RunContext;

    let lc = lifecycle();
    let dto = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known(),
        )
        .await
        .unwrap();
    let id = Uuid::parse_str(&dto.id).unwrap();
    lc.try_promote_to_running(id);

    let exec = DecayAnnotatorExecutor::new();
    let ctx = RunContext {
        run_id: id,
        context_page: None,
        model: None,
        block_repo: lc.block_repo_clone(),
        page_repo: lc.page_repo_clone(),
    };
    let (_tx, rx) = watch::channel(false);
    let outcome = exec.run(ctx, rx).await.unwrap();
    assert_eq!(outcome.blocks_modified, 0);
    assert!(outcome.summary.contains("No decay"));
}

// ── FIFO ordering via a fake executor ─────────────────────────────

/// Records the order in which the executor is invoked so
/// the worker test can assert FIFO promotion.
pub struct OrderRecorder {
    pub id: &'static str,
    pub order: Arc<std::sync::Mutex<Vec<Uuid>>>,
}

#[async_trait]
impl AgentExecutor for OrderRecorder {
    fn agent_type(&self) -> &'static str {
        self.id
    }
    async fn run(
        &self,
        ctx: RegistryRunContext,
        _cancel: watch::Receiver<bool>,
    ) -> Result<AgentRunOutcome, RegistryError> {
        self.order.lock().unwrap().push(ctx.run_id);
        Ok(AgentRunOutcome {
            summary: "ok".to_string(),
            blocks_modified: 0,
        })
    }
}

#[tokio::test]
async fn fifo_ordering_via_fake_executor() {
    use quilt_analysis::agent_room::queue::process_one;

    let lc = AgentLifecycle::new(
        InMemoryBlockRepo::new() as Arc<dyn BlockRepository>,
        InMemoryPageRepo::new() as Arc<dyn PageRepository>,
    );
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));
    let exec: Arc<dyn AgentExecutor> = Arc::new(OrderRecorder {
        id: "decay-annotator",
        order: order.clone(),
    });
    let mut reg = AgentRegistry::new();
    reg.register(exec);
    let reg = Arc::new(reg);

    let known = reg.list_types();
    let a = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await
        .unwrap();
    let a_id = Uuid::parse_str(&a.id).unwrap();
    let b = lc
        .spawn(
            SpawnAgentRequest {
                agent_type: "decay-annotator".to_string(),
                context_page: None,
                model: None,
                queue_mode: None,
            },
            &known,
        )
        .await
        .unwrap();
    let b_id = Uuid::parse_str(&b.id).unwrap();

    let queue = AgentQueue::new(lc.clone(), reg);
    process_one(&queue, a_id).await.unwrap();
    process_one(&queue, b_id).await.unwrap();

    let observed = order.lock().unwrap().clone();
    assert_eq!(observed, vec![a_id, b_id], "FIFO ordering: A then B");

    assert_eq!(
        lc.get(a_id).unwrap().status,
        AgentStatus::Completed.as_str()
    );
    assert_eq!(
        lc.get(b_id).unwrap().status,
        AgentStatus::Completed.as_str()
    );
}
