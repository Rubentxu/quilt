//! Knowledge Evolution Tracker Engine
//!
//! Tracks how knowledge and beliefs evolve over time by analyzing
//! changes in blocks and their content.

use crate::ai_client::{AIClient, AIClientError};
use crate::knowledge_evolution::types::{BeliefChange, KnowledgeTimeline};
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// The KnowledgeEvolutionTracker tracks belief and knowledge changes over time.
#[derive(Clone)]
pub struct KnowledgeEvolutionTracker {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
}

impl std::fmt::Debug for KnowledgeEvolutionTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KnowledgeEvolutionTracker")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("ai_client", &"Arc<dyn AIClient>")
            .finish()
    }
}

impl KnowledgeEvolutionTracker {
    /// Create a new KnowledgeEvolutionTracker.
    pub fn new(block_repo: Arc<dyn BlockRepository>, ai_client: Arc<dyn AIClient>) -> Self {
        Self {
            block_repo,
            ai_client,
        }
    }

    /// Track the evolution of knowledge for a topic over a given timespan.
    ///
    /// Analyzes blocks updated within the timespan to detect belief changes,
    /// abandoned ideas, and reinforced ideas.
    #[instrument(skip(self))]
    pub async fn track(
        &self,
        topic: &str,
        timespan_days: u32,
    ) -> Result<KnowledgeTimeline, KnowledgeEvolutionError> {
        let mut timeline = KnowledgeTimeline::new(topic);

        let since = chrono::Utc::now() - chrono::Duration::days(timespan_days as i64);
        let blocks = self
            .block_repo
            .get_updated_since(since)
            .await
            .map_err(KnowledgeEvolutionError::Repository)?;

        // Filter blocks that are related to the topic
        let topic_blocks: Vec<&Block> = blocks
            .iter()
            .filter(|b| b.content.to_lowercase().contains(&topic.to_lowercase()))
            .collect();

        // Group blocks by rough time periods
        let period_blocks = self.group_blocks_by_period(&topic_blocks, timespan_days);

        // Detect belief changes between periods
        let belief_changes = self.detect_belief_changes(&period_blocks, topic).await?;
        for change in belief_changes {
            timeline.add_change(change);
        }

        // Identify abandoned and reinforced ideas
        let (abandoned, reinforced) = self.identify_ideas(&topic_blocks, topic).await;
        for idea in abandoned {
            timeline.add_abandoned(idea);
        }
        for idea in reinforced {
            timeline.add_reinforced(idea);
        }

        Ok(timeline)
    }

    /// Group blocks into time periods for comparison.
    fn group_blocks_by_period<'a>(
        &self,
        blocks: &'a [&Block],
        timespan_days: u32,
    ) -> HashMap<String, Vec<&'a Block>> {
        let mut periods: HashMap<String, Vec<&'a Block>> = HashMap::new();
        let now = chrono::Utc::now();

        for block in blocks {
            let days_ago = (now - block.updated_at).num_days();
            let period = if days_ago < timespan_days as i64 / 4 {
                "recent".to_string()
            } else if days_ago < timespan_days as i64 / 2 {
                "mid".to_string()
            } else {
                "old".to_string()
            };
            periods.entry(period).or_default().push(block);
        }

        periods
    }

    /// Detect belief changes by comparing content across time periods.
    async fn detect_belief_changes(
        &self,
        period_blocks: &HashMap<String, Vec<&Block>>,
        topic: &str,
    ) -> Result<Vec<BeliefChange>, KnowledgeEvolutionError> {
        let mut changes = Vec::new();

        // Get content summaries for each period
        let recent_content = self.summarize_content(
            period_blocks
                .get("recent")
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        );
        let old_content = self.summarize_content(
            period_blocks
                .get("old")
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
        );

        if recent_content != old_content && !recent_content.is_empty() && !old_content.is_empty() {
            // Try AI-based comparison first
            let change = self
                .compare_with_ai(&old_content, &recent_content, topic)
                .await;
            changes.push(change);
        } else if recent_content != old_content {
            // Fallback to heuristic
            changes.push(BeliefChange::new(
                chrono::Utc::now(),
                old_content.as_str(),
                recent_content.as_str(),
                0.5,
                None,
            ));
        }

        Ok(changes)
    }

    /// Summarize content from blocks.
    fn summarize_content(&self, blocks: &[&Block]) -> String {
        blocks
            .iter()
            .map(|b| b.content.as_plain_text().chars().take(100).collect::<String>())
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// Compare two content summaries using AI.
    async fn compare_with_ai(&self, old: &str, new: &str, topic: &str) -> BeliefChange {
        let prompt = format!(
            "Compare old and new understanding of '{}':\nOld: {}\nNew: {}\n\n\
            Identify what changed and return JSON with: from, to, confidence (0-1)",
            topic, old, new
        );

        if let Ok(response) = self
            .ai_client
            .chat("You are a knowledge evolution analyst.", &prompt)
            .await
        {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&response) {
                return BeliefChange::new(
                    chrono::Utc::now(),
                    json.get("from").and_then(|v| v.as_str()).unwrap_or(old),
                    json.get("to").and_then(|v| v.as_str()).unwrap_or(new),
                    json.get("confidence")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.5) as f32,
                    None,
                );
            }
        }

        // Fallback
        BeliefChange::new(chrono::Utc::now(), old, new, 0.5, None)
    }

    /// Identify abandoned and reinforced ideas.
    async fn identify_ideas(&self, blocks: &[&Block], topic: &str) -> (Vec<String>, Vec<String>) {
        let mut abandoned = Vec::new();
        let mut reinforced = Vec::new();

        // Heuristic: count positive/negative indicators in content

        for block in blocks {
            let content_lower = block.content.to_lowercase();
            // Indicators of reinforcement
            if content_lower.contains("confirmed")
                || content_lower.contains("proven")
                || content_lower.contains("still correct")
                || content_lower.contains("reinforced")
            {
                reinforced.push(block.content.as_plain_text().chars().take(50).collect());
            }
            // Indicators of abandonment
            if content_lower.contains("changed")
                || content_lower.contains("abandoned")
                || content_lower.contains("no longer")
                || content_lower.contains("rejected")
            {
                abandoned.push(block.content.as_plain_text().chars().take(50).collect());
            }
        }

        // If no specific ideas found, use heuristic based on recency
        if reinforced.is_empty() && abandoned.is_empty() && !blocks.is_empty() {
            let now = chrono::Utc::now();
            let recent_blocks: Vec<_> = blocks
                .iter()
                .filter(|b| (now - b.updated_at).num_days() < 7)
                .collect();

            if !recent_blocks.is_empty() {
                reinforced.push(format!("Recently discussed: {}", topic));
            }
        }

        (abandoned, reinforced)
    }
}

/// Errors for the KnowledgeEvolutionTracker.
#[derive(Debug, thiserror::Error)]
pub enum KnowledgeEvolutionError {
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("AI client error: {0}")]
    AI(#[from] AIClientError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use quilt_domain::value_objects::{BlockFormat, JournalDay, Uuid};
    use std::collections::HashMap;

    fn make_block(id: Uuid, content: &str, days_ago: i64) -> Block {
        Block {
            id,
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: quilt_domain::content::BlockContent::from_text(content),
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now() - chrono::Duration::days(days_ago),
            updated_at: chrono::Utc::now() - chrono::Duration::days(days_ago),
            journal_day: None,
            updated_journal_day: None,
        }
    }

    #[derive(Debug, Clone, Default)]
    struct MockBlockRepo {
        blocks: Vec<Block>,
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<Block>, quilt_domain::errors::DomainError> {
            Ok(None)
        }
        async fn get_by_page(
            &self,
            _page_id: Uuid,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_children(
            &self,
            _parent_id: Uuid,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(
            &self,
            _id: Uuid,
        ) -> Result<(Block, Vec<Uuid>), quilt_domain::errors::DomainError> {
            Err(quilt_domain::errors::DomainError::NotImplemented("mock"))
        }
        async fn insert(&self, _block: &Block) -> Result<(), quilt_domain::errors::DomainError> {
            Ok(())
        }
        async fn update(&self, _block: &Block) -> Result<(), quilt_domain::errors::DomainError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), quilt_domain::errors::DomainError> {
            Ok(())
        }

        async fn hard_delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn restore(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }

        async fn get_deleted_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn recycle_bin(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), quilt_domain::errors::DomainError> {
            Ok(())
        }
        async fn get_backlinks(
            &self,
            _block_id: Uuid,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(self.blocks.clone())
        }
        async fn count_by_page(
            &self,
            _page_id: Uuid,
        ) -> Result<usize, quilt_domain::errors::DomainError> {
            Ok(0)
        }
        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
    }

    fn make_test_tracker(blocks: Vec<Block>) -> KnowledgeEvolutionTracker {
        let repo = Arc::new(MockBlockRepo { blocks });
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        KnowledgeEvolutionTracker::new(repo, ai)
    }

    #[tokio::test]
    async fn test_track_topic() {
        let blocks = vec![
            make_block(Uuid::new_v4(), "Learning Rust async programming", 30),
            make_block(Uuid::new_v4(), "Rust async is great and proven", 5),
        ];
        let tracker = make_test_tracker(blocks);

        let result = tracker.track("Rust", 30).await.unwrap();

        assert_eq!(result.topic, "Rust");
        // Should have detected some belief changes
        assert!(!result.belief_changes.is_empty() || !result.reinforced_ideas.is_empty());
    }

    #[tokio::test]
    async fn test_track_with_no_matching_blocks() {
        let blocks = vec![make_block(Uuid::new_v4(), "Going to the grocery store", 5)];
        let tracker = make_test_tracker(blocks);

        let result = tracker.track("Rust", 30).await.unwrap();

        assert_eq!(result.topic, "Rust");
        // No matching blocks, so empty timeline
        assert!(result.belief_changes.is_empty());
    }

    #[tokio::test]
    async fn test_track_with_abandoned_ideas() {
        let blocks = vec![
            make_block(
                Uuid::new_v4(),
                "I thought Rust was hard but changed my mind",
                10,
            ),
            make_block(Uuid::new_v4(), "No longer believe in that old approach", 5),
        ];
        let tracker = make_test_tracker(blocks);

        let result = tracker.track("Rust", 30).await.unwrap();

        assert!(!result.abandoned_ideas.is_empty() || !result.reinforced_ideas.is_empty());
    }

    #[test]
    fn test_tracker_debug() {
        let tracker = make_test_tracker(vec![]);
        let debug = format!("{:?}", tracker);
        assert!(debug.contains("KnowledgeEvolutionTracker"));
    }
}
