//! MCP Handlers module
//!
//! Contains presentation-layer traits and implementations for MCP tools and resources.
//! Each handler owns a domain of tools/resources and delegates to application use cases.

use crate::protocol::Evidence;
use crate::resources::Resource;
use crate::tools::Tool;
use async_trait::async_trait;

/// MCP tool handler trait — each handler owns a domain of tools.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    /// Return tool definitions this handler owns.
    fn tools(&self) -> Vec<Tool>;
    /// Execute a tool by name.
    async fn execute(&self, name: &str, args: &serde_json::Value) -> Result<String, String>;

    /// Derive evidence for a tool call from its name, args, and result.
    ///
    /// This is a **pure function** (Option B per design): it takes
    /// inputs and returns output, with no internal state. The server
    /// layer calls this after `execute()` succeeds; if the handler
    /// returns `None`, the server injects `Evidence::universal_fallback`.
    ///
    /// Handlers MUST NOT store state between `execute()` and
    /// `tool_evidence()` — derive everything from `(name, args, result)`.
    fn tool_evidence(
        &self,
        _name: &str,
        _args: &serde_json::Value,
        _result: &serde_json::Value,
    ) -> Option<Evidence> {
        None
    }
}

/// MCP resource provider trait — each provider owns a domain of resources.
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    /// Return resource definitions this provider owns.
    fn resources(&self) -> Vec<Resource>;
    /// Read a resource by URI.
    async fn read(&self, uri: &str) -> Result<String, String>;

    /// Derive evidence for a resource read from its URI and result.
    /// Default: `None` → server injects `Evidence::universal_fallback`.
    fn resource_evidence(&self, _uri: &str, _result: &serde_json::Value) -> Option<Evidence> {
        None
    }
}

pub mod block;
pub mod graph;
pub mod page;
pub mod properties;
pub mod query;
pub mod resource;
pub mod retrieval;
pub mod schemas;
pub mod system;
pub mod template;
pub mod temporal;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// Test handler that does not override `tool_evidence` — exercises
    /// the default `None` branch.
    struct DefaultToolHandler;

    #[async_trait]
    impl ToolHandler for DefaultToolHandler {
        fn tools(&self) -> Vec<Tool> {
            vec![]
        }
        async fn execute(&self, _name: &str, _args: &Value) -> Result<String, String> {
            Ok("{}".into())
        }
    }

    struct DefaultResourceProvider;

    #[async_trait]
    impl ResourceProvider for DefaultResourceProvider {
        fn resources(&self) -> Vec<Resource> {
            vec![]
        }
        async fn read(&self, _uri: &str) -> Result<String, String> {
            Ok("{}".into())
        }
    }

    // T-06: default tool_evidence returns None.
    #[test]
    fn default_tool_evidence_is_none() {
        let h = DefaultToolHandler;
        let args = serde_json::json!({});
        let result = serde_json::json!({"ok": true});
        let ev = h.tool_evidence("quilt_x", &args, &result);
        assert!(ev.is_none(), "default tool_evidence should be None");
    }

    // T-06: default resource_evidence returns None.
    #[test]
    fn default_resource_evidence_is_none() {
        let p = DefaultResourceProvider;
        let result = serde_json::json!({});
        let ev = p.resource_evidence("quilt://x", &result);
        assert!(ev.is_none(), "default resource_evidence should be None");
    }
}
