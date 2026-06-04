//! MCP Protocol types
//!
//! Extracts all MCP request/response types from the original server.rs.

use crate::resources::Resource;
use crate::tools::Tool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Request types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize { params: InitializeParams },
    #[serde(rename = "tools/list")]
    ListTools,
    #[serde(rename = "tools/call")]
    CallTool { params: CallToolParams },
    #[serde(rename = "resources/list")]
    ListResources,
    #[serde(rename = "resources/read")]
    ReadResource { params: ReadResourceParams },
    #[serde(rename = "notifications_enabled")]
    EnableNotifications,
}

#[derive(Debug, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
}

#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<Roots>,
    pub sampling: Option<Sampling>,
}

#[derive(Debug, Deserialize)]
pub struct Roots {
    pub list: bool,
}

#[derive(Debug, Deserialize)]
pub struct Sampling {}

#[derive(Debug, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

// ── Response types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(tag = "method")]
pub enum McpResponse {
    #[serde(rename = "initialize")]
    Initialize(InitializeResult),
    #[serde(rename = "tools/list")]
    ToolsList(ToolsListResult),
    #[serde(rename = "tools/call")]
    ToolsCall(ToolsCallResult),
    #[serde(rename = "resources/list")]
    ResourcesList(ResourcesListResult),
    #[serde(rename = "resources/read")]
    ResourcesRead(ResourceReadResult),
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolCapabilities,
    pub resources: ResourceCapabilities,
    pub notifications: NotificationCapabilities,
}

#[derive(Debug, Serialize)]
pub struct ToolCapabilities {
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ResourceCapabilities {
    pub subscribe: bool,
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct NotificationCapabilities {}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Serialize)]
pub struct ToolsCallResult {
    pub content: Vec<ContentBlock>,
    pub is_error: Option<bool>,
    /// Server-injected evidence envelope (Evidence Contract v1).
    /// Always `None` when the handler does not provide evidence, so
    /// pre-change wire format is preserved (skip_serializing_if).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<MetaEnvelope>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: Resource },
}

#[derive(Debug, Serialize)]
pub struct ResourcesListResult {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Serialize)]
pub struct ResourceReadResult {
    pub contents: Vec<ResourceContent>,
    /// Server-injected evidence envelope (Evidence Contract v1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<MetaEnvelope>,
}

#[derive(Debug, Serialize)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
}

// ── Evidence Contract v1 ────────────────────────────────────────────────
//
// Server-level wrapper for tool/resource responses. Injected AFTER
// handler execution by `server.rs::handle_call_tool` /
// `handle_read_resource`. Handlers expose `tool_evidence()` /
// `resource_evidence()` pure functions to derive per-handler evidence
// (block IDs, query AST, page metadata). When the handler returns
// None, the server uses `Evidence::universal_fallback()`. On error,
// the server uses `Evidence::error_fallback()`.

/// Source authority of a block — used by G2 source ranking.
///
/// Ordering: `Manual > PropertyTyped > AutoExtracted`. Higher authority
/// wins as a secondary sort within the same relevance score band.
/// Variants are declared in ascending authority order so the
/// auto-derived `Ord` gives Manual the highest discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceAuthority {
    /// Block created by an agent or via auto-extraction.
    AutoExtracted,
    /// Block carries ≥1 non-string typed property (ADR-0003).
    PropertyTyped,
    /// Human-authored block (`user::<name>` convention, ADR-0003).
    Manual,
}

impl Default for SourceAuthority {
    fn default() -> Self {
        SourceAuthority::AutoExtracted
    }
}

/// Indicates whether an agent should create a duplicate resource.
/// Part of the Evidence contract (Q018) — prevents agents from
/// blindly creating duplicate pages/blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreateSafety {
    /// The resource already exists — do NOT create
    Exists,
    /// The resource probably exists — verify before creating
    Probable,
    /// Unknown whether the resource exists — safe to create
    Unknown,
}

/// Provenance metadata attached to every MCP tool/resource response.
///
/// `is_error: true` indicates the handler returned an error; in that
/// case the rest of the fields are at their defaults. `page_updated_at`
/// is `Some` only when the tool references a single page (e.g.,
/// `quilt_get_page_blocks`). `block_content` is NEVER included — the
/// hybrid payload carries IDs + page metadata, not body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Name of the tool that produced this response.
    pub tool_name: String,
    /// Server timestamp at injection time.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// True when the handler returned an error envelope.
    #[serde(default)]
    pub is_error: bool,
    /// Block IDs touched/produced by this tool call.
    #[serde(default)]
    pub block_ids: Vec<Uuid>,
    /// Page name when the tool references a single page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_name: Option<String>,
    /// Page `updated_at` when the tool references a single page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// DSL query AST for `quilt_query` (string form).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_ast: Option<String>,
    /// Matched search terms for `quilt_search`.
    #[serde(default)]
    pub matched_terms: Vec<String>,
    /// Source authority ranking (G2). None when not derivable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_authority: Option<SourceAuthority>,
    /// Create safety hint (Q018). None when not applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_safety: Option<CreateSafety>,
}

impl Default for Evidence {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            timestamp: chrono::Utc::now(),
            is_error: false,
            block_ids: Vec::new(),
            page_name: None,
            page_updated_at: None,
            query_ast: None,
            matched_terms: Vec::new(),
            source_authority: None,
            create_safety: None,
        }
    }
}

impl Evidence {
    /// Universal fallback: `{tool_name, timestamp}` only. Used when the
    /// handler does not override `tool_evidence()`.
    pub fn universal_fallback(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            ..Self::default()
        }
    }

    /// Error fallback: `{tool_name, timestamp, is_error: true}`.
    /// Agents need to know which tool failed and when, so we ALWAYS
    /// inject this on errors.
    pub fn error_fallback(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            is_error: true,
            ..Self::default()
        }
    }
}

/// `_meta` envelope carried by `ToolsCallResult` and `ResourceReadResult`.
///
/// Reserved at server level — no handler may serialize a top-level
/// `evidence` key from its returned string (see
/// `tests/evidence_contract_tests.rs`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetaEnvelope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── T-02: SourceAuthority ─────────────────────────────────────

    #[test]
    fn source_authority_ordering_manual_wins() {
        // Manual > PropertyTyped > AutoExtracted
        assert!(SourceAuthority::Manual > SourceAuthority::PropertyTyped);
        assert!(SourceAuthority::PropertyTyped > SourceAuthority::AutoExtracted);
        assert!(SourceAuthority::Manual > SourceAuthority::AutoExtracted);
    }

    #[test]
    fn source_authority_serde_roundtrip() {
        for v in [
            SourceAuthority::Manual,
            SourceAuthority::PropertyTyped,
            SourceAuthority::AutoExtracted,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: SourceAuthority = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    // ── T-03: Evidence struct ────────────────────────────────────

    #[test]
    fn evidence_omits_none_options() {
        // No page_name / page_updated_at / query_ast / source_authority / create_safety
        // → those keys must NOT appear in the JSON output.
        let e = Evidence::universal_fallback("quilt_x");
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert!(v.get("page_name").is_none());
        assert!(v.get("page_updated_at").is_none());
        assert!(v.get("query_ast").is_none());
        assert!(v.get("source_authority").is_none());
        assert!(v.get("create_safety").is_none());
        assert_eq!(v["tool_name"], "quilt_x");
        assert_eq!(v["is_error"], false);
        assert!(v["block_ids"].as_array().unwrap().is_empty());
        assert!(v["matched_terms"].as_array().unwrap().is_empty());
        assert!(v["timestamp"].is_string());
    }

    #[test]
    fn evidence_includes_some_options() {
        let now = chrono::Utc::now();
        let e = Evidence {
            tool_name: "t".into(),
            timestamp: now,
            is_error: false,
            block_ids: vec![Uuid::new_v4()],
            page_name: Some("P".into()),
            page_updated_at: Some(now),
            query_ast: Some("AST".into()),
            matched_terms: vec!["foo".into()],
            source_authority: Some(SourceAuthority::Manual),
            create_safety: Some(CreateSafety::Exists),
        };
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["page_name"], "P");
        assert!(v["page_updated_at"].is_string());
        assert_eq!(v["query_ast"], "AST");
        assert_eq!(v["source_authority"], "Manual");
        assert_eq!(v["matched_terms"][0], "foo");
        assert_eq!(v["create_safety"], "Exists");
    }

    #[test]
    fn evidence_error_fallback_is_error_true() {
        let e = Evidence::error_fallback("quilt_bad");
        assert!(e.is_error);
        assert_eq!(e.tool_name, "quilt_bad");
        assert!(e.block_ids.is_empty());
        assert!(e.page_name.is_none());
    }

    // ── T-04: MetaEnvelope ───────────────────────────────────────

    #[test]
    fn meta_envelope_default_is_empty() {
        let m = MetaEnvelope::default();
        let v: serde_json::Value = serde_json::to_value(&m).unwrap();
        // `evidence` is None → key must be omitted.
        assert!(v.get("evidence").is_none(), "got: {}", v);
    }

    // ── T-05: _meta field on response types ──────────────────────

    #[test]
    fn tools_call_result_without_meta_is_byte_identical_to_prechange() {
        // Pre-change fixture: no _meta field. With `_meta: None` and
        // `skip_serializing_if = "Option::is_none"`, the JSON must be
        // identical to the pre-change wire format.
        let r = ToolsCallResult {
            content: vec![ContentBlock::Text { text: "ok".into() }],
            is_error: Some(false),
            _meta: None,
        };
        let v: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert!(v.get("_meta").is_none(), "got: {}", v);
    }

    #[test]
    fn resource_read_result_without_meta_is_byte_identical_to_prechange() {
        let r = ResourceReadResult {
            contents: vec![ResourceContent {
                uri: "quilt://x".into(),
                mime_type: "application/json".into(),
                text: Some("ok".into()),
            }],
            _meta: None,
        };
        let v: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert!(v.get("_meta").is_none(), "got: {}", v);
    }
}
