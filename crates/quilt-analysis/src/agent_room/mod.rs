//! Agent Room — orchestrating AI agent runs on the knowledge graph.
//!
//! V1 scope (CG-5):
//! - ONE agent type registered: `decay-annotator`.
//! - Sequential queue (one Running at a time).
//! - Polling-based UI consumption (no WebSocket/SSE).
//! - Cancel is the only intervention primitive.
//!
//! The state of every run is persisted as a `type:: agent-run`
//! block in the graph (per ADR-0015), so the same
//! `AgentRunRenderer` already used by `AgentActivityFeed`
//! renders them without changes.
//!
//! The module exposes:
//! - [`types`] — DTOs, `AgentStatus` enum, request/response shapes.
//! - [`lifecycle`] — `AgentLifecycle` (state machine + persistence).
//! - [`queue`] — sequential Tokio worker that drives runs.
//! - [`registry`] — `AgentExecutor` trait + `AgentRegistry` of types.
//! - [`agents`] — concrete executors (`decay-annotator` for V1).

pub mod agents;
pub mod lifecycle;
pub mod queue;
pub mod registry;
pub mod types;

pub use lifecycle::{AgentError, AgentLifecycle, AgentListFilter, AgentRunRecord};
pub use queue::AgentQueue;
pub use registry::{AgentExecutor, AgentRegistry, AgentRunOutcome, RunContext};
pub use types::{AgentDto, AgentListResponse, AgentStatus, SpawnAgentRequest};

// Re-export the lifecycle's `AgentError` as the canonical
// one (the registry's `AgentError` is the same name but
// scoped to executor failures; aliasing them at the module
// root keeps the public API single-named).
pub use crate::agent_room::lifecycle::AgentError as LifecycleError;
