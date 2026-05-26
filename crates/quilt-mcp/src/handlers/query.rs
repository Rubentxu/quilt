//! Query tool handler
//!
//! Owns: quilt_query, quilt_search

use crate::handlers::ToolHandler;
use crate::tools::Tool;
use crate::use_cases::SearchUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Query tool handler.
pub struct QueryToolHandler {
    search_use_cases: Arc<dyn SearchUseCases>,
}

impl QueryToolHandler {
    pub fn new(search_use_cases: Arc<dyn SearchUseCases>) -> Self {
        Self { search_use_cases }
    }
}

#[async_trait]
impl ToolHandler for QueryToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_query".to_string(),
                description: "Execute a Quilt DSL query against the current graph".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "dsl": { "type": "string", "description": "DSL query string" },
                        "limit": { "type": "integer", "description": "Max results", "default": 100 }
                    },
                    "required": ["dsl"]
                }),
            },
            Tool {
                name: "quilt_search".to_string(),
                description: "Full-text search across all pages and blocks".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "limit": { "type": "integer", "description": "Max results", "default": 50 }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_query" => {
                let dsl = args
                    .get("dsl")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'dsl' parameter")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

                let plan = self
                    .search_use_cases
                    .query(dsl, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "ast": plan.ast,
                    "sql": plan.sql,
                    "params": plan.params,
                    "note": "Query parsed. AST and SQL generated. Block execution not yet wired — returns plan only."
                })).unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            "quilt_search" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'query' parameter")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;

                let results = self
                    .search_use_cases
                    .search(query, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                let json_results: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "block_id": r.block_id,
                            "page_name": r.page_name,
                            "snippet": r.snippet,
                            "score": r.score,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": results.len(),
                    "results": json_results,
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}
