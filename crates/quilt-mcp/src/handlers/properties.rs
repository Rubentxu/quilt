//! MCP tool handler for property intelligence operations.
//!
//! Owns: `quilt_properties_batch`

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use async_trait::async_trait;
use quilt_application::property::PropertyServiceTrait;
use quilt_domain::properties::analytics::AnalyticsParams;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Property tool handler — wraps `PropertyServiceTrait`.
pub struct PropertyToolHandler {
    service: Arc<dyn PropertyServiceTrait>,
}

impl PropertyToolHandler {
    pub fn new(service: Arc<dyn PropertyServiceTrait>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl ToolHandler for PropertyToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_properties_batch".to_string(),
                description:
                    "Batch query property definitions: get by keys, search by name, or list by usage"
                        .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "keys": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Property keys to fetch"
                        },
                        "query": {
                            "type": "string",
                            "description": "Substring search (matches key or title)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results",
                            "default": 50
                        }
                    }
                }),
            },
            Tool {
                name: "quilt_properties_suggest".to_string(),
                description: "Suggest properties matching partial input (autocomplete for discovery)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "partial": {
                            "type": "string",
                            "description": "Partial property name to match"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max suggestions",
                            "default": 10
                        }
                    },
                    "required": ["partial"]
                }),
            },
            Tool {
                name: "quilt_properties_analytics".to_string(),
                description: "Get property analytics: co-occurrence (PMI), usage trends, aggregate stats".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "co_occurrence_limit": {
                            "type": "integer",
                            "description": "Max co-occurrence pairs",
                            "default": 20
                        },
                        "trend_limit": {
                            "type": "integer",
                            "description": "Max trending properties",
                            "default": 20
                        },
                        "trend_period_days": {
                            "type": "integer",
                            "description": "Period in days for trends",
                            "default": 30
                        }
                    }
                }),
            },
            Tool {
                name: "quilt_properties_lifecycle".to_string(),
                description: "Manage property lifecycle: deprecate, merge, or create alias".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["deprecate", "merge", "alias"],
                            "description": "Lifecycle action to perform"
                        },
                        "key": {
                            "type": "string",
                            "description": "Property key (for deprecate) or source key (for merge)"
                        },
                        "target_key": {
                            "type": "string",
                            "description": "Target property key (required for merge and alias)"
                        },
                        "new_key": {
                            "type": "string",
                            "description": "New alias key (required for alias action)"
                        }
                    },
                    "required": ["action", "key"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_properties_batch" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                let limit = limit.min(100);

                let keys: Vec<String> = args
                    .get("keys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let query_str = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                let mut results = Vec::new();

                if !keys.is_empty() {
                    let by_keys = self
                        .service
                        .batch_get(&keys)
                        .await
                        .map_err(|e| e.to_string())?;
                    results.extend(by_keys);
                }

                if !query_str.is_empty() {
                    let searched = self
                        .service
                        .search(query_str, limit)
                        .await
                        .map_err(|e| e.to_string())?;
                    let existing_keys: std::collections::HashSet<_> =
                        results.iter().map(|d| d.db_ident.clone()).collect();
                    for def in searched {
                        if !existing_keys.contains(&def.db_ident) {
                            results.push(def);
                        }
                    }
                }

                if keys.is_empty() && query_str.is_empty() {
                    results = self
                        .service
                        .list_by_usage(limit)
                        .await
                        .map_err(|e| e.to_string())?;
                }

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": results.len(),
                    "definitions": results,
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            "quilt_properties_suggest" => {
                let partial = args
                    .get("partial")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'partial' parameter")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
                let limit = limit.min(50);

                let suggestions = self
                    .service
                    .suggest(partial, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": suggestions.len(),
                    "suggestions": suggestions,
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            "quilt_properties_analytics" => {
                let params = AnalyticsParams {
                    co_occurrence_limit: args
                        .get("co_occurrence_limit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(20) as usize,
                    trend_limit: args
                        .get("trend_limit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(20) as usize,
                    trend_period_days: args
                        .get("trend_period_days")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(30) as u32,
                };

                let result = self
                    .service
                    .analytics(&params)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            "quilt_properties_lifecycle" => {
                let action = args
                    .get("action")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'action' parameter")?;
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'key' parameter")?;

                let result = match action {
                    "deprecate" => {
                        let def = self
                            .service
                            .deprecate(key)
                            .await
                            .map_err(|e| e.to_string())?;
                        serde_json::to_value(def).map_err(|e| e.to_string())?
                    }
                    "merge" => {
                        let target = args
                            .get("target_key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'target_key' for merge")?;
                        let def = self
                            .service
                            .merge(key, target)
                            .await
                            .map_err(|e| e.to_string())?;
                        serde_json::to_value(def).map_err(|e| e.to_string())?
                    }
                    "alias" => {
                        let new_key = args
                            .get("new_key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'new_key' for alias")?;
                        let target = args
                            .get("target_key")
                            .and_then(|v| v.as_str())
                            .ok_or("Missing 'target_key' for alias")?;
                        let def = self
                            .service
                            .alias(new_key, target)
                            .await
                            .map_err(|e| e.to_string())?;
                        serde_json::to_value(def).map_err(|e| e.to_string())?
                    }
                    _ => return Err(format!("Unknown lifecycle action: {}", action)),
                };

                Ok(serde_json::to_string_pretty(&result)
                    .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        if name == "quilt_properties_batch" {
            if let Some(defs) = result.get("definitions").and_then(|v| v.as_array()) {
                for def in defs {
                    if let Some(key) = def.get("db_ident").and_then(|v| v.as_str()) {
                        ev.matched_terms.push(key.to_string());
                    }
                }
            }
            Some(ev)
        } else {
            None
        }
    }
}
