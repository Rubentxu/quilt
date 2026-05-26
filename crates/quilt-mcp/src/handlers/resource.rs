//! Graph resource provider
//!
//! Owns: quilt://graph, quilt://pages, quilt://journals, quilt://tags

use crate::handlers::ResourceProvider;
use crate::resources::Resource;
use crate::use_cases::ResourceUseCases;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::instrument;

/// Graph resource provider.
pub struct GraphResourceProvider {
    resource_use_cases: Arc<dyn ResourceUseCases>,
}

impl GraphResourceProvider {
    pub fn new(resource_use_cases: Arc<dyn ResourceUseCases>) -> Self {
        Self { resource_use_cases }
    }
}

#[async_trait]
impl ResourceProvider for GraphResourceProvider {
    fn resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "quilt://graph".to_string(),
                name: "Current Graph".to_string(),
                description: "Full graph data with blocks, pages, and connections".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "quilt://pages".to_string(),
                name: "All Pages".to_string(),
                description: "List of all pages in the graph".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "quilt://journals".to_string(),
                name: "Journal Pages".to_string(),
                description: "List of all journal pages".to_string(),
                mime_type: "application/json".to_string(),
            },
            Resource {
                uri: "quilt://tags".to_string(),
                name: "All Tags".to_string(),
                description: "List of all tags with usage counts".to_string(),
                mime_type: "application/json".to_string(),
            },
        ]
    }

    #[instrument(skip(self))]
    async fn read(&self, uri: &str) -> Result<String, String> {
        match uri {
            "quilt://graph" => {
                let snapshot = self
                    .resource_use_cases
                    .graph_snapshot()
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "pages": snapshot.pages_count,
                    "journals": snapshot.journals_count,
                    "blocks": snapshot.blocks_count,
                    "last_updated": chrono::Utc::now().to_rfc3339(),
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            "quilt://pages" => {
                let pages = self
                    .resource_use_cases
                    .list_pages()
                    .await
                    .map_err(|e| e.to_string())?;

                let page_list: Vec<serde_json::Value> = pages
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id,
                            "name": p.name,
                            "title": p.title,
                            "journal": p.is_journal,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&page_list).unwrap_or_else(|e| e.to_string()))
            }

            "quilt://journals" => {
                let journals = self
                    .resource_use_cases
                    .list_journals()
                    .await
                    .map_err(|e| e.to_string())?;

                let journal_list: Vec<serde_json::Value> = journals
                    .iter()
                    .map(|j| {
                        serde_json::json!({
                            "id": j.id,
                            "name": j.name,
                            "journal_day": j.journal_day,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&journal_list).unwrap_or_else(|e| e.to_string()))
            }

            "quilt://tags" => {
                let tags = self
                    .resource_use_cases
                    .list_tags()
                    .await
                    .map_err(|e| e.to_string())?;

                let tag_list: Vec<serde_json::Value> = tags
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "count": t.usage_count,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&tag_list).unwrap_or_else(|e| e.to_string()))
            }

            _ => Err(format!("Unknown resource: {}", uri)),
        }
    }
}
