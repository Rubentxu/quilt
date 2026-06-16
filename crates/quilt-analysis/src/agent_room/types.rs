//! DTOs and the `AgentStatus` enum for the Agent Room surface.
//!
//! Wire format: serde camelCase, RFC 3339 for timestamps. The
//! string set of `AgentStatus` is intentionally identical to
//! the one `AgentRunRenderer` already renders (see
//! `quilt-ui/src/features/outliner-tiptap/rendering/AgentRunRenderer.tsx`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Lifecycle status of an agent run.
///
/// The set is closed — these are the only valid strings. The
/// `as_str()` mapping is the single source of truth on the
/// wire; any change here MUST be reflected in the
/// `AgentRunRenderer` color tokens and the TypeScript
/// `AgentStatus` union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AgentStatus {
    /// Wire-facing string. Matches the values `AgentRunRenderer`
    /// already understands.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Queued => "Queued",
            AgentStatus::Running => "Running",
            AgentStatus::Completed => "Completed",
            AgentStatus::Failed => "Failed",
            AgentStatus::Cancelled => "Cancelled",
        }
    }

    /// Parse a wire-facing string into the enum. Returns `None`
    /// for unknown values so the caller can decide how to react
    /// (typically: ignore the row rather than crash).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Queued" => Some(AgentStatus::Queued),
            "Running" => Some(AgentStatus::Running),
            "Completed" => Some(AgentStatus::Completed),
            "Failed" => Some(AgentStatus::Failed),
            "Cancelled" => Some(AgentStatus::Cancelled),
            _ => None,
        }
    }
}

/// Per-agent DTO returned by every endpoint in the Agent Room
/// surface. camelCase on the wire (Rust field name → camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDto {
    /// UUID of the underlying AgentRun block.
    pub id: String,
    /// Agent type id, e.g. `"decay-annotator"` in V1.
    pub agent_type: String,
    /// Informational model label (no LLM in V1, per ADR-0001).
    pub model: Option<String>,
    /// Current lifecycle state.
    pub status: String,
    /// Optional context page the agent was scoped to.
    pub context_page: Option<String>,
    /// One-line summary set when the agent reaches a terminal
    /// `Completed` state.
    pub summary: Option<String>,
    /// Number of blocks this agent has written to the graph.
    pub blocks_modified: u32,
    /// When the worker started. `None` while `Queued`.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run reached a terminal state. `None` while
    /// `Queued` or `Running`.
    pub completed_at: Option<DateTime<Utc>>,
    /// Error message, populated only when `status === "Failed"`.
    pub error: Option<String>,
}

/// `GET /api/v1/agents` response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentListResponse {
    pub agents: Vec<AgentDto>,
    /// Full registry size regardless of the `?limit=` filter.
    pub total: usize,
}

/// `POST /api/v1/agents` request body. Optional fields are
/// accepted but ignored for the parts V1 does not implement
/// (e.g. `queue_mode` is parsed but not honoured — see
/// `tasks.md` for the rationale).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpawnAgentRequest {
    pub agent_type: String,
    #[serde(default)]
    pub context_page: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// Accepted for forward compatibility; ignored in V1.
    #[serde(default)]
    pub queue_mode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_status_as_str_matches_existing_strings() {
        // These exact strings are what AgentRunRenderer already
        // renders. Changing any of them is a wire-format break
        // and a renderer break.
        assert_eq!(AgentStatus::Queued.as_str(), "Queued");
        assert_eq!(AgentStatus::Running.as_str(), "Running");
        assert_eq!(AgentStatus::Completed.as_str(), "Completed");
        assert_eq!(AgentStatus::Failed.as_str(), "Failed");
        assert_eq!(AgentStatus::Cancelled.as_str(), "Cancelled");
    }

    #[test]
    fn agent_status_parse_round_trip() {
        for s in [
            AgentStatus::Queued,
            AgentStatus::Running,
            AgentStatus::Completed,
            AgentStatus::Failed,
            AgentStatus::Cancelled,
        ] {
            assert_eq!(AgentStatus::parse(s.as_str()), Some(s));
        }
        assert_eq!(AgentStatus::parse("Unknown"), None);
        assert_eq!(AgentStatus::parse("queued"), None); // case-sensitive
    }

    #[test]
    fn agent_dto_serialization_is_camel_case() {
        let dto = AgentDto {
            id: "agent-1".to_string(),
            agent_type: "decay-annotator".to_string(),
            model: Some("v1".to_string()),
            status: "Running".to_string(),
            context_page: Some("p/x".to_string()),
            summary: None,
            blocks_modified: 0,
            started_at: None,
            completed_at: None,
            error: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        // Spot-check the camelCase keys.
        assert!(json.contains("\"agentType\""), "got: {json}");
        assert!(json.contains("\"contextPage\""), "got: {json}");
        assert!(json.contains("\"blocksModified\""), "got: {json}");
        assert!(json.contains("\"startedAt\""), "got: {json}");
        assert!(json.contains("\"completedAt\""), "got: {json}");
    }

    #[test]
    fn spawn_request_default_omits_optional_fields() {
        let req = SpawnAgentRequest {
            agent_type: "decay-annotator".to_string(),
            context_page: None,
            model: None,
            queue_mode: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert_eq!(json, "{\"agentType\":\"decay-annotator\"}");
    }

    #[test]
    fn spawn_request_deserialises_partial_body() {
        // The handler should accept `{"agentType": "..."}` only.
        let json = r#"{"agentType": "decay-annotator"}"#;
        let req: SpawnAgentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.agent_type, "decay-annotator");
        assert!(req.context_page.is_none());
        assert!(req.model.is_none());
        assert!(req.queue_mode.is_none());
    }
}
