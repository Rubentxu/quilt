//! Block handler implementation for MCP tools.
//!
//! Implements [`BlockHandler`](super::BlockHandler) trait for block-related
//! MCP tools like create_block, delete_block, get_block_tree, etc.

use super::{BlockHandler, HandlerResult};
use async_trait::async_trait;
use quilt_domain::entities::{Block, BlockCreate};
use quilt_domain::content::BlockContent;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::services::TimezoneService;
use quilt_domain::value_objects::{BlockFormat, TaskMarker, Uuid};
use std::sync::Arc;
use tracing::instrument;

/// Default implementation of [`BlockHandler`].
pub struct DefaultBlockHandler {
    block_repo: Arc<dyn BlockRepository>,
    timezone_service: Arc<TimezoneService>,
}

impl DefaultBlockHandler {
    /// Create a new block handler.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        timezone_service: Arc<TimezoneService>,
    ) -> Self {
        Self {
            block_repo,
            timezone_service,
        }
    }
}

#[async_trait]
impl BlockHandler for DefaultBlockHandler {
    #[instrument(skip(self))]
    async fn create_block(&self, params: super::CreateBlockParams) -> HandlerResult {
        let block = Block::new(
            BlockCreate {
                page_id: params
                    .page_name
                    .parse()
                    .map_err(|e: uuid::Error| e.to_string())?,
                content: BlockContent::from_text(params.content),
                parent_id: params.parent_id,
                order: 1.0,
                marker: params.marker.as_ref().and_then(|m| TaskMarker::from_str(m)),
                format: BlockFormat::Markdown,
                properties: Default::default(),
            },
            &self.timezone_service,
        )
        .map_err(|e| e.to_string())?;

        self.block_repo
            .insert(&block)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "id": block.id.to_string(),
            "page_id": block.page_id.to_string(),
            "content": &block.content,
            "parent_id": params.parent_id.map(|id| id.to_string()),
            "marker": params.marker,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn get_block_tree(&self, block_id: Uuid) -> HandlerResult {
        let block = self
            .block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Block not found: {}", block_id))?;

        let children = self
            .block_repo
            .get_children(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "block": {
                "id": block.id.to_string(),
                "content": &block.content,
                "page_id": block.page_id.to_string(),
            },
            "children": children.iter().map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "content": &b.content,
                    "page_id": b.page_id.to_string(),
                })
            }).collect::<Vec<_>>(),
            "children_count": children.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn delete_block(&self, block_id: Uuid) -> HandlerResult {
        self.block_repo
            .get_by_id(block_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Block not found: {}", block_id))?;

        self.block_repo
            .delete(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "deleted",
            "block_id": block_id.to_string(),
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn restore_block(&self, block_id: Uuid) -> HandlerResult {
        self.block_repo
            .restore(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "restored",
            "block_id": block_id.to_string(),
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn link_blocks(&self, source_id: Uuid, target_id: Uuid) -> HandlerResult {
        // Verify both blocks exist
        let mut source = self
            .block_repo
            .get_by_id(source_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Source block not found: {}", source_id))?;

        self.block_repo
            .get_by_id(target_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Target block not found: {}", target_id))?;

        // Add reference to source block's refs list
        if !source.refs.contains(&target_id) {
            source.refs.push(target_id);
            self.block_repo
                .update(&source)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(serde_json::json!({
            "status": "linked",
            "source_id": source_id.to_string(),
            "target_id": target_id.to_string(),
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn get_backlinks(&self, block_id: Uuid) -> HandlerResult {
        let backlinks = self
            .block_repo
            .get_backlinks(block_id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "block_id": block_id.to_string(),
            "backlinks": backlinks.iter().map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "content": &b.content,
                    "page_id": b.page_id.to_string(),
                })
            }).collect::<Vec<_>>(),
            "count": backlinks.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn recycle_bin(&self) -> HandlerResult {
        let blocks = self
            .block_repo
            .recycle_bin()
            .await
            .map_err(|e| e.to_string())?;

        let items: Vec<serde_json::Value> = blocks
            .iter()
            .map(|b| {
                serde_json::json!({
                    "block_id": b.id.to_string(),
                    "page_id": b.page_id.to_string(),
                    "content": &b.content,
                    "deleted_at": b.updated_at.to_rfc3339(),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "recycle_bin": items,
            "count": items.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn orphan_pages(&self) -> HandlerResult {
        // This method is on PageHandler, but we need page_repo
        // For now, return empty - will be handled by PageHandler
        Ok(serde_json::json!({
            "orphan_pages": [],
            "count": 0,
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn orphan_blocks(&self) -> HandlerResult {
        let blocks = self
            .block_repo
            .get_orphan_blocks()
            .await
            .map_err(|e| e.to_string())?;

        let items: Vec<serde_json::Value> = blocks
            .iter()
            .map(|b| {
                serde_json::json!({
                    "id": b.id.to_string(),
                    "page_id": b.page_id.to_string(),
                    "content": &b.content,
                    "created_at": b.created_at.to_rfc3339(),
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "orphan_blocks": items,
            "count": items.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }
}

/// Dummy block handler for backward compatibility.
///
/// This handler is used as a placeholder when McpServer is created with
/// real handlers. It should not be called directly.
pub struct DummyBlockHandler;

#[async_trait]
impl BlockHandler for DummyBlockHandler {
    async fn create_block(&self, _params: super::CreateBlockParams) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn get_block_tree(&self, _block_id: Uuid) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn delete_block(&self, _block_id: Uuid) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn restore_block(&self, _block_id: Uuid) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn link_blocks(&self, _source_id: Uuid, _target_id: Uuid) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn get_backlinks(&self, _block_id: Uuid) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn recycle_bin(&self) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn orphan_pages(&self) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }

    async fn orphan_blocks(&self) -> HandlerResult {
        Err("DummyBlockHandler called - use real handlers".to_string())
    }
}
