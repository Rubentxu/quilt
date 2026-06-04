//! Retrieval tool handler (G5 — Named-thing retrieval).
//!
//! Owns: `quilt_query_retrieve`.
//!
//! Uses `SearchUseCases::resolve_by_name` for fuzzy page/block name resolution.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::SearchUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;
use uuid::Uuid;

/// Retrieval tool handler — fuzzy name resolution.
pub struct RetrievalToolHandler {
    search_use_cases: Arc<dyn SearchUseCases>,
}

impl RetrievalToolHandler {
    pub fn new(search_use_cases: Arc<dyn SearchUseCases>) -> Self {
        Self { search_use_cases }
    }
}

#[async_trait]
impl ToolHandler for RetrievalToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_query_retrieve".to_string(),
            description: concat!(
                "Resolve a name to matching pages and blocks via fuzzy matching. ",
                "Returns all entities whose name matches the given term, sorted by ",
                "relevance score. Use this to find pages/blocks when you only know ",
                "a partial name."
            )
            .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name or partial name to search for (fuzzy match)."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of results to return.",
                        "default": 10
                    }
                },
                "required": ["name"]
            }),
        }]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_query_retrieve" => {
                let name_arg = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'name' parameter")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

                let results = self
                    .search_use_cases
                    .resolve_by_name(name_arg, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                let json_results: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "kind": r.kind.to_string(),
                            "name": r.name,
                            "score": r.score,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": json_results.len(),
                    "results": json_results,
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // G5: evidence includes matched term and resolved block IDs.
    fn tool_evidence(&self, name: &str, args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_query_retrieve" => {
                if let Some(q) = args.get("name").and_then(|v| v.as_str()) {
                    ev.matched_terms.push(q.to_string());
                }
                if let Some(results) = result.get("results").and_then(|v| v.as_array()) {
                    for r in results {
                        if let Some(id) = r.get("id").and_then(|v| v.as_str()) {
                            if let Ok(uuid) = Uuid::parse_str(id) {
                                ev.block_ids.push(uuid);
                            }
                        }
                    }
                }
                Some(ev)
            }
            _ => None,
        }
    }
}
