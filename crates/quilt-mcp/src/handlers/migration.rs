//! Migration tool handler (GS-9)
//!
//! Owns: quilt_migration_scan, quilt_migration_ingest, quilt_migration_reindex
//!
//! All three tools enforce the two-step scan→confirm flow (INV-3):
//! 1. quilt_migration_scan — read-only, returns IngestionPlan
//! 2. quilt_migration_ingest — accepts plan, processes "new" candidates only
//! 3. quilt_migration_reindex — accepts plan, processes "modified" candidates only

use crate::handlers::ToolHandler;
use crate::tools::Tool;
use crate::use_cases::MigrationUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Migration tool handler (scan, ingest, reindex).
pub struct MigrationToolHandler {
    migration_use_cases: Arc<MigrationUseCases>,
}

impl MigrationToolHandler {
    pub fn new(migration_use_cases: Arc<MigrationUseCases>) -> Self {
        Self {
            migration_use_cases,
        }
    }
}

#[async_trait]
impl ToolHandler for MigrationToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_migration_scan".to_string(),
                description: "Scan the active graph directory for Markdown files and return an ingestion plan (read-only, no writes)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "depth": {
                            "type": "integer",
                            "description": "Maximum directory depth (default: 8, min: 1)",
                            "minimum": 1,
                            "default": 8
                        }
                    }
                }),
            },
            Tool {
                name: "quilt_migration_ingest".to_string(),
                description: "Ingest new Markdown files from an approved ingestion plan (two-step flow: scan first, then ingest)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "plan": {
                            "type": "object",
                            "description": "Ingestion plan from quilt_migration_scan — only candidates with status 'new' are processed",
                        }
                    },
                    "required": ["plan"]
                }),
            },
            Tool {
                name: "quilt_migration_reindex".to_string(),
                description: "Reindex modified Markdown files from an approved ingestion plan (two-step flow: scan first, then reindex). Uses optimistic CAS on source_mtime for concurrency safety."
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "plan": {
                            "type": "object",
                            "description": "Ingestion plan from quilt_migration_scan — only candidates with status 'modified' are processed",
                        }
                    },
                    "required": ["plan"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_migration_scan" => {
                let depth = args
                    .get("depth")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as u32;

                let plan = self
                    .migration_use_cases
                    .scan(std::path::Path::new("."), depth)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&plan).unwrap_or_else(|e| e.to_string()))
            }

            "quilt_migration_ingest" => {
                let plan = args.get("plan").ok_or("Missing 'plan'")?;

                let plan = serde_json::from_value::<quilt_application::migration::IngestionPlan>(
                    plan.clone(),
                )
                .map_err(|e| format!("Invalid plan: {}", e))?;

                let result = self
                    .migration_use_cases
                    .ingest(&plan)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()))
            }

            "quilt_migration_reindex" => {
                let plan = args.get("plan").ok_or("Missing 'plan'")?;

                let plan = serde_json::from_value::<quilt_application::migration::IngestionPlan>(
                    plan.clone(),
                )
                .map_err(|e| format!("Invalid plan: {}", e))?;

                let result = self
                    .migration_use_cases
                    .reindex(&plan)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}
