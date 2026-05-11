//! Background AI Tasks
//!
//! Long-running background services that provide AI-assisted features:
//! - `AITaggingService`: Automatically suggests tags for untagged blocks
//! - `AILinkDiscovery`: Automatically suggests links between related blocks

use crate::ai_client::{AIClient, AIClientError};
use crate::counterfactual_explorer::CounterfactualExplorer;
use crate::knowledge_evolution::KnowledgeEvolutionTracker;
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Polling interval for tag service (10 seconds).
const TAG_POLL_INTERVAL_SECS: u64 = 10;
/// Polling interval for link discovery (60 seconds).
const LINK_POLL_INTERVAL_SECS: u64 = 60;

/// AI Tagging Service
///
/// Polls for untagged blocks and suggests tags using AI.
pub struct AITaggingService {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
    shutdown: broadcast::Receiver<()>,
}

impl AITaggingService {
    /// Create a new AITaggingService.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        ai_client: Arc<dyn AIClient>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            block_repo,
            ai_client,
            shutdown,
        }
    }

    /// Run the tagging service until shutdown.
    pub fn run(self) -> impl Future<Output = ()> {
        self.run_impl()
    }

    async fn run_impl(self) {
        info!("AITaggingService started");
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(TAG_POLL_INTERVAL_SECS)
        );

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("AITaggingService shutting down");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = self.process_untagged_blocks().await {
                        error!("AITaggingService error: {}", e);
                    }
                }
            }
        }
    }

    /// Process untagged blocks and suggest tags.
    async fn process_untagged_blocks(&self) -> Result<(), AITaggingError> {
        let since = chrono::Utc::now() - chrono::Duration::hours(24);
        let blocks = self
            .block_repo
            .get_updated_since(since)
            .await
            .map_err(AITaggingError::Repository)?;

        // Find blocks without tags or with empty tags
        let untagged: Vec<_> = blocks
            .into_iter()
            .filter(|b| b.tags.is_empty())
            .take(10) // Process in batches
            .collect();

        for block in untagged {
            match self.suggest_tags_for_block(&block).await {
                Ok(tags) if !tags.is_empty() => {
                    info!(
                        block_id = %block.id,
                        tags = ?tags,
                        "Suggested tags for block"
                    );
                    // In a full implementation, these would be presented to the user
                    // or automatically applied based on confidence thresholds
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(block_id = %block.id, error = %e, "Failed to suggest tags");
                }
            }
        }

        Ok(())
    }

    /// Suggest tags for a single block.
    async fn suggest_tags_for_block(&self, block: &Block) -> Result<Vec<String>, AIClientError> {
        self.ai_client.suggest_tags(&block.content, None).await
    }
}

/// AI Link Discovery Service
///
/// Polls for recent blocks and suggests links to related content.
pub struct AILinkDiscovery {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
    shutdown: broadcast::Receiver<()>,
}

impl AILinkDiscovery {
    /// Create a new AILinkDiscovery service.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        ai_client: Arc<dyn AIClient>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            block_repo,
            ai_client,
            shutdown,
        }
    }

    /// Run the link discovery service until shutdown.
    pub fn run(self) -> impl Future<Output = ()> {
        self.run_impl()
    }

    async fn run_impl(self) {
        info!("AILinkDiscovery started");
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(LINK_POLL_INTERVAL_SECS)
        );

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!("AILinkDiscovery shutting down");
                    break;
                }
                _ = interval.tick() => {
                    if let Err(e) = self.process_recent_blocks().await {
                        error!("AILinkDiscovery error: {}", e);
                    }
                }
            }
        }
    }

    /// Process recent blocks and suggest links.
    async fn process_recent_blocks(&self) -> Result<(), AILinkDiscoveryError> {
        let since = chrono::Utc::now() - chrono::Duration::hours(1);
        let blocks = self
            .block_repo
            .get_updated_since(since)
            .await
            .map_err(AILinkDiscoveryError::Repository)?;

        for block in blocks.into_iter().take(5) {
            match self.suggest_links_for_block(&block).await {
                Ok(links) if !links.is_empty() => {
                    info!(
                        block_id = %block.id,
                        suggested_links = ?links,
                        "Suggested links for block"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(block_id = %block.id, error = %e, "Failed to suggest links");
                }
            }
        }

        Ok(())
    }

    /// Suggest links for a single block.
    async fn suggest_links_for_block(&self, block: &Block) -> Result<Vec<String>, AIClientError> {
        let existing: Vec<String> = block.refs.iter().map(|r| r.to_string()).collect();
        self.ai_client.suggest_links(&block.content, &existing).await
    }
}

/// Errors for AI tagging service.
#[derive(Debug, thiserror::Error)]
pub enum AITaggingError {
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("AI client error: {0}")]
    AI(#[from] AIClientError),
}

/// Errors for AI link discovery service.
#[derive(Debug, thiserror::Error)]
pub enum AILinkDiscoveryError {
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("AI client error: {0}")]
    AI(#[from] AIClientError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_values() {
        assert_eq!(TAG_POLL_INTERVAL_SECS, 10);
        assert_eq!(LINK_POLL_INTERVAL_SECS, 60);
    }
}
