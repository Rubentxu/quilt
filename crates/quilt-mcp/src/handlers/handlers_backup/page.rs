//! Page handler implementation for MCP tools.
//!
//! Implements [`PageHandler`](super::PageHandler) trait for page-related
//! MCP tools like list_pages, get_page_blocks, get_journal, create_task, etc.

use super::{HandlerResult, PageInfo, PageHandler as PageHandlerTrait};
use async_trait::async_trait;
use quilt_domain::entities::{Block, BlockCreate, Page, PageCreate};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use quilt_domain::value_objects::{BlockFormat, JournalDay, TaskMarker, Uuid};
use quilt_domain::Clock;
use std::str::FromStr;
use std::sync::Arc;
use tracing::instrument;

/// Default implementation of [`PageHandler`].
pub struct DefaultPageHandler {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    timezone_service: Arc<dyn Clock>,
}

impl DefaultPageHandler {
    /// Create a new page handler.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        timezone_service: Arc<dyn Clock>,
    ) -> Self {
        Self {
            block_repo,
            page_repo,
            timezone_service,
        }
    }
}

#[async_trait]
impl PageHandlerTrait for DefaultPageHandler {
    #[instrument(skip(self))]
    async fn list_pages(&self) -> HandlerResult {
        let pages = self.page_repo.get_all().await.map_err(|e| e.to_string())?;

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
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn get_page_blocks(&self, params: super::PageBlocksParams) -> HandlerResult {
        let page = self
            .page_repo
            .get_by_name(&params.page_name)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Page not found: {}", params.page_name))?;

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "page": { "id": page.id.to_string(), "name": page.name },
            "blocks": blocks.iter().map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "content": b.content.as_ref(),
                    "page_id": b.hierarchy.page_id.to_string(),
                })
            }).collect::<Vec<_>>(),
            "count": blocks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn get_journal(&self, params: super::JournalParams) -> HandlerResult {
        let day = JournalDay::from_str(&params.date).map_err(|e| e.to_string())?;

        let page = match self
            .page_repo
            .get_journal(day)
            .await
            .map_err(|e| e.to_string())?
        {
            Some(p) => p,
            None => {
                // Create journal page
                let page =
                    Page::new_journal(day, BlockFormat::Markdown).map_err(|e| e.to_string())?;
                self.page_repo
                    .insert(&page)
                    .await
                    .map_err(|e| e.to_string())?;
                page
            }
        };

        let blocks = self
            .block_repo
            .get_by_page(page.id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "page": { "id": page.id.to_string(), "name": page.name, "journal_day": day.as_i32() },
            "blocks": blocks.iter().map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "content": b.content.as_ref(),
                    "page_id": b.hierarchy.page_id.to_string(),
                })
            }).collect::<Vec<_>>(),
            "block_count": blocks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn create_task(&self, params: super::CreateTaskParams) -> HandlerResult {
        // Ensure page exists
        let page = match self.page_repo.get_by_name(&params.page_name).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let p = Page::new(PageCreate {
                    name: params.page_name.clone(),
                    title: None,
                    namespace_id: None,
                    journal_day: None,
                    format: BlockFormat::Markdown,
                    file_id: None,
                })
                .map_err(|e| e.to_string())?;
                self.page_repo.insert(&p).await.map_err(|e| e.to_string())?;
                p
            }
            Err(e) => return Err(e.to_string()),
        };

        let marker = params
            .priority
            .as_ref()
            .and_then(|p| match p.to_lowercase().as_str() {
                "a" => Some(TaskMarker::Now),
                "b" => Some(TaskMarker::Later),
                "c" => Some(TaskMarker::Todo),
                _ => Some(TaskMarker::Todo),
            })
            .unwrap_or(TaskMarker::Todo);

        let block = Block::new(
            BlockCreate {
                page_id: page.id,
                content: params.content.clone(),
                parent_id: None,
                order: 1.0,
                marker: Some(marker),
                format: BlockFormat::Markdown,
                properties: Default::default(),
            },
            &*self.timezone_service,
        )
        .map_err(|e| e.to_string())?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "id": block.id.to_string(),
            "page_name": params.page_name,
            "content": params.content,
            "marker": format!("{:?}", marker),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }
}
