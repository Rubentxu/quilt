//! Deep link handler implementation for MCP tools.
//!
//! Implements [`DeepLinkHandler`](super::DeepLinkHandler) trait for deep link
//! MCP tools like create_deep_link, get_deep_links, delete_deep_link.

use super::{DeepLinkHandler, HandlerResult};
use async_trait::async_trait;
use quilt_domain::entities::{DeepLink, DeepLinkCreate, LinkSourceType, LinkType};
use quilt_domain::repositories::DeepLinkRepository;
use quilt_domain::value_objects::Uuid;
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;

// ── Local wire-format DTOs ────────────────────────────────────────
//
// These mirror the existing `serde_json::json!({ ... })` shapes used
// by the handler responses. Link enums are serialized via the same
// `format!("{:?}", x).to_lowercase()` lowercase Debug format the
// previous `json!()` calls used, so the wire format is preserved.

/// Wire shape for a `create_deep_link` response — a snapshot of the
/// link as it was just inserted.
#[derive(Serialize)]
struct CreateDeepLinkResponse {
    id: String,
    source_id: String,
    source_type: String,
    target_id: Option<String>,
    target_page_name: Option<String>,
    link_type: String,
    external_url: Option<String>,
    link_text: Option<String>,
    context: Option<String>,
}

/// Wire shape for a `get_deep_links` empty-filter result.
#[derive(Serialize)]
struct EmptyDeepLinksResponse<'a> {
    deep_links: Vec<()>,
    count: usize,
    note: &'a str,
}

/// Wire shape for one deep-link entry in `get_deep_links`.
#[derive(Serialize)]
struct DeepLinkEntryWire {
    id: String,
    source_id: String,
    source_type: String,
    target_id: Option<String>,
    target_page_name: Option<String>,
    link_type: String,
    external_url: Option<String>,
    link_text: Option<String>,
    context: Option<String>,
}

/// Wire shape for the `get_deep_links` outer response.
#[derive(Serialize)]
struct DeepLinksListResponse {
    deep_links: Vec<DeepLinkEntryWire>,
    count: usize,
}

/// Wire shape for the `delete_deep_link` response.
#[derive(Serialize)]
struct DeleteDeepLinkResponse {
    status: &'static str,
    id: String,
}

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

        let response = CreateDeepLinkResponse {
            id: deep_link.id.to_string(),
            source_id: source_id.to_string(),
            source_type,
            target_id: target_id.map(|id| id.to_string()),
            target_page_name,
            link_type,
            external_url,
            link_text,
            context,
        };
        Ok(serde_json::to_string_pretty(&response)
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
            let response = EmptyDeepLinksResponse {
                deep_links: vec![],
                count: 0,
                note: "Provide target_id, link_type, or source_id to filter deep links",
            };
            return Ok(serde_json::to_string_pretty(&response)
                .unwrap_or_else(|e| format!("Serialization error: {}", e)));
        };

        let items: Vec<DeepLinkEntryWire> = links
            .iter()
            .map(|l| DeepLinkEntryWire {
                id: l.id.to_string(),
                source_id: l.source_id.to_string(),
                source_type: format!("{:?}", l.source_type).to_lowercase(),
                target_id: l.target_id.map(|id| id.to_string()),
                target_page_name: l.target_page_name.clone(),
                link_type: format!("{:?}", l.link_type).to_lowercase(),
                external_url: l.external_url.clone(),
                link_text: l.link_text.clone(),
                context: l.context.clone(),
            })
            .collect();

        let response = DeepLinksListResponse {
            count: items.len(),
            deep_links: items,
        };
        Ok(serde_json::to_string_pretty(&response)
            .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }

    #[instrument(skip(self))]
    async fn delete_deep_link(&self, id: Uuid) -> HandlerResult {
        self.deep_link_repo
            .delete(id)
            .await
            .map_err(|e| e.to_string())?;

        let response = DeleteDeepLinkResponse {
            status: "deleted",
            id: id.to_string(),
        };
        serde_json::to_string(&response).map_err(|e| e.to_string())
    }
}
