//! Integration tests for MCP GraphResourceProvider.
//!
//! Uses mock ResourceUseCases to test: graph snapshot, pages,
//! journals, tags, and unknown resources.

use std::sync::Arc;

use async_trait::async_trait;
use quilt_application::ApplicationError;
use quilt_application::use_cases::{
    GraphSnapshot, JournalSummary, PageSummary, ResourceUseCases, TagSummary,
};
use quilt_mcp::handlers::ResourceProvider;
use quilt_mcp::handlers::resource::GraphResourceProvider;

// ── Simple Mock (no Mutex — uses fixed data) ────────────────

struct MockResourceUseCases {
    pages_count: usize,
    journals_count: usize,
    blocks_count: usize,
}

#[async_trait]
impl ResourceUseCases for MockResourceUseCases {
    async fn graph_snapshot(&self) -> Result<GraphSnapshot, ApplicationError> {
        Ok(GraphSnapshot {
            pages_count: self.pages_count,
            journals_count: self.journals_count,
            blocks_count: self.blocks_count,
            recent_pages: vec![],
        })
    }

    async fn list_pages(&self) -> Result<Vec<PageSummary>, ApplicationError> {
        Ok(vec![PageSummary {
            id: "p1".into(),
            name: "home".into(),
            title: Some("Home".into()),
            is_journal: false,
        }])
    }

    async fn list_journals(&self) -> Result<Vec<JournalSummary>, ApplicationError> {
        Ok(vec![])
    }

    async fn list_tags(&self) -> Result<Vec<TagSummary>, ApplicationError> {
        Ok(vec![])
    }
}

// ── Helpers ──────────────────────────────────────────────────

fn provider() -> GraphResourceProvider {
    let mock = Arc::new(MockResourceUseCases {
        pages_count: 0,
        journals_count: 0,
        blocks_count: 0,
    });
    GraphResourceProvider::new(mock)
}

fn provider_with_data() -> GraphResourceProvider {
    let mock = Arc::new(MockResourceUseCases {
        pages_count: 5,
        journals_count: 2,
        blocks_count: 42,
    });
    GraphResourceProvider::new(mock)
}

// ── quilt://graph ───────────────────────────────────────────

#[tokio::test]
async fn test_graph_resource_empty() {
    let p = provider();
    let result = p.read("quilt://graph").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["pages"], 0);
    assert_eq!(parsed["journals"], 0);
    assert_eq!(parsed["blocks"], 0);
    assert!(parsed["last_updated"].is_string());
}

#[tokio::test]
async fn test_graph_resource_with_data() {
    let p = provider_with_data();
    let result = p.read("quilt://graph").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["pages"], 5);
    assert_eq!(parsed["journals"], 2);
    assert_eq!(parsed["blocks"], 42);
}

// ── quilt://pages ───────────────────────────────────────────

#[tokio::test]
async fn test_pages_resource() {
    let p = provider();
    let result = p.read("quilt://pages").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed.is_array());
    assert_eq!(parsed[0]["name"], "home");
    assert_eq!(parsed[0]["title"], "Home");
}

// ── quilt://journals ────────────────────────────────────────

#[tokio::test]
async fn test_journals_resource() {
    let p = provider();
    let result = p.read("quilt://journals").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
}

// ── quilt://tags ────────────────────────────────────────────

#[tokio::test]
async fn test_tags_resource() {
    let p = provider();
    let result = p.read("quilt://tags").await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
}

// ── Unknown resource ────────────────────────────────────────

#[tokio::test]
async fn test_unknown_resource() {
    let p = provider();
    let result = p.read("quilt://nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown resource"));
}

// ── Resource listing ────────────────────────────────────────

#[test]
fn test_resources_list() {
    let p = provider();
    let resources = p.resources();
    let uris: Vec<&str> = resources.iter().map(|r| r.uri.as_str()).collect();
    assert!(uris.contains(&"quilt://graph"));
    assert!(uris.contains(&"quilt://pages"));
    assert!(uris.contains(&"quilt://journals"));
    assert!(uris.contains(&"quilt://tags"));
}
