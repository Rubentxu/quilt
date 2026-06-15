use crate::AnalysisError;
use crate::agent_store::store::{self, find_by_key};
use crate::agent_store::types::{InteractionProfile, MemoryEntry, MemoryQuery, ThinkingPattern};
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::{BlockFormat, BlockType, PropertyValue, Uuid as DomainUuid};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tracing::instrument;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Store error: {0}")]
    Store(#[from] store::StoreError),
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("Profile not found for agent: {0}")]
    ProfileNotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<StoreError> for AnalysisError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::Store(e) => e.into(),
            StoreError::Repository(e) => AnalysisError::Storage(e),
            StoreError::ProfileNotFound(e) => AnalysisError::Configuration(e),
            StoreError::Serialization(e) => AnalysisError::Serialization(e),
        }
    }
}

#[derive(Clone)]
pub struct AgentStore {
    block_repo: Arc<dyn BlockRepository>,
}

impl std::fmt::Debug for AgentStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentStore")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .finish()
    }
}

impl AgentStore {
    pub fn new(block_repo: Arc<dyn BlockRepository>) -> Self {
        Self { block_repo }
    }

    #[instrument(skip(self, entry))]
    pub async fn store(&self, entry: MemoryEntry) -> Result<DomainUuid, StoreError> {
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
    ) -> Result<DomainUuid, StoreError> {
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
    pub async fn retrieve(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>, StoreError> {
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

            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(&block.content) {
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
    pub async fn decay(&self, agent_id: &str) -> Result<u32, StoreError> {
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
            if let Ok(entry) = serde_json::from_str::<MemoryEntry>(&block.content) {
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
    ) -> Result<(), StoreError> {
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
            block_type: BlockType::Paragraph,
            marker: None,
            priority: None,
            content,
            properties,
            refs: Vec::new(),
            tags: vec!["agent-memory".to_string(), "profile".to_string()],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            completed_at: None,
            cancelled_at: None,
            created_at: now,
            updated_at: now,
        };
        self.block_repo.insert(&block).await?;
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_profile(
        &self,
        agent_id: &str,
    ) -> Result<Option<InteractionProfile>, StoreError> {
        let blocks = self
            .block_repo
            .search(
                &format!("agent_id:{} observation_type:interaction_profile", agent_id),
                1,
            )
            .await?;
        for block in blocks {
            if let Ok(profile) = serde_json::from_str::<InteractionProfile>(&block.content) {
                return Ok(Some(profile));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_store::types::{CognitiveBias, MemoryQuery, ThinkingPattern};
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use quilt_domain::repositories::BlockRepository;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn uuid_from_u8(i: u8) -> DomainUuid {
        let mut b = [0u8; 16];
        b[0] = i;
        DomainUuid::from_bytes(b)
    }

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

        async fn list_distinct_keys(
            &self,
            _cursor: Option<&str>,
            _limit: u32,
        ) -> Result<Vec<String>, DomainError> {
            Err(DomainError::Storage(
                "list_distinct_keys not supported in mock".to_string(),
            ))
        }

        async fn list_by_property_key(
            &self,
            _key: &str,
            _limit: u32,
        ) -> Result<Vec<Block>, DomainError> {
            Err(DomainError::Storage(
                "list_by_property_key not supported in mock".to_string(),
            ))
        }

        async fn list_distinct_authors(
            &self,
            _prefix: Option<&str>,
        ) -> Result<Vec<String>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve() {
        let repo = Arc::new(MockBlockRepo::default());
        let store = AgentStore::new(repo.clone());
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
        let id = store.store(entry.clone()).await.unwrap();
        assert_eq!(id, entry.id);
        let query = MemoryQuery {
            agent_id: "agent-1".to_string(),
            context: Some("rust".to_string()),
            query: None,
            limit: 10,
        };
        let results = store.retrieve(query).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_retrieve_filters_by_agent() {
        let repo = Arc::new(MockBlockRepo::default());
        let store = AgentStore::new(repo);
        let query = MemoryQuery {
            agent_id: "agent-1".to_string(),
            context: None,
            query: None,
            limit: 10,
        };
        let results = store.retrieve(query).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_update_profile() {
        let repo = Arc::new(MockBlockRepo::default());
        let store = AgentStore::new(repo);
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
        store.update_profile("agent-1", profile).await.unwrap();
        let retrieved = store.get_profile("agent-1").await.unwrap();
        assert!(retrieved.is_some());
        let p = retrieved.unwrap();
        assert_eq!(p.thinking_pattern.domain, "rust");
        assert_eq!(p.cognitive_biases.len(), 1);
    }

    #[tokio::test]
    async fn test_decay_returns_count() {
        let repo = Arc::new(MockBlockRepo::default());
        let store = AgentStore::new(repo);
        let count = store.decay("agent-1").await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_record_thinking_pattern() {
        let repo = Arc::new(MockBlockRepo::default());
        let store = AgentStore::new(repo);
        let pattern = ThinkingPattern {
            domain: "ml".to_string(),
            preferred_structure: "layered".to_string(),
            abstraction_level: 0.6,
            topic_affinities: vec!["neural-networks".to_string()],
        };
        let id = store
            .record_thinking_pattern("agent-1", pattern)
            .await
            .unwrap();
        assert_ne!(id, DomainUuid::nil());
    }
}
