//! Shared decay detection used by the Morning Briefing engine and the
//! Decay Monitor service.
//!
//! Centralising the algorithm in a free function prevents the two
//! callers from drifting in thresholds, sort order, or cap. The
//! behaviour is intentionally byte-identical to the original
//! private method on `MorningBriefing` — the morning briefing
//! regression tests guard this.

use crate::morning_briefing::types::DecayAlert;
use chrono::{DateTime, Duration, Utc};
use quilt_domain::repositories::{BlockRepository, PageRepository};

/// Threshold in days after which a block is considered "medium" decay.
pub const DECAY_MEDIUM_DAYS: i64 = 14;
/// Threshold in days after which a block is considered "high" decay.
pub const DECAY_HIGH_DAYS: i64 = 30;
/// How far back to look for candidate blocks (in days).
const CANDIDATE_WINDOW_DAYS: i64 = 90;
/// Maximum number of alerts a single call returns.
const MAX_ALERTS: usize = 10;

/// Detect blocks that haven't been updated in a while.
///
/// Returns at most `MAX_ALERTS` (10) `DecayAlert` records, sorted
/// by `days_since_update` descending. A block is a candidate only
/// if it was updated within the last 90 days — anything older is
/// out of scope for the user-facing "review" workflow.
///
/// Repository errors degrade silently to an empty vector: the
/// caller (Morning Briefing or Decay Monitor) is a soft
/// aggregator, not a hard data path, and a transient DB blip
/// should not surface as 5xx to the client.
pub async fn detect_decay_alerts(
    block_repo: &dyn BlockRepository,
    page_repo: &dyn PageRepository,
    today_start: DateTime<Utc>,
) -> Vec<DecayAlert> {
    // Look at blocks updated in the last CANDIDATE_WINDOW_DAYS as candidates
    let window = today_start - Duration::days(CANDIDATE_WINDOW_DAYS);
    let blocks = match block_repo.get_updated_since(window).await {
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
        let page_name = match page_repo.get_by_id(block.page_id).await {
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

    // Sort by days_since_update desc and cap at MAX_ALERTS
    alerts.sort_by(|a, b| b.days_since_update.cmp(&a.days_since_update));
    alerts.truncate(MAX_ALERTS);
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use quilt_domain::entities::{Block, Page};
    use quilt_test_helpers::{InMemoryBlockRepo, InMemoryPageRepo};

    fn today_start() -> DateTime<Utc> {
        let now = Utc::now();
        Utc.from_utc_datetime(&now.date_naive().and_hms_opt(0, 0, 0).unwrap())
    }

    fn make_block(updated_days_ago: i64) -> Block {
        use quilt_domain::value_objects::{BlockFormat, BlockType, Uuid};
        use std::collections::HashMap;
        let now = Utc::now();
        let page_id = Uuid::new_v4();
        Block {
            id: Uuid::new_v4(),
            page_id,
            parent_id: None,
            order: 0.0,
            level: 0,
            format: BlockFormat::Markdown,
            block_type: BlockType::Paragraph,
            marker: None,
            priority: None,
            content: "old meeting notes".to_string(),
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            completed_at: None,
            cancelled_at: None,
            collapsed: false,
            created_at: now - Duration::days(updated_days_ago + 5),
            updated_at: now - Duration::days(updated_days_ago),
        }
    }

    fn page_for_block(block: &Block) -> Page {
        use quilt_domain::properties::entry::DefaultPropertyEntry;
        use quilt_domain::value_objects::{BlockFormat, PropertyValue};
        use std::collections::HashMap;
        Page {
            id: block.page_id,
            name: "journals/test".to_string(),
            title: None,
            namespace_id: None,
            journal_day: None,
            format: BlockFormat::Markdown,
            file_id: None,
            original_name: None,
            journal: false,
            created_at: block.created_at,
            updated_at: block.updated_at,
            properties: HashMap::<String, DefaultPropertyEntry<PropertyValue>>::new(),
        }
    }

    #[tokio::test]
    async fn detects_high_severity_block() {
        let block = make_block(45);
        let page = page_for_block(&block);
        let block_repo = InMemoryBlockRepo::new().with_blocks(vec![block.clone()]);
        let page_repo = InMemoryPageRepo::new().with_pages(vec![page]);
        let alerts = detect_decay_alerts(
            block_repo.as_ref(),
            page_repo.as_ref(),
            today_start(),
        )
        .await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, "high");
        assert!(alerts[0].days_since_update >= 30);
    }

    #[tokio::test]
    async fn detects_medium_severity_block() {
        let block = make_block(20);
        let page = page_for_block(&block);
        let block_repo = InMemoryBlockRepo::new().with_blocks(vec![block.clone()]);
        let page_repo = InMemoryPageRepo::new().with_pages(vec![page]);
        let alerts = detect_decay_alerts(
            block_repo.as_ref(),
            page_repo.as_ref(),
            today_start(),
        )
        .await;
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, "medium");
        assert!(alerts[0].days_since_update >= 14 && alerts[0].days_since_update < 30);
    }

    #[tokio::test]
    async fn skips_recent_block() {
        let block = make_block(2);
        let page = page_for_block(&block);
        let block_repo = InMemoryBlockRepo::new().with_blocks(vec![block.clone()]);
        let page_repo = InMemoryPageRepo::new().with_pages(vec![page]);
        let alerts = detect_decay_alerts(
            block_repo.as_ref(),
            page_repo.as_ref(),
            today_start(),
        )
        .await;
        assert!(alerts.is_empty());
    }

    #[tokio::test]
    async fn caps_at_ten() {
        let blocks: Vec<Block> = (0..15).map(|_| make_block(60)).collect();
        let page = page_for_block(&blocks[0]);
        let block_repo = InMemoryBlockRepo::new().with_blocks(blocks);
        let page_repo = InMemoryPageRepo::new().with_pages(vec![page]);
        let alerts = detect_decay_alerts(
            block_repo.as_ref(),
            page_repo.as_ref(),
            today_start(),
        )
        .await;
        assert_eq!(alerts.len(), 10);
    }

    #[tokio::test]
    async fn returns_empty_on_empty_repos() {
        // An "error" path is hard to simulate without a custom stub
        // (the in-memory repos never error). The integration tests in
        // the server crate cover the actual error path (where the
        // SQLite repo returns Err); this test just asserts that an
        // empty repo path returns no alerts.
        let block_repo = InMemoryBlockRepo::new();
        let page_repo = InMemoryPageRepo::new();
        let alerts = detect_decay_alerts(
            block_repo.as_ref(),
            page_repo.as_ref(),
            today_start(),
        )
        .await;
        assert!(alerts.is_empty());
    }
}
