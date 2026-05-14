//! Counterfactual Explorer Engine
//!
//! Explores "what if" scenarios by analyzing blocks and their relationships
//! to suggest alternative paths and challenged assumptions.

use crate::ai_client::{AIClient, AIClientError};
use crate::counterfactual_explorer::types::{CounterfactualBranch, CounterfactualTree};
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::JournalDay;
use std::sync::Arc;
use tracing::instrument;

/// The CounterfactualExplorer analyzes scenarios and decision points
/// to generate alternative branch explorations.
#[derive(Clone)]
pub struct CounterfactualExplorer {
    block_repo: Arc<dyn BlockRepository>,
    ai_client: Arc<dyn AIClient>,
}

impl std::fmt::Debug for CounterfactualExplorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterfactualExplorer")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("ai_client", &"Arc<dyn AIClient>")
            .finish()
    }
}

impl CounterfactualExplorer {
    /// Create a new CounterfactualExplorer.
    pub fn new(block_repo: Arc<dyn BlockRepository>, ai_client: Arc<dyn AIClient>) -> Self {
        Self {
            block_repo,
            ai_client,
        }
    }

    /// Explore counterfactual scenarios for a given scenario and decision point.
    ///
    /// Analyzes the scenario and decision point to generate alternative branches,
    /// consequences, and challenged assumptions.
    #[instrument(skip(self))]
    pub async fn explore(
        &self,
        scenario: &str,
        decision_point: &str,
    ) -> Result<CounterfactualTree, CounterfactualExplorerError> {
        let mut tree = CounterfactualTree::new(scenario);

        // Get recent blocks to provide context
        let recent_blocks = self
            .block_repo
            .get_updated_since(chrono::Utc::now() - chrono::Duration::days(7))
            .await
            .map_err(CounterfactualExplorerError::Repository)?;

        // Try to use AI to generate branches if available
        let branches = self
            .generate_branches_ai(scenario, decision_point, &recent_blocks)
            .await?;

        for branch in branches {
            tree.add_branch(branch);
        }

        Ok(tree)
    }

    /// Generate branches using AI analysis.
    async fn generate_branches_ai(
        &self,
        scenario: &str,
        decision_point: &str,
        context_blocks: &[quilt_domain::entities::Block],
    ) -> Result<Vec<CounterfactualBranch>, CounterfactualExplorerError> {
        // Build context from recent blocks
        let context: String = context_blocks
            .iter()
            .take(10)
            .map(|b| b.content.chars().take(200).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n---\n");

        // Try chat completion first
        let prompt = format!(
            "Scenario: {}\nDecision Point: {}\n\nContext:\n{}\n\n\
            Explore this scenario and generate 3 alternative branches. \
            For each branch provide: label, consequences, assumptions_challenged, confidence (0-1). \
            Return JSON array with objects containing: label, consequences[], assumptions_challenged[], confidence.",
            scenario, decision_point, context
        );

        match self
            .ai_client
            .chat("You are a counterfactual reasoning assistant.", &prompt)
            .await
        {
            Ok(response) => {
                // Try to parse JSON from response
                if let Ok(branches) = serde_json::from_str::<Vec<serde_json::Value>>(&response) {
                    let result: Vec<CounterfactualBranch> = branches
                        .iter()
                        .filter_map(|b| {
                            Some(CounterfactualBranch::new(
                                b.get("label")?.as_str()?.to_string(),
                                b.get("consequences")?
                                    .as_array()?
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect(),
                                b.get("assumptions_challenged")?
                                    .as_array()?
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect(),
                                b.get("confidence")?.as_f64().unwrap_or(0.5) as f32,
                            ))
                        })
                        .collect();
                    if !result.is_empty() {
                        return Ok(result);
                    }
                }
                // Fall through to heuristic if parsing failed
                Ok(self.generate_branches_heuristic(scenario, decision_point))
            }
            Err(_) => {
                // AI not available, use heuristic approach
                Ok(self.generate_branches_heuristic(scenario, decision_point))
            }
        }
    }

    /// Generate branches using heuristic rules when AI is unavailable.
    fn generate_branches_heuristic(
        &self,
        scenario: &str,
        decision_point: &str,
    ) -> Vec<CounterfactualBranch> {
        vec![
            CounterfactualBranch::new(
                format!("Alternative to: {}", decision_point),
                vec![
                    format!("Explore different outcome for: {}", scenario),
                    "Consider other factors that might influence the result".to_string(),
                ],
                vec![
                    format!("Challenge assumption: {}", decision_point),
                    "Question the premises of this decision".to_string(),
                ],
                0.6,
            ),
            CounterfactualBranch::new(
                "Opposite approach".to_string(),
                vec![
                    "What if we do the reverse?".to_string(),
                    "Explore contrasting path".to_string(),
                ],
                vec![
                    "Challenge status quo".to_string(),
                    "Question conventional wisdom".to_string(),
                ],
                0.5,
            ),
            CounterfactualBranch::new(
                "Middle ground".to_string(),
                vec![
                    "What if we compromise?".to_string(),
                    "Find balanced alternative".to_string(),
                ],
                vec!["Challenge binary thinking".to_string()],
                0.55,
            ),
        ]
    }
}

/// Errors for the CounterfactualExplorer.
#[derive(Debug, thiserror::Error)]
pub enum CounterfactualExplorerError {
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
    use quilt_domain::value_objects::{BlockFormat, Uuid};
    use quilt_domain::Block;
    use std::collections::HashMap;

    fn make_block(id: Uuid, content: &str) -> quilt_domain::entities::Block {
        quilt_domain::entities::Block {
            id,
            page_id: Uuid::new_v4(),
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: content.to_string(),
            properties: HashMap::new(),
            refs: Vec::new(),
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            journal_day: None,
            updated_journal_day: None,
        }
    }

    #[derive(Debug, Clone, Default)]
    struct MockBlockRepo {
        blocks: Vec<quilt_domain::entities::Block>,
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(
            &self,
            _id: Uuid,
        ) -> Result<Option<quilt_domain::entities::Block>, quilt_domain::errors::DomainError>
        {
            Ok(None)
        }
        async fn get_by_page(
            &self,
            _page_id: Uuid,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_children(
            &self,
            _parent_id: Uuid,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(
            &self,
            _id: Uuid,
        ) -> Result<(quilt_domain::entities::Block, Vec<Uuid>), quilt_domain::errors::DomainError>
        {
            Err(quilt_domain::errors::DomainError::NotImplemented("mock"))
        }
        async fn insert(
            &self,
            _block: &quilt_domain::entities::Block,
        ) -> Result<(), quilt_domain::errors::DomainError> {
            Ok(())
        }
        async fn update(
            &self,
            _block: &quilt_domain::entities::Block,
        ) -> Result<(), quilt_domain::errors::DomainError> {
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
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn search(
            &self,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
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
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
        async fn get_orphan_blocks(
            &self,
        ) -> Result<Vec<quilt_domain::entities::Block>, quilt_domain::errors::DomainError> {
            Ok(vec![])
        }
    }

    fn make_test_explorer(blocks: Vec<quilt_domain::entities::Block>) -> CounterfactualExplorer {
        let repo = Arc::new(MockBlockRepo { blocks });
        let ai = Arc::new(crate::ai_client::MockAIClient::new());
        CounterfactualExplorer::new(repo, ai)
    }

    #[tokio::test]
    async fn test_explore_scenario() {
        let blocks = vec![
            make_block(
                Uuid::new_v4(),
                "Consider learning Rust for systems programming",
            ),
            make_block(Uuid::new_v4(), "Rust has great async support"),
        ];
        let explorer = make_test_explorer(blocks);

        let result = explorer
            .explore("Learning Rust", "Should I invest time in Rust?")
            .await
            .unwrap();

        assert_eq!(result.topic, "Learning Rust");
        assert!(!result.branches.is_empty());
    }

    #[tokio::test]
    async fn test_explore_with_empty_context() {
        let explorer = make_test_explorer(vec![]);

        let result = explorer
            .explore("Test scenario", "Test decision")
            .await
            .unwrap();

        assert!(!result.branches.is_empty());
        assert_eq!(result.branches.len(), 3); // Heuristic generates 3 branches
    }

    #[test]
    fn test_explorer_debug() {
        let blocks = vec![];
        let explorer = make_test_explorer(blocks);
        let debug = format!("{:?}", explorer);
        assert!(debug.contains("CounterfactualExplorer"));
    }
}
