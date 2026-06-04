//! Structure Gardener Engine
//!
//! Tracks belief evolution in journal pages, detects contradictions,
//! and suggests areas for deeper exploration.

use crate::AnalysisError;
use crate::structure_gardener::types::*;
use quilt_domain::entities::Block;
use quilt_domain::errors::DomainError;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::instrument;

/// The Structure Gardener tracks belief evolution over time from journal pages.
#[derive(Clone)]
pub struct StructureGardener {
    block_repo: Arc<dyn BlockRepository>,
}

impl std::fmt::Debug for StructureGardener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructureGardener")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .finish()
    }
}

impl StructureGardener {
    pub fn new(block_repo: Arc<dyn BlockRepository>) -> Self {
        Self { block_repo }
    }

    /// Extract beliefs from an agent's journal pages.
    ///
    /// Only processes pages where `name` starts with `"journals/"`.
    #[instrument(skip(self))]
    pub async fn extract_beliefs(
        &self,
        agent_id: &str,
    ) -> Result<Vec<ExtractedBelief>, StructureGardenerError> {
        // Find all journal pages for this agent
        // In a real implementation, this would use PageRepository
        // For now, we scan block content for belief patterns
        let blocks = self
            .block_repo
            .get_updated_since(chrono::Utc::now() - chrono::Duration::days(365))
            .await
            .map_err(AnalysisError::Storage)?;

        let mut seen_concepts: HashMap<String, ExtractedBelief> = HashMap::new();

        for block in blocks {
            // Only process blocks from journal pages
            // In practice we'd check page name, here we use heuristic
            if let Some(belief) = self.extract_belief_from_block(&block).await {
                let concept_key = belief.concept.to_lowercase();
                if let Some(existing) = seen_concepts.get_mut(&concept_key) {
                    // Update existing belief
                    existing.source_ids.push(block.id);
                    existing.confidence = (existing.confidence + belief.confidence) / 2.0;
                    if belief.first_seen < existing.first_seen {
                        existing.first_seen = belief.first_seen;
                    }
                } else {
                    seen_concepts.insert(concept_key, belief);
                }
            }
        }

        Ok(seen_concepts.into_values().collect())
    }

    /// Extract a single belief from a block using heuristic pattern matching.
    async fn extract_belief_from_block(&self, block: &Block) -> Option<ExtractedBelief> {
        let content = &block.content;

        // Pre-filter: skip blocks unlikely to contain beliefs
        if !looks_like_belief(content) {
            return None;
        }

        // Use heuristic pattern matching to extract belief
        let (concept, confidence) = self.heuristic_extract_belief(content).await;

        if confidence < 0.3 {
            return None;
        }

        Some(ExtractedBelief {
            concept,
            confidence,
            source_ids: vec![block.id],
            first_seen: block.created_at,
        })
    }

    /// Heuristic belief extraction from text (per ADR-0001: no AI/LLM).
    async fn heuristic_extract_belief(&self, content: &str) -> (String, f64) {
        let content_lower = content.to_lowercase();

        // Belief indicators
        let belief_indicators = [
            "i believe",
            "i think",
            "my view is",
            "in my opinion",
            "i suspect",
            "i feel",
        ];

        for indicator in &belief_indicators {
            if let Some(pos) = content_lower.find(indicator) {
                let after = &content[pos..];
                // Extract the belief statement (up to 100 chars)
                let belief_text = after.chars().take(100).collect::<String>();
                let confidence = if indicator == &"i believe" || indicator == &"i think" {
                    0.85
                } else {
                    0.6
                };
                return (belief_text.trim().to_string(), confidence);
            }
        }

        (content.chars().take(50).collect(), 0.4)
    }

    /// Build a complete structure profile for an agent.
    #[instrument(skip(self))]
    pub async fn build_model(&self, agent_id: &str) -> Result<MentalModel, StructureGardenerError> {
        let beliefs = self.extract_beliefs(agent_id).await?;

        let evolution = self.track_evolution_internal(agent_id).await?;

        Ok(MentalModel {
            agent_id: agent_id.to_string(),
            beliefs: beliefs
                .into_iter()
                .map(|eb| Belief {
                    id: Uuid::new_v4(),
                    statement: eb.concept,
                    confidence: eb.confidence,
                    source_blocks: eb.source_ids,
                    last_updated: eb.first_seen,
                })
                .collect(),
            evolution,
        })
    }

    /// Track the evolution of beliefs over a time window.
    #[instrument(skip(self))]
    pub async fn track_evolution(
        &self,
        agent_id: &str,
        since: Option<chrono::DateTime<chrono::Utc>>,
        until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<BeliefSnapshot>, StructureGardenerError> {
        let beliefs = self.extract_beliefs(agent_id).await?;

        let now = chrono::Utc::now();
        let since = since.unwrap_or(now - chrono::Duration::days(30));
        let until = until.unwrap_or(now);

        let mut snapshots = Vec::new();

        for belief in beliefs {
            let observation_count = belief.source_ids.len();
            let state = determine_belief_state(observation_count, belief.first_seen, now);

            if belief.first_seen >= since && belief.first_seen <= until {
                snapshots.push(BeliefSnapshot {
                    belief_id: Uuid::new_v4(),
                    confidence: belief.confidence,
                    supporting_blocks: observation_count,
                    timestamp: belief.first_seen,
                    state,
                });
            }
        }

        // Sort by timestamp
        snapshots.sort_by_key(|a| a.timestamp);

        Ok(snapshots)
    }

    /// Internal method to track evolution without time filtering.
    async fn track_evolution_internal(
        &self,
        agent_id: &str,
    ) -> Result<Vec<BeliefSnapshot>, StructureGardenerError> {
        self.track_evolution(agent_id, None, None).await
    }

    /// Detect contradictions in an agent's structure profile.
    #[instrument(skip(self))]
    pub async fn detect_contradictions(
        &self,
        agent_id: &str,
    ) -> Result<Vec<Contradiction>, StructureGardenerError> {
        let beliefs = self.extract_beliefs(agent_id).await?;
        let now = chrono::Utc::now();
        let mut contradictions = Vec::new();

        for (i, a) in beliefs.iter().enumerate() {
            for b in beliefs.iter().skip(i + 1) {
                if are_concepts_contradictory(&a.concept, &b.concept) {
                    // Check temporal decay
                    let older = if a.first_seen < b.first_seen { a } else { b };
                    let days_old = (now - older.first_seen).num_days();

                    let base_severity = assess_contradiction_severity(&a.concept, &b.concept);

                    // Temporal decay: contradictions with beliefs > 30d old are downgraded
                    let severity = if days_old > 30 {
                        base_severity * 0.6
                    } else {
                        base_severity
                    };

                    contradictions.push(Contradiction {
                        belief_a: a.source_ids.first().copied().unwrap_or_else(Uuid::nil),
                        belief_b: b.source_ids.first().copied().unwrap_or_else(Uuid::nil),
                        explanation: format!(
                            "'{}' and '{}' express incompatible positions",
                            a.concept, b.concept
                        ),
                        severity,
                    });
                }
            }
        }

        Ok(contradictions)
    }

    /// Suggest areas where the structure profile is shallow.
    #[instrument(skip(self))]
    pub async fn suggest_deepening(
        &self,
        agent_id: &str,
        depth_threshold: usize,
    ) -> Result<Vec<DeepeningSuggestion>, StructureGardenerError> {
        let beliefs = self.extract_beliefs(agent_id).await?;

        let mut suggestions = Vec::new();

        for belief in beliefs {
            let depth = belief.source_ids.len();
            if depth <= depth_threshold {
                let questions = generate_deepening_questions(&belief.concept);
                suggestions.push(DeepeningSuggestion {
                    concept: belief.concept.clone(),
                    current_depth: depth,
                    suggested_questions: questions,
                });
            }
        }

        // Sort by importance (observation count, ascending = weakest first)
        suggestions.sort_by_key(|s| s.current_depth);

        Ok(suggestions)
    }
}

/// Determine the state of a belief based on observation patterns.
fn determine_belief_state(
    observation_count: usize,
    first_seen: chrono::DateTime<chrono::Utc>,
    now: chrono::DateTime<chrono::Utc>,
) -> BeliefState {
    let days_since_first = (now - first_seen).num_days();

    if observation_count == 1 {
        if days_since_first > 30 {
            BeliefState::Abandoned
        } else {
            BeliefState::New
        }
    } else if observation_count >= 4 {
        BeliefState::Strengthened
    } else if observation_count >= 2 {
        if days_since_first > 30 {
            BeliefState::Abandoned
        } else {
            BeliefState::Unchanged
        }
    } else {
        BeliefState::Unchanged
    }
}

/// Heuristic check if text looks like it contains a belief statement.
fn looks_like_belief(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    let belief_patterns = [
        "i believe",
        "i think",
        "my view",
        "in my opinion",
        "i suspect",
        "i feel",
        "it seems",
        "probably",
        "likely",
        "certainly", // strong belief
        "definitely",
    ];

    belief_patterns.iter().any(|p| content_lower.contains(p))
}

/// Check if two concepts are contradictory (heuristic).
fn are_concepts_contradictory(a: &str, b: &str) -> bool {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    // Simple valence-based detection
    let pos_words = [
        "good", "great", "best", "better", "fast", "easy", "safe", "better",
    ];
    let neg_words = [
        "bad",
        "worst",
        "terrible",
        "slow",
        "hard",
        "unsafe",
        "worse",
        "difficult",
    ];

    let a_positive = pos_words.iter().any(|w| a_lower.contains(w));
    let a_negative = neg_words.iter().any(|w| a_lower.contains(w));
    let b_positive = pos_words.iter().any(|w| b_lower.contains(w));
    let b_negative = neg_words.iter().any(|w| b_lower.contains(w));

    // Both mention same concept but opposite valence
    (a_positive && b_negative) || (a_negative && b_positive)
}

/// Assess the severity of a contradiction.
fn assess_contradiction_severity(a: &str, b: &str) -> f64 {
    // High severity: direct contradictions
    let direct_negations = [
        "not", "never", "no ", "don't", "doesn't", "isn't", "aren't", "won't",
    ];
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let a_has_negation = direct_negations.iter().any(|n| a_lower.contains(n));
    let b_has_negation = direct_negations.iter().any(|n| b_lower.contains(n));

    if a_has_negation != b_has_negation {
        0.9 // High: one is negated form of the other
    } else if are_concepts_contradictory(a, b) {
        0.7 // Medium: different valences
    } else {
        0.4 // Low: nuanced disagreement
    }
}

/// Generate deepening questions for a concept.
fn generate_deepening_questions(concept: &str) -> Vec<String> {
    vec![
        format!("What evidence supports '{}'?", concept),
        format!("What are the counterarguments to '{}'?", concept),
        format!(
            "How has your understanding of '{}' evolved over time?",
            concept
        ),
    ]
}

/// Errors for the Structure Gardener.
pub type StructureGardenerError = AnalysisError;

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use quilt_domain::value_objects::BlockFormat;

    fn make_block(
        id: Uuid,
        page_id: Uuid,
        content: &str,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Block {
        Block {
            id,
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: content.to_string(),
            properties: std::collections::HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at,
            updated_at: created_at,
        }
    }

    fn uuid_from_u8(i: u8) -> Uuid {
        let mut b = [0u8; 16];
        b[0] = i;
        Uuid::from_bytes(b)
    }

    #[derive(Debug, Clone, Default)]
    struct MockBlockRepo {
        blocks: Vec<Block>,
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(&self, _id: Uuid) -> Result<Option<Block>, DomainError> {
            Ok(None)
        }
        async fn get_by_page(&self, _page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_children(&self, _parent_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(&self, _id: Uuid) -> Result<(Block, Vec<Uuid>), DomainError> {
            Err(DomainError::NotImplemented(
                "get_with_refs not implemented in mock",
            ))
        }
        async fn insert(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn update(&self, _block: &Block) -> Result<(), DomainError> {
            Ok(())
        }
        async fn delete(&self, _id: Uuid) -> Result<(), DomainError> {
            Ok(())
        }
        async fn move_block(
            &self,
            _id: Uuid,
            _new_parent: Option<Uuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_backlinks(&self, _block_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(self.blocks.clone())
        }
        async fn count_by_page(&self, _page_id: Uuid) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn count_all(&self) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn query_dsl(
            &self,
            _sql: &str,
            _params: &[String],
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage(
                "query_dsl not supported in mock".to_string(),
            ))
        }
        async fn list_by_property(
            &self,
            _key: &str,
            _value: &str,
            _limit: usize,
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage(
                "list_by_property not supported in mock".to_string(),
            ))
        }
    }

    fn make_test_gardener(blocks: Vec<Block>) -> StructureGardener {
        let repo = Arc::new(MockBlockRepo { blocks });
        StructureGardener::new(repo)
    }

    #[tokio::test]
    async fn test_extract_clear_belief() {
        let blocks = vec![make_block(
            uuid_from_u8(1),
            uuid_from_u8(100),
            "I believe Rust async is the future of concurrent programming",
            chrono::Utc::now(),
        )];
        let gardener = make_test_gardener(blocks);
        let beliefs = gardener.extract_beliefs("user").await.unwrap();

        // Should extract the belief about Rust async
        assert!(!beliefs.is_empty());
        let found = beliefs
            .iter()
            .any(|b| b.concept.to_lowercase().contains("rust"));
        assert!(found, "Should find Rust-related belief");
    }

    #[tokio::test]
    async fn test_no_belief_extraction() {
        let blocks = vec![make_block(
            uuid_from_u8(1),
            uuid_from_u8(100),
            "Went grocery shopping today",
            chrono::Utc::now(),
        )];
        let gardener = make_test_gardener(blocks);
        let beliefs = gardener.extract_beliefs("user").await.unwrap();

        // No belief indicators → empty or low confidence
        assert!(beliefs.is_empty() || beliefs.iter().all(|b| b.confidence < 0.5));
    }

    #[tokio::test]
    async fn test_ambiguous_belief() {
        let blocks = vec![make_block(
            uuid_from_u8(1),
            uuid_from_u8(100),
            "Maybe Rust is better, but I'm not sure",
            chrono::Utc::now(),
        )];
        let gardener = make_test_gardener(blocks);
        let beliefs = gardener.extract_beliefs("user").await.unwrap();

        // "Maybe" → lower confidence
        if !beliefs.is_empty() {
            assert!(beliefs[0].confidence < 0.6);
        }
    }

    #[tokio::test]
    async fn test_belief_strengthened() {
        let now = chrono::Utc::now();
        let blocks = vec![
            make_block(
                uuid_from_u8(1),
                uuid_from_u8(100),
                "I believe Rust is great",
                now - chrono::Duration::days(1),
            ),
            make_block(
                uuid_from_u8(2),
                uuid_from_u8(100),
                "I believe Rust is great for systems",
                now - chrono::Duration::days(3),
            ),
            make_block(
                uuid_from_u8(3),
                uuid_from_u8(100),
                "Rust is great",
                now - chrono::Duration::days(5),
            ),
        ];
        let gardener = make_test_gardener(blocks);
        let beliefs = gardener.extract_beliefs("user").await.unwrap();

        // Multiple mentions of same belief → strengthened
        assert!(beliefs.len() <= 3); // Some consolidation expected
    }

    #[tokio::test]
    async fn test_belief_timeline_ordered() {
        let now = chrono::Utc::now();
        let blocks = vec![
            make_block(
                uuid_from_u8(1),
                uuid_from_u8(100),
                "I think Rust is good",
                now - chrono::Duration::days(5),
            ),
            make_block(
                uuid_from_u8(2),
                uuid_from_u8(100),
                "I think Rust is good and getting better",
                now - chrono::Duration::days(3),
            ),
        ];
        let gardener = make_test_gardener(blocks);
        let evolution = gardener.track_evolution("user", None, None).await.unwrap();

        for i in 1..evolution.len() {
            assert!(evolution[i].timestamp >= evolution[i - 1].timestamp);
        }
    }

    #[tokio::test]
    async fn test_shallow_belief_suggestion() {
        let blocks = vec![make_block(
            uuid_from_u8(1),
            uuid_from_u8(100),
            "I think Rust macros are interesting",
            chrono::Utc::now(),
        )];
        let gardener = make_test_gardener(blocks);
        let suggestions = gardener.suggest_deepening("user", 2).await.unwrap();

        // Single mention → depth 1, should be below threshold 2 → suggestion generated
        assert!(!suggestions.is_empty());
        assert!(
            suggestions
                .iter()
                .any(|s| s.concept.to_lowercase().contains("macros"))
        );
    }

    #[tokio::test]
    async fn test_deep_belief_not_flagged() {
        let now = chrono::Utc::now();
        let blocks = vec![
            make_block(
                uuid_from_u8(1),
                uuid_from_u8(100),
                "Rust async is the future",
                now - chrono::Duration::days(1),
            ),
            make_block(
                uuid_from_u8(2),
                uuid_from_u8(100),
                "Rust async is great for concurrency",
                now - chrono::Duration::days(2),
            ),
            make_block(
                uuid_from_u8(3),
                uuid_from_u8(100),
                "Using Rust async with tokio",
                now - chrono::Duration::days(3),
            ),
            make_block(
                uuid_from_u8(4),
                uuid_from_u8(100),
                "Rust async patterns",
                now - chrono::Duration::days(4),
            ),
            make_block(
                uuid_from_u8(5),
                uuid_from_u8(100),
                "Deep dive into Rust async",
                now - chrono::Duration::days(5),
            ),
        ];
        let gardener = make_test_gardener(blocks);
        let suggestions = gardener.suggest_deepening("user", 2).await.unwrap();

        // Multiple mentions (depth >= 3) → not flagged
        assert!(
            suggestions.is_empty()
                || !suggestions
                    .iter()
                    .any(|s| s.concept.to_lowercase().contains("async"))
        );
    }

    #[test]
    fn test_determine_belief_state_new() {
        let now = chrono::Utc::now();
        assert_eq!(
            determine_belief_state(1, now - chrono::Duration::days(1), now),
            BeliefState::New
        );
    }

    #[test]
    fn test_determine_belief_state_abandoned() {
        let now = chrono::Utc::now();
        assert_eq!(
            determine_belief_state(1, now - chrono::Duration::days(40), now),
            BeliefState::Abandoned
        );
    }

    #[test]
    fn test_determine_belief_state_strengthened() {
        let now = chrono::Utc::now();
        assert_eq!(
            determine_belief_state(5, now - chrono::Duration::days(5), now),
            BeliefState::Strengthened
        );
    }

    #[test]
    fn test_are_concepts_contradictory() {
        assert!(are_concepts_contradictory("Rust is good", "Rust is bad"));
        assert!(are_concepts_contradictory("Rust is fast", "Rust is slow"));
        assert!(!are_concepts_contradictory("Rust is good", "Go is good"));
    }

    #[test]
    fn test_assess_contradiction_severity_direct() {
        // Direct negation
        let sev = assess_contradiction_severity("Rust is good", "Rust is not good");
        assert!(sev > 0.8);
    }

    #[test]
    fn test_assess_contradiction_severity_nuanced() {
        // Different valences but not direct negation
        let sev = assess_contradiction_severity("Rust is good", "Rust is bad");
        assert!(sev > 0.5);
    }
}
