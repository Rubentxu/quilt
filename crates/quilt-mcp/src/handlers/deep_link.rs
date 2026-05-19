//! Deep link handler implementation for MCP tools.
//!
//! Implements [`DeepLinkHandler`](super::DeepLinkHandler) trait for deep link
//! MCP tools like create_deep_link, get_deep_links, delete_deep_link.

use super::{DeepLinkHandler, HandlerResult};
use async_trait::async_trait;
use quilt_domain::entities::{DeepLink, DeepLinkCreate, LinkSourceType, LinkType};
use quilt_domain::repositories::DeepLinkRepository;
use quilt_domain::value_objects::Uuid;
use std::sync::Arc;
use tracing::instrument;

/// Default implementation of [`DeepLinkHandler`].
pub struct DefaultDeepLinkHandler {
    deep_link_repo: Arc<dyn DeepLinkRepository>,
}

impl DefaultDeepLinkHandler {
    /// Create a new deep link handler.
    pub fn new(deep_link_repo: Arc<dyn DeepLinkRepository>) -> Self {
        Self { deep_link_repo }
    }
}

#[async_trait]
impl DeepLinkHandler for DefaultDeepLinkHandler {
    #[instrument(skip(self))]
    async fn create_deep_link(
        &self,
        source_id: Uuid,
        source_type: String,
        target_id: Option<Uuid>,
        target_page_name: Option<String>,
        link_type: String,
        external_url: Option<String>,
        link_text: Option<String>,
        context: Option<String>,
    ) -> HandlerResult {
        let source_t = match source_type.to_lowercase().as_str() {
            "block" => LinkSourceType::Block,
            "page" => LinkSourceType::Page,
            other => return Err(format!("Unknown source type: {}", other)),
        };

        let link_t = match link_type.to_lowercase().as_str() {
            "block" => LinkType::InternalBlock,
            "page" => LinkType::InternalPage,
            "url" => LinkType::ExternalUrl,
            other => return Err(format!("Unknown link type: {}", other)),
        };

        let deep_link = DeepLink::new(DeepLinkCreate {
            source_id,
            source_type: source_t,
            target_id,
            target_page_name: target_page_name.clone(),
            link_type: link_t,
            external_url: external_url.clone(),
            link_text: link_text.clone(),
            context: context.clone(),
        })
        .map_err(|e| e.to_string())?;

        self.deep_link_repo
            .insert(&deep_link)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "id": deep_link.id.to_string(),
            "source_id": source_id.to_string(),
            "source_type": source_type,
            "target_id": target_id.map(|id| id.to_string()),
            "target_page_name": target_page_name,
            "link_type": link_type,
            "external_url": external_url,
            "link_text": link_text,
            "context": context,
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn get_deep_links(
        &self,
        source_id: Option<Uuid>,
        _source_type: Option<String>,
        target_id: Option<Uuid>,
        link_type: Option<String>,
        limit: Option<usize>,
    ) -> HandlerResult {
        let limit = limit.unwrap_or(50);

        // Use the most specific filter available based on provided params
        let links = if let Some(tid) = target_id {
            self.deep_link_repo
                .get_by_target(tid)
                .await
                .map_err(|e| e.to_string())?
        } else if let Some(link_t) = link_type {
            let lt = match link_t.to_lowercase().as_str() {
                "block" => LinkType::InternalBlock,
                "page" => LinkType::InternalPage,
                "url" => LinkType::ExternalUrl,
                _ => return Err(format!("Unknown link type: {}", link_t)),
            };
            self.deep_link_repo
                .get_by_type(lt)
                .await
                .map_err(|e| e.to_string())?
        } else if let Some(sid) = source_id {
            // Default to Block source type if not specified
            self.deep_link_repo
                .get_page(sid, LinkSourceType::Block, 0, limit)
                .await
                .map_err(|e| e.to_string())?
        } else {
            // No filter provided - return empty result with guidance
            return Ok(serde_json::to_string_pretty(&serde_json::json!({
                "deep_links": [],
                "count": 0,
                "note": "Provide target_id, link_type, or source_id to filter deep links"
            })).unwrap_or_else(|e| format!("Serialization error: {}", e)));
        };

        let items: Vec<serde_json::Value> = links
            .iter()
            .map(|l| {
                serde_json::json!({
                    "id": l.id.to_string(),
                    "source_id": l.source_id.to_string(),
                    "source_type": format!("{:?}", l.source_type).to_lowercase(),
                    "target_id": l.target_id.map(|id| id.to_string()),
                    "target_page_name": l.target_page_name,
                    "link_type": format!("{:?}", l.link_type).to_lowercase(),
                    "external_url": l.external_url,
                    "link_text": l.link_text,
                    "context": l.context,
                })
            })
            .collect();

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "deep_links": items,
            "count": items.len(),
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn delete_deep_link(&self, id: Uuid) -> HandlerResult {
        self.deep_link_repo
            .delete(id)
            .await
            .map_err(|e| e.to_string())?;

        Ok(serde_json::json!({
            "status": "deleted",
            "id": id.to_string(),
        })
        .to_string())
    }
}
