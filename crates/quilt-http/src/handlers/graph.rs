//! Graph HTTP handlers
//!
//! REST endpoints for graph operations:
//! - GET /api/graph - Get the knowledge graph data

use std::collections::HashSet;
use std::sync::Arc;

use axum::{extract::State, Json};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::instrument;

use crate::error::HttpError;
use crate::state::HttpState;
use quilt_domain::repositories::{PageReader, PageRepository, PageWriter};

/// A node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    #[serde(rename = "nodeType")]
    pub node_type: String,
    pub journal: bool,
    /// Cognitive type for overlay (Cluster, Frontier, Gap, Stable)
    #[serde(rename = "cognitiveType", skip_serializing_if = "Option::is_none")]
    pub cognitive_type: Option<String>,
}

/// An edge in the graph (link between nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
}

/// Graph data response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

/// Extract wiki-style links from block content
/// Matches [[Page Name]] patterns
fn extract_links(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
        .collect()
}

/// Get the knowledge graph data
///
/// Builds a graph from all pages (nodes) and wiki-links between them (edges).
/// Wiki-links are extracted from block content using the [[Page Name]] pattern.
#[instrument(skip(state))]
pub async fn resource_graph(
    State(state): State<Arc<HttpState>>,
) -> Result<Json<GraphDataDto>, HttpError> {
    let page_repo = quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository::new(state.pool.clone());

    // Get all pages
    let pages = page_repo
        .get_all()
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    // Build a map of page names to IDs (lowercase for case-insensitive matching)
    let page_name_to_id: std::collections::HashMap<String, String> = pages
        .iter()
        .map(|p| (p.name.to_lowercase(), p.id.to_string()))
        .collect();

    // Create nodes from pages
    let nodes: Vec<GraphNodeDto> = pages
        .iter()
        .map(|p| GraphNodeDto {
            id: p.id.to_string(),
            name: p.name.clone(),
            node_type: if p.journal {
                "journal".to_string()
            } else {
                "page".to_string()
            },
            journal: p.journal,
        })
        .collect();

    // Query all blocks with their content to extract links
    let blocks = sqlx::query("SELECT id, page_id, content FROM blocks WHERE deleted = 0")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| HttpError::DatabaseError(e.to_string()))?;

    // Create edges from wiki-links in blocks
    let mut edges: Vec<GraphEdgeDto> = Vec::new();
    let mut seen_edges: HashSet<(String, String)> = HashSet::new();

    for row in blocks.iter() {
        let page_id: String = row.get("page_id");
        let content: String = row.get("content");

        let links = extract_links(&content);
        for target_name in links {
            // Only add edge if target page exists
            if let Some(target_id) = page_name_to_id.get(&target_name) {
                let source_id = page_id.clone();
                let edge_key = (source_id.clone(), target_id.clone());

                // Avoid duplicate edges
                if !seen_edges.contains(&edge_key) {
                    seen_edges.insert(edge_key);
                    edges.push(GraphEdgeDto {
                        source: source_id,
                        target: target_id.clone(),
                    });
                }
            }
        }
    }

    let now = chrono::Utc::now();
    Ok(Json(GraphDataDto {
        nodes,
        edges,
        last_updated: now.to_rfc3339(),
    }))
}

/// Mount graph routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new().route("/api/graph", axum::routing::get(resource_graph))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_links_basic() {
        let content = "This is a link to [[Test Page]]";
        let links = extract_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "test page");
    }

    #[test]
    fn test_extract_links_multiple() {
        let content = "Links to [[Page One]] and [[Page Two]] and [[Another Page]]";
        let links = extract_links(content);

        assert_eq!(links.len(), 3);
        assert!(links.contains(&"page one".to_string()));
        assert!(links.contains(&"page two".to_string()));
        assert!(links.contains(&"another page".to_string()));
    }

    #[test]
    fn test_extract_links_none() {
        let content = "No links here";
        let links = extract_links(content);

        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_links_empty_brackets() {
        let content = "Empty [[]] brackets";
        let links = extract_links(content);

        // Empty brackets should produce empty string
        assert!(links.is_empty() || links.iter().any(|s| s.is_empty()));
    }

    #[test]
    fn test_extract_links_case_insensitive() {
        let content = "[[Test Page]]";
        let links = extract_links(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "test page"); // Lowercased
    }

    #[test]
    fn test_graph_node_dto_serialization() {
        let node = GraphNodeDto {
            id: "node-123".to_string(),
            name: "Test Node".to_string(),
            node_type: "page".to_string(),
            journal: false,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"id\":\"node-123\""));
        assert!(json.contains("\"name\":\"Test Node\""));
        assert!(json.contains("\"nodeType\":\"page\""));
        assert!(json.contains("\"journal\":false"));
    }

    #[test]
    fn test_graph_edge_dto_serialization() {
        let edge = GraphEdgeDto {
            source: "node-1".to_string(),
            target: "node-2".to_string(),
        };

        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains("\"source\":\"node-1\""));
        assert!(json.contains("\"target\":\"node-2\""));
    }

    #[test]
    fn test_graph_data_dto_serialization() {
        let dto = GraphDataDto {
            nodes: vec![
                GraphNodeDto {
                    id: "1".to_string(),
                    name: "Page 1".to_string(),
                    node_type: "page".to_string(),
                    journal: false,
                },
            ],
            edges: vec![GraphEdgeDto {
                source: "1".to_string(),
                target: "2".to_string(),
            }],
            last_updated: "2024-01-01T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
        assert!(json.contains("\"lastUpdated\""));
    }
}
