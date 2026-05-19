//! Search handler implementation for MCP tools.
//!
//! Implements [`SearchHandler`](super::SearchHandler) trait for search and query
//! MCP tools like search, query, rebuild_index, index_health.

use super::{HandlerResult, SearchHandler as SearchHandlerTrait};
use async_trait::async_trait;
use quilt_search::{SearchIndexManager, SearchService};
use std::sync::Arc;
use tracing::instrument;

/// Default implementation of [`SearchHandler`].
pub struct DefaultSearchHandler {
    search_service: Arc<SearchService>,
    search_index: Option<Arc<SearchIndexManager>>,
    pool: Option<sqlx::SqlitePool>,
}

impl DefaultSearchHandler {
    /// Create a new search handler.
    pub fn new(
        search_service: Arc<SearchService>,
        search_index: Option<Arc<SearchIndexManager>>,
        pool: Option<sqlx::SqlitePool>,
    ) -> Self {
        Self {
            search_service,
            search_index,
            pool,
        }
    }
}

#[async_trait]
impl SearchHandlerTrait for DefaultSearchHandler {
    #[instrument(skip(self))]
    async fn search(&self, params: super::SearchParams) -> HandlerResult {
        let limit = params.limit.unwrap_or(50);

        let results = &*self
            .search_service
            .search(&params.query, limit)
            .await
            .map_err(|e| e.to_string())?;

        let json_results: Vec<serde_json::Value> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "block_id": r.block_id.to_string(),
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

    #[instrument(skip(self))]
    async fn query(&self, params: super::QueryParams) -> HandlerResult {
        let _pool = self.pool.as_ref().ok_or_else(|| {
            "Database pool not configured. Use with_pool() to enable queries.".to_string()
        })?;

        // For now, return a placeholder - actual DSL query execution would go here
        // The original implementation calls QueryService::execute which requires
        // the DSL parser to be implemented
        Ok(serde_json::json!({
            "dsl": params.dsl,
            "limit": params.limit.unwrap_or(100),
            "note": "DSL query execution not yet implemented - this is a placeholder"
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn rebuild_index(&self, mode: Option<String>, since: Option<String>) -> HandlerResult {
        let index = self.search_index.as_ref().ok_or_else(|| {
            "SearchIndex not configured. Use with_search_index() to enable.".to_string()
        })?;

        let mode = mode.as_deref().unwrap_or("full");

        match mode {
            "incremental" => {
                let since_str = since
                    .ok_or_else(|| "Since timestamp required for incremental mode".to_string())?;
                let since_dt = chrono::DateTime::parse_from_rfc3339(&since_str)
                    .map_err(|e| format!("Invalid timestamp format (RFC3339 required): {}", e))?
                    .with_timezone(&chrono::Utc);
                index
                    .rebuild_incremental(since_dt)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                index.rebuild_full().await.map_err(|e| e.to_string())?;
            }
        }

        Ok(serde_json::json!({
            "status": "rebuilt",
            "mode": mode,
        })
        .to_string())
    }

    #[instrument(skip(self))]
    async fn index_health(&self) -> HandlerResult {
        let index = self.search_index.as_ref().ok_or_else(|| {
            "SearchIndex not configured. Use with_search_index() to enable.".to_string()
        })?;

        let health = index.health_check().await.map_err(|e| e.to_string())?;

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "fts_count": health.fts_count,
            "blocks_count": health.blocks_count,
            "in_sync": health.in_sync,
            "status": if health.in_sync { "healthy" } else { "out_of_sync" },
        }))
        .unwrap_or_else(|e| format!("Serialization error: {}", e)))
    }
}
