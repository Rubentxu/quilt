//! Page tool handler
//!
//! Owns: quilt_list_pages, quilt_get_page_blocks, quilt_get_journal

use crate::handlers::ToolHandler;
use crate::serialization::block_to_json;
use crate::tools::Tool;
use crate::use_cases::PageUseCases;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

/// Page tool handler.
pub struct PageToolHandler {
    page_use_cases: Arc<dyn PageUseCases>,
}

impl PageToolHandler {
    pub fn new(page_use_cases: Arc<dyn PageUseCases>) -> Self {
        Self { page_use_cases }
    }
}

#[async_trait]
impl ToolHandler for PageToolHandler {
    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "quilt_list_pages".to_string(),
                description: "List all pages in the graph".to_string(),
                input_schema: serde_json::json!({ "type": "object", "properties": {} }),
            },
            Tool {
                name: "quilt_get_page_blocks".to_string(),
                description: "Get all blocks on a page".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "page_name": { "type": "string", "description": "Page name" },
                        "format": { "type": "string", "description": "Format: markdown or org", "default": "markdown" }
                    },
                    "required": ["page_name"]
                }),
            },
            Tool {
                name: "quilt_get_journal".to_string(),
                description: "Get or create a journal page for a specific date".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "description": "Date in YYYY-MM-DD format" }
                    },
                    "required": ["date"]
                }),
            },
        ]
    }

    #[instrument(skip(self, args))]
    async fn execute(&self, name: &str, args: &Value) -> Result<String, String> {
        match name {
            "quilt_list_pages" => {
                let pages = self
                    .page_use_cases
                    .list()
                    .await
                    .map_err(|e| e.to_string())?;

                let page_list: Vec<serde_json::Value> = pages
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id.to_string(),
                            "name": p.name,
                            "title": p.title,
                            "journal": p.journal,
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "count": page_list.len(),
                    "pages": page_list,
                }))
                .unwrap_or_else(|e| e.to_string()))
            }

            "quilt_get_page_blocks" => {
                let page_name = args
                    .get("page_name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'page_name'")?;

                let page_with_blocks = self
                    .page_use_cases
                    .get_blocks(page_name)
                    .await
                    .map_err(|e| e.to_string())?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "page": { "id": page_with_blocks.page.id.to_string(), "name": page_with_blocks.page.name },
                    "blocks": page_with_blocks.blocks.iter().map(block_to_json).collect::<Vec<_>>(),
                    "count": page_with_blocks.blocks.len(),
                })).unwrap_or_else(|e| e.to_string()))
            }

            "quilt_get_journal" => {
                let date = args
                    .get("date")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'date'")?;

                let page = self
                    .page_use_cases
                    .get_or_create_journal(date)
                    .await
                    .map_err(|e| e.to_string())?;

                // Get blocks for this page by name
                let page_with_blocks = self
                    .page_use_cases
                    .get_blocks(&page.name)
                    .await
                    .map_err(|e| e.to_string())?;

                // Get journal_day value
                let journal_day = page.journal_day.map(|d| d.as_i32());

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "page": { "id": page.id.to_string(), "name": page.name, "journal_day": journal_day },
                    "blocks": page_with_blocks.blocks.iter().map(block_to_json).collect::<Vec<_>>(),
                    "block_count": page_with_blocks.blocks.len(),
                })).unwrap_or_else(|e| e.to_string()))
            }

            _ => Err(format!("Unknown tool: {}", name)),
        }
    }
}
