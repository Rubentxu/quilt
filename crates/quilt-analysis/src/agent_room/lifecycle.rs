//! The agent lifecycle: state machine + persistence + signal.
//!
//! Per ADR-0015 the persisted state is the `type:: agent-run`
//! block in the graph. This module:
//! 1. Holds the in-memory `AgentRunRecord` map (status +
//!    cancel signal + small metadata).
//! 2. Writes the same data to the AgentRun block on every
//!    transition so the renderer + future restarts see the
//!    truth.
//! 3. Enforces the state machine (terminal states are
//!    absorbing; see the `agent-room-lifecycle` spec).
//!
//! The state machine is *single-writer*: the worker
//! (in `queue.rs`) is the only component that promotes
//! `Queued → Running` and finalises a run. The cancel
//! handler is the only component that may transition to
//! `Cancelled` while the run is `Queued` or `Running`.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use thiserror::Error;
use tokio::sync::watch;

use quilt_domain::entities::Block;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid};

use super::types::{AgentDto, AgentListResponse, AgentStatus, SpawnAgentRequest};

/// Maximum single-run duration before the watcher forces a
/// transition to `Failed`. Documented in the lifecycle spec.
pub const DEFAULT_AGENT_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Default cap on the number of agents returned by `list()`.
pub const DEFAULT_AGENT_LIST_LIMIT: usize = 50;

/// Optional filter for `list()`. Every field is an AND; the
/// absence of a field means "no filter on that dimension".
#[derive(Debug, Clone, Default)]
pub struct AgentListFilter {
    pub status: Option<AgentStatus>,
    pub agent_type: Option<String>,
    pub limit: Option<usize>,
}

/// Errors that the lifecycle can surface to the HTTP layer.
#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),
    #[error("Unknown agent type: {0}")]
    UnknownType(String),
    #[error("Invalid transition from {from} to {to}")]
    InvalidTransition { from: String, to: String },
    #[error("Repository error: {0}")]
    Repository(String),
}

impl From<quilt_domain::errors::DomainError> for AgentError {
    fn from(e: quilt_domain::errors::DomainError) -> Self {
        AgentError::Repository(e.to_string())
    }
}

/// In-memory record for one agent run. The `cancel_tx` is a
/// `watch::Sender<bool>` — the worker holds the receiver and
/// the lifecycle / cancel handler flips it to `true` to ask
/// the worker to stop at the next checkpoint.
pub struct AgentRunRecord {
    pub id: Uuid,
    pub agent_type: String,
    pub model: Option<String>,
    pub context_page: Option<String>,
    pub status: AgentStatus,
    pub summary: Option<String>,
    pub error: Option<String>,
    pub blocks_modified: u32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancel_tx: watch::Sender<bool>,
}

impl AgentRunRecord {
    fn to_dto(&self) -> AgentDto {
        AgentDto {
            id: self.id.to_string(),
            agent_type: self.agent_type.clone(),
            model: self.model.clone(),
            status: self.status.as_str().to_string(),
            context_page: self.context_page.clone(),
            summary: self.summary.clone(),
            blocks_modified: self.blocks_modified,
            started_at: self.started_at,
            completed_at: self.completed_at,
            error: self.error.clone(),
        }
    }
}

/// The state machine + persistence layer. Cloneable, cheap.
#[derive(Clone)]
pub struct AgentLifecycle {
    pub(crate) records: Arc<RwLock<HashMap<Uuid, AgentRunRecord>>>,
    pub(crate) queue: Arc<Mutex<VecDeque<Uuid>>>,
    pub(crate) block_repo: Arc<dyn BlockRepository>,
    pub(crate) page_repo: Arc<dyn PageRepository>,
    timeout: Duration,
}

impl AgentLifecycle {
    /// Build a new lifecycle. The timeout defaults to
    /// `DEFAULT_AGENT_TIMEOUT`; tests can override via
    /// `with_timeout`.
    pub fn new(block_repo: Arc<dyn BlockRepository>, page_repo: Arc<dyn PageRepository>) -> Self {
        Self {
            records: Arc::new(RwLock::new(HashMap::new())),
            queue: Arc::new(Mutex::new(VecDeque::new())),
            block_repo,
            page_repo,
            timeout: DEFAULT_AGENT_TIMEOUT,
        }
    }

    /// Override the timeout (for tests). Returns `self` for
    /// builder-style chaining.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// True if at least one record is `Running` or in the queue
    /// waiting to be picked up. Used by the worker loop to
    /// decide whether to sleep.
    pub fn is_any_active(&self) -> bool {
        let recs = self.records.read();
        recs.values()
            .any(|r| matches!(r.status, AgentStatus::Queued | AgentStatus::Running))
    }

    /// Pop the next run id from the FIFO queue. The queue
    /// itself enforces FIFO ordering; the caller (worker) is
    /// expected to verify the record is still `Queued` before
    /// promoting.
    pub fn pop_next(&self) -> Option<Uuid> {
        self.queue.lock().pop_front()
    }

    /// Look up a record. Returns the DTO view, or `None`.
    pub fn get(&self, id: Uuid) -> Option<AgentDto> {
        self.records.read().get(&id).map(|r| r.to_dto())
    }

    /// List records, newest first (by `started_at` then by
    /// insertion order for `Queued` records). The `total`
    /// field is the size of the full registry, not the
    /// truncated list.
    pub fn list(&self, filter: AgentListFilter) -> AgentListResponse {
        let recs = self.records.read();
        let mut all: Vec<&AgentRunRecord> = recs.values().collect();
        all.sort_by(|a, b| {
            b.started_at
                .cmp(&a.started_at)
                .then_with(|| a.id.to_string().cmp(&b.id.to_string()))
        });
        if let Some(s) = filter.status {
            all.retain(|r| r.status == s);
        }
        if let Some(t) = &filter.agent_type {
            all.retain(|r| &r.agent_type == t);
        }
        let total = all.len();
        let limit = filter.limit.unwrap_or(DEFAULT_AGENT_LIST_LIMIT);
        let agents: Vec<AgentDto> = all.into_iter().take(limit).map(|r| r.to_dto()).collect();
        AgentListResponse { agents, total }
    }

    /// Create a new run. Validates the type is non-empty,
    /// inserts the record in `Queued`, enqueues, and writes
    /// the initial AgentRun block to the graph (so the run
    /// shows up in the `AgentActivityFeed` even before the
    /// worker picks it up).
    pub async fn spawn(
        &self,
        req: SpawnAgentRequest,
        known_types: &[String],
    ) -> Result<AgentDto, AgentError> {
        if req.agent_type.is_empty() {
            return Err(AgentError::UnknownType("(empty)".to_string()));
        }
        if !known_types.iter().any(|t| t == &req.agent_type) {
            return Err(AgentError::UnknownType(req.agent_type.clone()));
        }

        let id = Uuid::new_v4();
        let (cancel_tx, _cancel_rx) = watch::channel(false);

        let record = AgentRunRecord {
            id,
            agent_type: req.agent_type.clone(),
            model: req.model.clone(),
            context_page: req.context_page.clone(),
            status: AgentStatus::Queued,
            summary: None,
            error: None,
            blocks_modified: 0,
            started_at: None,
            completed_at: None,
            cancel_tx,
        };

        // Persist initial AgentRun block (Queued). Best-effort:
        // a failure here surfaces as a Repository error so the
        // client knows the spawn did not stick.
        self.write_agent_run_block(&record).await?;

        // Insert into registry + queue.
        {
            let mut recs = self.records.write();
            recs.insert(id, record);
        }
        self.queue.lock().push_back(id);

        // Spawn the timeout watcher — a detached task that
        // transitions the run to `Failed` if the worker does
        // not finish it in time.
        self.spawn_timeout_watcher(id);

        Ok(self.get(id).expect("just-inserted record"))
    }

    /// Cancel a run. Idempotent: if the run is already
    /// terminal, returns the current DTO unchanged.
    pub async fn cancel(&self, id: Uuid) -> Result<AgentDto, AgentError> {
        // Pull the record out, signal cancel, persist.
        let snapshot = {
            let mut recs = self.records.write();
            let rec = recs
                .get_mut(&id)
                .ok_or(AgentError::NotFound(id.to_string()))?;
            match rec.status {
                AgentStatus::Queued | AgentStatus::Running => {
                    // signal the worker
                    let _ = rec.cancel_tx.send(true);
                    rec.status = AgentStatus::Cancelled;
                    rec.completed_at = Some(Utc::now());
                    rec.to_dto()
                }
                _ => {
                    // terminal: idempotent no-op
                    rec.to_dto()
                }
            }
        };
        let _ = self.persist_status_to_block(id).await;
        Ok(snapshot)
    }

    /// Worker-side: promote `Queued → Running`. Returns the
    /// DTO on success, or `None` if the record is no longer
    /// in `Queued` (e.g. cancelled while waiting).
    pub fn try_promote_to_running(&self, id: Uuid) -> Option<AgentDto> {
        let mut recs = self.records.write();
        let rec = recs.get_mut(&id)?;
        if rec.status != AgentStatus::Queued {
            return None;
        }
        rec.status = AgentStatus::Running;
        rec.started_at = Some(Utc::now());
        Some(rec.to_dto())
    }

    /// Worker-side: check the cancel signal. Returns `true`
    /// if the user asked to cancel.
    pub fn is_cancelled(&self, id: Uuid) -> bool {
        let recs = self.records.read();
        recs.get(&id)
            .map(|r| r.cancel_tx.borrow().clone())
            .unwrap_or(false)
    }

    /// Worker-side: mark `Running → Completed`. `summary` and
    /// `blocks_modified` are stored on the record and
    /// persisted to the AgentRun block.
    pub async fn complete(
        &self,
        id: Uuid,
        summary: String,
        blocks_modified: u32,
    ) -> Result<AgentDto, AgentError> {
        let snapshot = {
            let mut recs = self.records.write();
            let rec = recs
                .get_mut(&id)
                .ok_or(AgentError::NotFound(id.to_string()))?;
            // Worker may observe a cancel that just landed; if so,
            // the cancelled branch wins.
            if matches!(rec.status, AgentStatus::Cancelled) {
                return Ok(rec.to_dto());
            }
            if rec.status != AgentStatus::Running {
                return Err(AgentError::InvalidTransition {
                    from: rec.status.as_str().to_string(),
                    to: "Completed".to_string(),
                });
            }
            rec.status = AgentStatus::Completed;
            rec.summary = Some(summary);
            rec.blocks_modified = blocks_modified;
            rec.completed_at = Some(Utc::now());
            rec.to_dto()
        };
        let _ = self.persist_status_to_block(id).await;
        Ok(snapshot)
    }

    /// Worker-side: mark `Running → Failed`.
    pub async fn fail(&self, id: Uuid, error: String) -> Result<AgentDto, AgentError> {
        let snapshot = {
            let mut recs = self.records.write();
            let rec = recs
                .get_mut(&id)
                .ok_or(AgentError::NotFound(id.to_string()))?;
            if matches!(rec.status, AgentStatus::Cancelled) {
                return Ok(rec.to_dto());
            }
            if rec.status != AgentStatus::Running {
                return Err(AgentError::InvalidTransition {
                    from: rec.status.as_str().to_string(),
                    to: "Failed".to_string(),
                });
            }
            rec.status = AgentStatus::Failed;
            rec.error = Some(error);
            rec.completed_at = Some(Utc::now());
            rec.to_dto()
        };
        let _ = self.persist_status_to_block(id).await;
        Ok(snapshot)
    }

    /// Worker-side: handle a cooperative cancel. The worker
    /// calls this after observing `is_cancelled() == true`.
    pub async fn on_worker_cancelled(&self, id: Uuid) -> Result<AgentDto, AgentError> {
        // If the user-side cancel handler already flipped the
        // status, this is a no-op.
        let snapshot = {
            let mut recs = self.records.write();
            let rec = recs
                .get_mut(&id)
                .ok_or(AgentError::NotFound(id.to_string()))?;
            if rec.status == AgentStatus::Cancelled {
                return Ok(rec.to_dto());
            }
            if rec.status != AgentStatus::Running {
                return Err(AgentError::InvalidTransition {
                    from: rec.status.as_str().to_string(),
                    to: "Cancelled".to_string(),
                });
            }
            rec.status = AgentStatus::Cancelled;
            rec.completed_at = Some(Utc::now());
            rec.to_dto()
        };
        let _ = self.persist_status_to_block(id).await;
        Ok(snapshot)
    }

    /// Worker-side: update `blocks_modified` (e.g. after each
    /// annotation write). The new value is reflected in the
    /// in-memory record only — the AgentRun block is updated
    /// at terminal transition.
    pub fn bump_blocks(&self, id: Uuid, delta: u32) {
        let mut recs = self.records.write();
        if let Some(rec) = recs.get_mut(&id) {
            rec.blocks_modified = rec.blocks_modified.saturating_add(delta);
        }
    }

    // ── persistence helpers ─────────────────────────────────

    /// Write the current state of the record to the
    /// underlying AgentRun block. Idempotent — the block's
    /// `run-status`, `summary`, `error`, `started-at`,
    /// `completed-at` properties are updated to match.
    pub(crate) async fn persist_status_to_block(&self, id: Uuid) -> Result<(), AgentError> {
        let snapshot = {
            let recs = self.records.read();
            recs.get(&id).map(|r| r.to_dto())
        };
        let dto = snapshot.ok_or(AgentError::NotFound(id.to_string()))?;
        self.write_agent_run_block_from_dto(&dto).await
    }

    /// Resolve a `context_page` name (or the synthetic
    /// `"agents/runs"` fallback) to a `Uuid` page id. The
    /// page is created on demand if it does not exist —
    /// annotation runs are about a meta-resource that
    /// shouldn't pollute the user's primary namespace, so we
    /// use a stable, dedicated page.
    async fn resolve_or_create_page_id(&self, page_name: &str) -> Uuid {
        if let Ok(Some(p)) = self.page_repo.get_by_name(page_name).await {
            return p.id;
        }
        // Try to create a page; if it already exists (race),
        // re-fetch.
        use quilt_domain::entities::{Page, PageCreate};
        let create = PageCreate {
            name: page_name.to_string(),
            title: Some(page_name.to_string()),
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            properties: std::collections::HashMap::new(),
        };
        match Page::new(create) {
            Ok(page) => {
                if self.page_repo.insert(&page).await.is_err() {
                    if let Ok(Some(p)) = self.page_repo.get_by_name(page_name).await {
                        return p.id;
                    }
                }
                page.id
            }
            Err(_) => {
                // Fallback: maybe the name is already taken
                // (race with another insert).
                if let Ok(Some(p)) = self.page_repo.get_by_name(page_name).await {
                    return p.id;
                }
                // Last resort: a fresh uuid. The block will
                // be orphaned but the registry still works.
                Uuid::new_v4()
            }
        }
    }

    /// Write the initial AgentRun block on spawn.
    async fn write_agent_run_block(&self, rec: &AgentRunRecord) -> Result<(), AgentError> {
        let page_name = rec
            .context_page
            .clone()
            .unwrap_or_else(|| "agents/runs".to_string());
        let page_id = self.resolve_or_create_page_id(&page_name).await;
        let block = self.build_agent_run_block(rec.id, page_id, &page_name, rec);
        self.block_repo
            .insert(&block)
            .await
            .map_err(|e| AgentError::Repository(e.to_string()))?;
        Ok(())
    }

    async fn write_agent_run_block_from_dto(&self, dto: &AgentDto) -> Result<(), AgentError> {
        let page_name = dto
            .context_page
            .clone()
            .unwrap_or_else(|| "agents/runs".to_string());
        let page_id = self.resolve_or_create_page_id(&page_name).await;
        let block_id = Uuid::parse_str(&dto.id)
            .map_err(|_| AgentError::Repository(format!("invalid uuid: {}", dto.id)))?;
        // Build a temporary "view" struct that mirrors the
        // record fields. We use the dto directly so updates
        // don't require a record lookup.
        let now = Utc::now();
        let mut props: std::collections::HashMap<String, PropertyValue> =
            std::collections::HashMap::new();
        props.insert("type".into(), PropertyValue::string("agent-run"));
        props.insert(
            "agent".into(),
            PropertyValue::string(dto.agent_type.clone()),
        );
        if let Some(m) = &dto.model {
            props.insert("model".into(), PropertyValue::string(m.clone()));
        }
        props.insert(
            "run-status".into(),
            PropertyValue::string(dto.status.clone()),
        );
        if let Some(s) = dto.started_at {
            props.insert("started-at".into(), PropertyValue::string(s.to_rfc3339()));
        }
        if let Some(c) = dto.completed_at {
            props.insert("completed-at".into(), PropertyValue::string(c.to_rfc3339()));
        }
        if let Some(s) = &dto.summary {
            props.insert("summary".into(), PropertyValue::string(s.clone()));
        }
        if let Some(e) = &dto.error {
            props.insert("error".into(), PropertyValue::string(e.clone()));
        }
        props.insert("agent-run-id".into(), PropertyValue::string(dto.id.clone()));

        let block = Block {
            id: block_id,
            page_id,
            parent_id: None,
            order: 0.0,
            level: 1,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            marker: None,
            priority: None,
            content: format!("🤖 Agent run: {}", dto.agent_type),
            properties: props,
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: dto.completed_at,
            cancelled_at: None,
            collapsed: false,
            created_at: dto.started_at.unwrap_or(now),
            updated_at: now,
        };
        // Use update if the block exists, else insert.
        match self.block_repo.get_by_id(block_id).await {
            Ok(Some(_)) => self
                .block_repo
                .update(&block)
                .await
                .map_err(|e| AgentError::Repository(e.to_string()))?,
            Ok(None) => self
                .block_repo
                .insert(&block)
                .await
                .map_err(|e| AgentError::Repository(e.to_string()))?,
            Err(e) => return Err(AgentError::Repository(e.to_string())),
        }
        Ok(())
    }

    fn build_agent_run_block(
        &self,
        id: Uuid,
        page_id: Uuid,
        _page_name: &str,
        rec: &AgentRunRecord,
    ) -> Block {
        let now = Utc::now();
        let mut props: std::collections::HashMap<String, PropertyValue> =
            std::collections::HashMap::new();
        props.insert("type".into(), PropertyValue::string("agent-run"));
        props.insert(
            "agent".into(),
            PropertyValue::string(rec.agent_type.clone()),
        );
        if let Some(m) = &rec.model {
            props.insert("model".into(), PropertyValue::string(m.clone()));
        }
        props.insert(
            "run-status".into(),
            PropertyValue::string(rec.status.as_str().to_string()),
        );
        if let Some(s) = rec.started_at {
            props.insert("started-at".into(), PropertyValue::string(s.to_rfc3339()));
        }
        if let Some(c) = rec.completed_at {
            props.insert("completed-at".into(), PropertyValue::string(c.to_rfc3339()));
        }
        if let Some(s) = &rec.summary {
            props.insert("summary".into(), PropertyValue::string(s.clone()));
        }
        if let Some(e) = &rec.error {
            props.insert("error".into(), PropertyValue::string(e.clone()));
        }
        props.insert("agent-run-id".into(), PropertyValue::string(id.to_string()));

        Block {
            id,
            page_id,
            parent_id: None,
            order: 0.0,
            level: 1,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            marker: None,
            priority: None,
            content: format!("🤖 Agent run: {}", rec.agent_type),
            properties: props,
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: rec.completed_at,
            cancelled_at: None,
            collapsed: false,
            created_at: now,
            updated_at: now,
        }
    }

    /// Spawn a detached task that forces a `Failed`
    /// transition if the run is still `Running` after the
    /// timeout elapses.
    fn spawn_timeout_watcher(&self, id: Uuid) {
        let lifecycle = self.clone();
        let timeout = self.timeout;
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            // Check current state — only fail if still Running.
            if let Some(dto) = lifecycle.get(id) {
                if dto.status == "Running" {
                    let _ = lifecycle
                        .fail(
                            id,
                            format!("Agent run exceeded {}-second timeout", timeout.as_secs()),
                        )
                        .await;
                }
            }
        });
    }
}

#[cfg(test)]
mod tests_disabled {
    // Tests moved to `tests/agent_room_integration.rs` to
    // bypass pre-existing inline test compile errors in
    // sibling modules. The integration suite covers the
    // same scenarios; see the test file for the
    // authoritative list.
}
