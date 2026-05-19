//! Graph-related Tauri commands
//!
//! # Deprecation Notice
//! These Tauri command handlers are legacy and kept for reference.
//! The primary interface is now the HTTP REST API in `quilt-http` crate.
//! See [`quilt_http::handlers`] for the new implementation.

use crate::state::AppState;
use quilt_domain::repositories::{PageReader, PageRepository, PageWriter};
use quilt_infrastructure::database::sqlite::repositories::SqlitePageRepository;
use serde::{Deserialize, Serialize};
use tauri::State;
use regex::Regex;
use sqlx::Row;

/// A node in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeDto {
    pub id: String,
    pub name: String,
    #[serde(rename = "nodeType")]
    pub node_type: String,
    pub journal: bool,
}

/// An edge in the graph (link between nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeDto {
    pub source: String,
    pub target: String,
}

/// Graph data returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDataDto {
    pub nodes: Vec<GraphNodeDto>,
    pub edges: Vec<GraphEdgeDto>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: String,
}

/// Create a page repository (helper)
pub fn create_page_repo(
    pool: &quilt_infrastructure::database::sqlite::connection::DbPool,
) -> SqlitePageRepository {
    SqlitePageRepository::new(pool.clone())
}

/// Extract wiki-style links from block content
/// Matches [[Page Name]] patterns
fn extract_links(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    re.captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_lowercase()))
        .collect()
}

/// Get the graph data for the knowledge graph view
#[tauri::command]
pub async fn resource_graph(state: State<'_, AppState>) -> Result<GraphDataDto, String> {
    let page_repo = create_page_repo(&state.pool);

    // Get all pages
    let pages = page_repo.get_all().await.map_err(|e| e.to_string())?;

    // Build a map of page names to IDs
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
            node_type: if p.journal { "journal".to_string() } else { "page".to_string() },
            journal: p.journal,
        })
        .collect();

    // Query all blocks with their content to extract links
    let blocks = sqlx::query("SELECT id, page_id, content FROM blocks WHERE deleted = 0")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| e.to_string())?;

    // Create edges from wiki-links in blocks
    let mut edges: Vec<GraphEdgeDto> = Vec::new();
    let mut seen_edges: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

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
    Ok(GraphDataDto {
        nodes,
        edges,
        last_updated: now.to_rfc3339(),
    })
}
