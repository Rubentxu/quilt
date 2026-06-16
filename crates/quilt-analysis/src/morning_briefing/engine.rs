//! Morning Briefing Engine
//!
//! Generates a daily briefing by aggregating data from the knowledge graph.

use crate::morning_briefing::types::*;
use crate::ConnectionEngine;
use chrono::{DateTime, Duration, Utc};
use quilt_domain::repositories::{BlockRepository, PageRepository};
use std::sync::Arc;
use tracing::instrument;

/// Threshold in days after which a block is considered "medium" decay.
const DECAY_MEDIUM_DAYS: i64 = 14;
/// Threshold in days after which a block is considered "high" decay.
const DECAY_HIGH_DAYS: i64 = 30;

/// The Morning Briefing service.
///
/// Aggregates agenda items, decay alerts, and serendipity highlights
/// into a single daily snapshot.
#[derive(Clone)]
pub struct MorningBriefing {
    block_repo: Arc<dyn BlockRepository>,
    page_repo: Arc<dyn PageRepository>,
    connection_engine: Option<ConnectionEngine>,
}

impl std::fmt::Debug for MorningBriefing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MorningBriefing")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("page_repo", &"Arc<dyn PageRepository>")
            .finish()
    }
}

impl MorningBriefing {
    /// Create a new MorningBriefing service.
    ///
    /// The `connection_engine` parameter is optional — if not provided,
    /// serendipity highlights will be empty.
    pub fn new(
        block_repo: Arc<dyn BlockRepository>,
        page_repo: Arc<dyn PageRepository>,
        connection_engine: Option<ConnectionEngine>,
    ) -> Self {
        Self {
            block_repo,
            page_repo,
            connection_engine,
        }
    }

    /// Generate the morning briefing for today.
    #[instrument(skip(self))]
    pub async fn generate(&self) -> MorningBriefingDto {
        let now = Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let today_start: DateTime<Utc> = DateTime::from_naive_utc_and_offset(today_start, Utc);

        // 1. Build today's agenda from recent blocks
        let agenda_items = self.build_agenda(today_start).await;

        // 2. Detect decay alerts
        let decay_alerts = self.detect_decay_alerts(today_start).await;

        // 3. Find serendipity highlights
        let serendipity_highlights = self.find_serendipity_highlights().await;

        // 4. Count days since last journal
        let days_since_last_journal = self.days_since_last_journal(today_start).await;

        MorningBriefingDto {
            agenda_items,
            decay_alerts,
            serendipity_highlights,
            generated_at: now,
            days_since_last_journal,
        }
    }

    /// Build agenda items from blocks updated today.
    async fn build_agenda(&self, today_start: DateTime<Utc>) -> Vec<AgendaItem> {
        let blocks = match self.block_repo.get_updated_since(today_start).await {
            Ok(blocks) => blocks,
            Err(_) => Vec::new(),
        };

        let mut items = Vec::new();
        for block in blocks.into_iter().take(20) {
            if let Some(item) = self.block_to_agenda_item(&block).await {
                items.push(item);
            }
        }
        items
    }

    async fn block_to_agenda_item(&self, block: &quilt_domain::entities::Block) -> Option<AgendaItem> {
        let content_preview = if block.content.len() > 200 {
            block.content[..200].to_string()
        } else {
            block.content.clone()
        };

        // Resolve page name from page_id
        let page_name = match self.page_repo.get_by_id(block.page_id).await {
            Ok(Some(page)) => page.name,
            _ => format!("page:{}", block.page_id),
        };

        // Check if block has children
        let has_children = match self.block_repo.get_children(block.id).await {
            Ok(children) => !children.is_empty(),
            Err(_) => false,
        };

        Some(AgendaItem {
            block_id: block.id.to_string(),
            content_preview,
            page_name,
            has_children,
            updated_at: block.updated_at,
        })
    }

    /// Detect blocks that haven't been updated in a while.
    async fn detect_decay_alerts(&self, today_start: DateTime<Utc>) -> Vec<DecayAlert> {
        // Look at blocks updated in the last 90 days as candidates
        let window = today_start - Duration::days(90);
        let blocks = match self.block_repo.get_updated_since(window).await {
            Ok(blocks) => blocks,
            Err(_) => return Vec::new(),
        };

        let now = Utc::now();
        let mut alerts = Vec::new();

        for block in blocks {
            let days_since = (now - block.updated_at).num_days();

            // Only flag blocks older than DECAY_MEDIUM_DAYS
            if days_since < DECAY_MEDIUM_DAYS {
                continue;
            }

            let severity = if days_since >= DECAY_HIGH_DAYS {
                "high".to_string()
            } else {
                "medium".to_string()
            };

            let reason = if days_since >= DECAY_HIGH_DAYS {
                format!("No updates in {} days — significantly stale", days_since)
            } else {
                format!("No updates in {} days — consider reviewing", days_since)
            };

            let content_preview = if block.content.len() > 150 {
                block.content[..150].to_string() + "…"
            } else {
                block.content.clone()
            };

            // Resolve page name from page_id
            let page_name = match self.page_repo.get_by_id(block.page_id).await {
                Ok(Some(page)) => page.name,
                _ => format!("page:{}", block.page_id),
            };

            alerts.push(DecayAlert {
                block_id: block.id.to_string(),
                content_preview,
                page_name,
                days_since_update: days_since,
                severity,
                reason,
            });
        }

        // Sort by days_since_update desc and cap at 10
        alerts.sort_by(|a, b| b.days_since_update.cmp(&a.days_since_update));
        alerts.truncate(10);
        alerts
    }

    /// Find serendipitous connections from the connection engine.
    async fn find_serendipity_highlights(&self) -> Vec<SerendipityHighlight> {
        use crate::connection_engine::SerendipityQuery;

        let engine = match &self.connection_engine {
            Some(e) => e,
            None => return Vec::new(),
        };

        let query = SerendipityQuery {
            topic: None,
            limit: 5,
            offset: 0,
            min_confidence: 0.4,
            temporal_window_days: Some(7),
            page_id: None,
        };

        match engine.find_connections(query).await {
            Ok(conns) => conns
                .into_iter()
                .map(|c| SerendipityHighlight {
                    block_a_id: c.idea_a.to_string(),
                    block_b_id: c.idea_b.to_string(),
                    block_a_preview: String::new(),
                    block_b_preview: String::new(),
                    explanation: c.explanation,
                    confidence: c.confidence,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Count days since the last journal page update.
    async fn days_since_last_journal(&self, _today_start: DateTime<Utc>) -> i64 {
        let recent_pages = match self.page_repo.get_recent(100).await {
            Ok(pages) => pages,
            Err(_) => return 0,
        };

        let mut max_date: Option<DateTime<Utc>> = None;
        for page in recent_pages {
            if page.journal {
                if max_date.map(|d| page.updated_at > d).unwrap_or(true) {
                    max_date = Some(page.updated_at);
                }
            }
        }

        match max_date {
            Some(last) => (Utc::now() - last).num_days().max(0),
            None => 0,
        }
    }
}

