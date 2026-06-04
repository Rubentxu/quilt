//! Temporal tool handler (G3 — Temporal classification).
//!
//! Owns: `quilt_query_temporal`.
//!
//! Compiles a temporal DSL query and returns the SQL plan + estimated cost
//! WITHOUT executing it. Agents use this to inspect what SQL would run before
//! committing to a full execute.

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
use crate::tools::Tool;
use crate::use_cases::SearchUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Temporal tool handler — temporal query planning.
pub struct TemporalToolHandler {
    search_use_cases: Arc<dyn SearchUseCases>,
}

impl TemporalToolHandler {
    pub fn new(search_use_cases: Arc<dyn SearchUseCases>) -> Self {
        Self { search_use_cases }
    }
}

#[async_trait]
impl ToolHandler for TemporalToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "quilt_query_temporal".to_string(),
            description: concat!(
                "Compile a temporal DSL query and return the SQL plan + estimated cost ",
                "WITHOUT executing it. Use this to inspect what SQL would run before ",
                "committing to a full execute. Example: `(temporal :this-week (page \"x\"))`."
            )
            .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dsl": {
                        "type": "string",
                        "description": "Temporal DSL query, e.g. '(temporal :this-week (page \"x\"))'."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results.",
                        "default": 100
                    }
                },
                "required": ["dsl"]
            }),
        }]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_query_temporal" => {
                let dsl = args
                    .get("dsl")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'dsl' parameter")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

                // Compile to SQL plan without executing
                let plan = self
                    .search_use_cases
                    .query(dsl, limit)
                    .await
                    .map_err(|e| e.to_string())?;

                // G3: Return PLAN — SQL preview + cost estimate, NOT rows.
                // The agent gets the compiled SQL to inspect before running it.
                let estimated_cost = if plan.sql.to_lowercase().contains("join") {
                    "high"
                } else if plan.sql.to_lowercase().contains("fts") {
                    "medium"
                } else {
                    "low"
                };

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "plan": {
                        "ast": plan.ast,
                        "sql": plan.sql,
                        "params": plan.params,
                        "estimated_cost": estimated_cost,
                        "note": "Plan only — not executed. Use quilt_query for full execution."
                    }
                }))
                .unwrap_or_else(|e| format!("Serialization error: {}", e)))
            }
            _ => Err(format!("Unknown tool: {}", name)),
        }
    }

    // G3: evidence includes the DSL query AST from the plan.
    fn tool_evidence(&self, name: &str, _args: &Value, result: &Value) -> Option<Evidence> {
        let mut ev = Evidence::universal_fallback(name);
        match name {
            "quilt_query_temporal" => {
                if let Some(plan) = result.get("plan") {
                    if let Some(ast) = plan.get("ast") {
                        ev.query_ast = Some(ast.to_string());
                    }
                }
                Some(ev)
            }
            _ => None,
        }
    }
}
