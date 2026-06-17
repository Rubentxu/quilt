//! Migration tool handler
//!
//! Owns: quilt_scan_directory, quilt_ingest_markdown, quilt_reindex

use crate::handlers::ToolHandler;
use crate::protocol::Evidence;
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
                name: "quilt_scan_directory".to_string(),
                description: "Scan a directory for markdown files and return an ingestion plan"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path to directory to scan"
                        },
                        "recursive": {
                            "type": "boolean",
                            "description": "Scan recursively (default: true)",
                            "default": true
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "quilt_ingest_markdown".to_string(),
                description: "Ingest new pages from an ingestion plan (quilt_scan_directory)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "plan": {
                            "type": "object",
                            "description": "Ingestion plan from quilt_scan_directory",
                        }
                    },
                    "required": ["plan"]
                }),
            },
            Tool {
                name: "quilt_reindex".to_string(),
                description: "Re-index all modified files (detect changes by mtime and re-import)"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute path to directory to reindex"
                        }
                    },
                    "required": ["path"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_scan_directory" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path'")?;
                let recursive = args
                    .get("recursive")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let depth = if recursive { u32::MAX } else { 1 };

                let plan = self
                    .migration_use_cases
                    .scan(std::path::Path::new(path), depth)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&plan).unwrap_or_else(|e| e.to_string()))
            }

            "quilt_ingest_markdown" => {
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

            "quilt_reindex" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path'")?;

                let plan = self
                    .migration_use_cases
                    .scan(std::path::Path::new(path), u32::MAX)
                    .await
                    .map_err(|e| e.to_string())?;

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
