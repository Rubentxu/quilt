//! Serendipity Monitor service
//!
//! Thin wrapper around [`crate::connection_engine::ConnectionEngine`]
//! that returns a [`SerendipityMonitorDto`] with block content previews
//! and page names for each highlighted connection.

use super::types::{SerendipityHighlightDetail, SerendipityMonitorDto};
use crate::connection_engine::{ConnectionEngine, SerendipityQuery};
use chrono::Utc;
use quilt_domain::repositories::{BlockRepository, PageRepository};
use std::sync::Arc;

/// Service that produces serendipity highlights as a standalone snapshot.
#[derive(Clone)]
pub struct SerendipityMonitorService {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    connection_engine: Option<ConnectionEngine>,
}

impl std::fmt::Debug for SerendipityMonitorService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SerendipityMonitorService")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("page_repo", &"Arc<dyn PageRepository>")
            .field("connection_engine", &"Option<ConnectionEngine>")
            .finish()
    }
}

impl SerendipityMonitorService {
    /// Create a new service (without a connection engine — one-shot mode).
    pub fn new(block_repo: Arc<dyn BlockRepository>, page_repo: Arc<dyn PageRepository>) -> Self {
        Self {
            block_repo,
            page_repo,
            connection_engine: None,
        }
    }

    /// Create a new service with an existing connection engine.
    pub fn with_engine(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        connection_engine: ConnectionEngine,
    ) -> Self {
        Self {
            block_repo,
            page_repo,
            connection_engine: Some(connection_engine),
        }
    }

    /// Run serendipity detection now and return a DTO with block previews.
    pub async fn detect_now(&self) -> SerendipityMonitorDto {
        let now = Utc::now();

        let query = SerendipityQuery {
            topic: None,
            limit: 20,
            offset: 0,
            min_confidence: 0.3,
            temporal_window_days: Some(30),
            page_id: None,
        };

        let engine = match &self.connection_engine {
            Some(e) => e.clone(),
            None => ConnectionEngine::new(self.block_repo.clone()),
        };

        let connections = match engine.find_connections(query.clone()).await {
            Ok(conns) => conns,
            Err(_) => Vec::new(),
        };

        let total = connections.len();

        let mut highlights = Vec::with_capacity(connections.len());
        for conn in connections {
            let block_a_preview = self.resolve_preview(conn.idea_a).await;
            let block_b_preview = self.resolve_preview(conn.idea_b).await;

            let (block_a_page, block_b_page) = tokio::join!(
                self.resolve_page_name(conn.idea_a),
                self.resolve_page_name(conn.idea_b),
            );

            highlights.push(SerendipityHighlightDetail {
                block_a_id: conn.idea_a.to_string(),
                block_b_id: conn.idea_b.to_string(),
                block_a_preview,
                block_b_preview,
                explanation: conn.explanation,
                confidence: conn.confidence,
                block_a_page,
                block_b_page,
            });
        }

        SerendipityMonitorDto {
            highlights,
            total,
            generated_at: now,
        }
    }

    /// Resolve a block's content preview (up to 200 chars).
    async fn resolve_preview(&self, id: quilt_domain::value_objects::Uuid) -> String {
        match self.block_repo.get_by_id(id).await {
            Ok(Some(block)) => {
                if block.content.len() > 200 {
                    block.content[..200].to_string()
                } else {
                    block.content.clone()
                }
            }
            _ => String::new(),
        }
    }

    /// Resolve a block's page name.
    async fn resolve_page_name(
        &self,
        block_id: quilt_domain::value_objects::Uuid,
    ) -> Option<String> {
        let block = self.block_repo.get_by_id(block_id).await.ok()?;
        let block = block?;
        self.page_repo
            .get_by_id(block.page_id)
            .await
            .ok()?
            .map(|p| p.name)
    }
}
