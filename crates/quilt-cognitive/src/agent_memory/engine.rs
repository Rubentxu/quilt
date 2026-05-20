//! AgentMemory Engine — Main Interface

use crate::agent_memory::store::{self, find_by_key, load_all};
use crate::agent_memory::types::{InteractionProfile, MemoryEntry, MemoryQuery, ThinkingPattern};
use crate::ai_client::AIClient;
use quilt_domain::entities::Block;
use quilt_domain::content::BlockContent;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::{BlockFormat, PropertyValue, Uuid as DomainUuid};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("Profile not found for agent: {0}")]
    ProfileNotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct AgentMemory {
    block_repo: Arc<dyn BlockRepository>,
    #[allow(dead_code)]
    ai_client: Arc<dyn AIClient>,
}

impl std::fmt::Debug for AgentMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentMemory")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("ai_client", &"Arc<dyn AIClient>")
            .finish()
    }
}

impl AgentMemory {
    pub fn new(block_repo: Arc<dyn BlockRepository>, ai_client: Arc<dyn AIClient>) -> Self {
        Self {
            block_repo,
            ai_client,
        }
    }

    /// Load all memory entries for an agent from persistent storage.
    /// This should be called on startup to restore memory state.
    #[instrument(skip(self))]
    pub async fn load_all(&self, agent_id: &str) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = load_all(self.block_repo.as_ref(), agent_id).await?;
        Ok(entries)
    }

    #[instrument(skip(self, entry))]
    pub async fn store(&self, entry: MemoryEntry) -> Result<DomainUuid, MemoryError> {
        let existing = find_by_key(
            self.block_repo.as_ref(),
            &entry.agent_id,
            &entry.context,
            &entry.content,
        )
        .await?;

        if let Some(mut existing_entry) = existing {
            existing_entry.importance = entry.importance;
            existing_entry.decay_rate = entry.decay_rate;
            existing_entry.last_accessed = chrono::Utc::now();
            store::update(self.block_repo.as_ref(), &existing_entry).await?;
            Ok(existing_entry.id)
        } else {
            store::store(self.block_repo.as_ref(), &entry).await?;
            Ok(entry.id)
        }
    }

    #[instrument(skip(self, pattern))]
    pub async fn record_thinking_pattern(
        &self,
        agent_id: &str,
        pattern: ThinkingPattern,
    ) -> Result<DomainUuid, MemoryError> {
        let entry = MemoryEntry {
            id: DomainUuid::new_v4(),
            agent_id: agent_id.to_string(),
            context: pattern.domain.clone(),
            content: serde_json::to_string(&pattern)?,
            importance: 0.8,
            decay_rate: 0.05,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        };
        self.store(entry).await
    }

    #[instrument(skip(self))]
    pub async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, MemoryError> {
        let search_term = format!("agent-memory/{}*", query.context.as_deref().unwrap_or(""));
        let blocks = self
            .block_repo
            .search(&search_term, query.limit.max(100))
            .await?;

        let mut entries = Vec::new();

        for block in blocks {
            if let Some(PropertyValue::String(block_agent_id)) = block.properties.get("agent_id") {
                if block_agent_id.as_str() != query.agent_id {
                    continue;
                }
            } else {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(
                &serde_json::to_string(&block.content).unwrap_or_default(),
            ) {
                if let Some(ref q) = query.query {
                    if !entry.content.to_lowercase().contains(&q.to_lowercase()) {
                        continue;
                    }
                }

                let mut mutable_entry = entry.clone();
                mutable_entry.last_accessed = chrono::Utc::now();
                let _ = store::update(self.block_repo.as_ref(), &mutable_entry).await;

                entries.push(entry);
            }
        }

        entries.sort_by(|a, b| {
            b.relevance_score()
                .partial_cmp(&a.relevance_score())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        entries.truncate(query.limit);
        Ok(entries)
    }

    #[instrument(skip(self))]
    pub async fn decay(&self, agent_id: &str) -> Result<u32, MemoryError> {
        let blocks = self.block_repo.search("agent-memory/*", 1000).await?;

        let mut updated_count = 0u32;

        for block in blocks {
            if let Some(PropertyValue::String(block_agent_id)) = block.properties.get("agent_id") {
                if block_agent_id.as_str() != agent_id {
                    continue;
                }
            } else {
                continue;
            }

            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(
                &serde_json::to_string(&block.content).unwrap_or_default(),
            ) {
                let score = entry.relevance_score();
                if score < 0.1 {
                    let mut stale_entry = entry.clone();
                    stale_entry.last_accessed = chrono::Utc::now() - chrono::Duration::days(365);
                    let _ = store::update(self.block_repo.as_ref(), &stale_entry).await;
                    updated_count += 1;
                }
            }
        }

        Ok(updated_count)
    }

    #[instrument(skip(self, profile))]
    pub async fn update_profile(
        &self,
        agent_id: &str,
        profile: InteractionProfile,
    ) -> Result<(), MemoryError> {
        let page_id = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut h = DefaultHasher::new();
            agent_id.hash(&mut h);
            let hash = h.finish();
            let mut bytes = [0u8; 16];
            bytes[0..8].copy_from_slice(&hash.to_be_bytes());
            DomainUuid::from_bytes(bytes)
        };

        let content = serde_json::to_string(&profile)?;
        let mut properties = HashMap::new();
        properties.insert(
            "agent_id".to_string(),
            PropertyValue::String(agent_id.to_string()),
        );
        properties.insert(
            "observation_type".to_string(),
            PropertyValue::String("interaction_profile".to_string()),
        );

        let now = chrono::Utc::now();
        let block = Block {
            id: DomainUuid::new_v4(),
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: BlockContent::from_text(content),
            properties,
            refs: Vec::new(),
            tags: vec!["agent-memory".to_string(), "profile".to_string()],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: now,
            updated_at: now,
            journal_day: None,
            updated_journal_day: None,
        };

        self.block_repo.insert(&block).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_profile(
        &self,
        agent_id: &str,
    ) -> Result<Option<InteractionProfile>, MemoryError> {
        let blocks = self
            .block_repo
            .search(
                &format!("agent_id:{} observation_type:interaction_profile", agent_id),
                1,
            )
            .await?;

        for block in blocks {
            let content_str = serde_json::to_string(&block.content).ok();
            if let Some(content_str) = content_str {
                if let Ok(profile) = serde_json::from_str::<InteractionProfile>(&content_str) {
                    return Ok(Some(profile));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_memory::types::{
        CognitiveBias, InteractionProfile, MemoryQuery, ThinkingPattern,
    };
    use crate::ai_client::MockAIClient;
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::JournalDay;
    use quilt_domain::Uuid;
    use std::collections::HashMap;

    #[allow(dead_code)]
    fn uuid_from_u8(i: u8) -> DomainUuid {
        let mut b = [0u8; 16];
        b[0] = i;
        DomainUuid::from_bytes(b)
    }

    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct MockBlockRepo {
        blocks: Mutex<HashMap<DomainUuid, Block>>,
    }

    impl Clone for MockBlockRepo {
        fn clone(&self) -> Self {
            Self {
                blocks: Mutex::new(self.blocks.lock().unwrap().clone()),
            }
        }
    }

    #[async_trait]
    impl BlockRepository for MockBlockRepo {
        async fn get_by_id(&self, _id: DomainUuid) -> Result<Option<Block>, DomainError> {
            Ok(self.blocks.lock().unwrap().values().next().cloned())
        }
        async fn get_by_page(&self, _page_id: DomainUuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_children(&self, _parent_id: DomainUuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_with_refs(
            &self,
            _id: DomainUuid,
        ) -> Result<(Block, Vec<DomainUuid>), DomainError> {
            Err(DomainError::NotImplemented(
                "get_with_refs not implemented in mock",
            ))
        }
        async fn insert(&self, block: &Block) -> Result<(), DomainError> {
            self.blocks.lock().unwrap().insert(block.id, block.clone());
            Ok(())
        }
        async fn update(&self, block: &Block) -> Result<(), DomainError> {
            self.blocks.lock().unwrap().insert(block.id, block.clone());
            Ok(())
        }
        async fn delete(&self, _id: DomainUuid) -> Result<(), DomainError> {
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
            _id: DomainUuid,
            _new_parent: Option<DomainUuid>,
            _new_order: f64,
        ) -> Result<(), DomainError> {
            Ok(())
        }
        async fn get_backlinks(&self, _block_id: DomainUuid) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Block>, DomainError> {
            let results: Vec<Block> = self
                .blocks
                .lock()
                .unwrap()
                .values()
                .filter(|b| {
                    b.content.contains("agent-memory")
                        || b.tags.iter().any(|t| t.contains("agent-memory"))
                })
                .cloned()
                .collect();
            Ok(results)
        }
        async fn get_updated_since(
            &self,
            _since: chrono::DateTime<chrono::Utc>,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(self.blocks.lock().unwrap().values().cloned().collect())
        }
        async fn count_by_page(&self, _page_id: DomainUuid) -> Result<usize, DomainError> {
            Ok(0)
        }
        async fn get_blocks_by_journal_day(
            &self,
            _day: JournalDay,
        ) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
        async fn get_orphan_blocks(&self) -> Result<Vec<Block>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let memory = AgentMemory::new(repo.clone(), ai);

        let entry = MemoryEntry {
            id: DomainUuid::new_v4(),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Ownership is powerful".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
        };

        let id = memory.store(entry.clone()).await.unwrap();
        assert_eq!(id, entry.id);

        let query = MemoryQuery {
            agent_id: "agent-1".to_string(),
            context: Some("rust".to_string()),
            query: None,
            limit: 10,
        };
        let results = memory.retrieve(query).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_retrieve_filters_by_agent() {
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let memory = AgentMemory::new(repo, ai);

        let query = MemoryQuery {
            agent_id: "agent-1".to_string(),
            context: None,
            query: None,
            limit: 10,
        };
        let results = memory.retrieve(query).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_update_profile() {
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let memory = AgentMemory::new(repo, ai);

        let profile = InteractionProfile {
            thinking_pattern: ThinkingPattern {
                domain: "rust".to_string(),
                preferred_structure: "hierarchical".to_string(),
                abstraction_level: 0.7,
                topic_affinities: vec!["concurrency".to_string(), "memory".to_string()],
            },
            cognitive_biases: vec![CognitiveBias {
                bias_type: "confirmation".to_string(),
                description: "Prefers evidence confirming prior beliefs".to_string(),
                strength: 0.6,
            }],
            knowledge_levels: vec![("rust".to_string(), 0.8)].into_iter().collect(),
        };

        memory.update_profile("agent-1", profile).await.unwrap();

        let retrieved = memory.get_profile("agent-1").await.unwrap();
        assert!(retrieved.is_some());
        let p = retrieved.unwrap();
        assert_eq!(p.thinking_pattern.domain, "rust");
        assert_eq!(p.cognitive_biases.len(), 1);
    }

    #[tokio::test]
    async fn test_decay_returns_count() {
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let memory = AgentMemory::new(repo, ai);

        let count = memory.decay("agent-1").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_record_thinking_pattern() {
        let repo = Arc::new(MockBlockRepo::default());
        let ai = Arc::new(MockAIClient::new());
        let memory = AgentMemory::new(repo, ai);

        let pattern = ThinkingPattern {
            domain: "ml".to_string(),
            preferred_structure: "layered".to_string(),
            abstraction_level: 0.6,
            topic_affinities: vec!["neural-networks".to_string()],
        };

        let id = memory
            .record_thinking_pattern("agent-1", pattern)
            .await
            .unwrap();
        assert_ne!(id, DomainUuid::nil());
    }
}
