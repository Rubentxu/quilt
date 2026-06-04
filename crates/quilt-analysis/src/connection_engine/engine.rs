use crate::AnalysisError;
use crate::connection_engine::types::{
    ConnectionType, SerendipityConnection, SerendipityOptions, SerendipityQuery,
};
use lru::LruCache;
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::Uuid;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::instrument;

pub type ConnectionError = AnalysisError;

fn uuid_lt(a: &Uuid, b: &Uuid) -> bool {
    a.as_bytes() < b.as_bytes()
}

pub fn structural_similarity(a: &Block, b: &Block) -> f32 {
    let refs_a: HashSet<&Uuid> = a.refs.iter().collect();
    let refs_b: HashSet<&Uuid> = b.refs.iter().collect();
    if refs_a.is_empty() && refs_b.is_empty() {
        return 0.0;
    }
    let intersection = refs_a.intersection(&refs_b).count();
    let union = refs_a.union(&refs_b).count();
    intersection as f32 / union as f32
}

pub fn temporal_proximity(a: &Block, b: &Block, halflife_days: f64) -> f32 {
    let diff = (a.created_at - b.created_at).num_seconds().abs() as f64;
    let halflife_secs = halflife_days * 24.0 * 3600.0;
    let ratio = diff / halflife_secs;
    let proximity = 0.5_f64.powf(ratio);
    proximity.clamp(0.0, 1.0) as f32
}

pub fn composite_score(structural: f32, temporal: f32) -> f32 {
    0.6 * structural + 0.4 * temporal
}

type CacheEntry = (Vec<SerendipityConnection>, std::time::Instant);

fn make_cache_key(query: &SerendipityQuery, block_ids: &[Uuid]) -> String {
    let ids_str: String = block_ids.iter().map(|u| u.to_string()).collect();
    format!(
        "{}|{}|{}|{}|{:?}",
        query.limit, query.offset, query.min_confidence, ids_str, query.temporal_window_days
    )
}

#[derive(Clone)]
pub struct TimedCache {
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
    ttl_secs: u64,
}

impl std::fmt::Debug for TimedCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimedCache")
            .field("ttl_secs", &self.ttl_secs)
            .finish()
    }
}

impl TimedCache {
    pub fn new(cap: usize, ttl_secs: u64) -> Self {
        let cap = NonZeroUsize::new(cap).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(cap))),
            ttl_secs,
        }
    }

    async fn get(&self, key: &str) -> Option<Vec<SerendipityConnection>> {
        let mut guard = self.cache.lock().await;
        if let Some((conns, instant)) = guard.get(key) {
            if instant.elapsed().as_secs() < self.ttl_secs {
                return Some(conns.clone());
            }
            guard.pop(key);
        }
        None
    }

    async fn insert(&self, key: String, conns: Vec<SerendipityConnection>) {
        let mut guard = self.cache.lock().await;
        guard.put(key, (conns, std::time::Instant::now()));
    }
}

#[derive(Clone)]
pub struct ConnectionEngine {
    block_repo: Arc<dyn BlockRepository>,
    cache: TimedCache,
    options: SerendipityOptions,
}

impl std::fmt::Debug for ConnectionEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnectionEngine")
            .field("block_repo", &"Arc<dyn BlockRepository>")
            .field("cache", &self.cache)
            .finish()
    }
}

impl ConnectionEngine {
    pub fn new(block_repo: Arc<dyn BlockRepository>) -> Self {
        Self {
            block_repo,
            cache: TimedCache::new(100, 300),
            options: SerendipityOptions::default(),
        }
    }

    pub fn with_options(block_repo: Arc<dyn BlockRepository>, options: SerendipityOptions) -> Self {
        Self {
            block_repo,
            cache: TimedCache::new(100, options.cache_ttl_secs),
            options,
        }
    }

    #[instrument(skip(self))]
    pub async fn find_connections(
        &self,
        query: SerendipityQuery,
    ) -> Result<Vec<SerendipityConnection>, ConnectionError> {
        let blocks = match query.temporal_window_days {
            Some(days) => {
                let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
                self.block_repo.get_updated_since(cutoff).await?
            }
            None => {
                let page_id = query.page_id.ok_or_else(|| {
                    AnalysisError::Validation(
                        "page_id is required when temporal_window_days is None".into(),
                    )
                })?;
                self.block_repo.get_by_page(page_id).await?
            }
        };

        let block_ids: Vec<Uuid> = blocks.iter().map(|b| b.id).collect();
        let cache_key = make_cache_key(&query, &block_ids);

        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        let direct_refs: HashSet<(Uuid, Uuid)> = blocks
            .iter()
            .flat_map(|b| {
                b.refs.iter().map(move |&r| {
                    if uuid_lt(&b.id, &r) {
                        (b.id, r)
                    } else {
                        (r, b.id)
                    }
                })
            })
            .collect();

        let mut connections = Vec::new();

        for (i, block_a) in blocks.iter().enumerate() {
            for block_b in blocks.iter().skip(i + 1) {
                let (lo, hi) = if uuid_lt(&block_a.id, &block_b.id) {
                    (block_a.id, block_b.id)
                } else {
                    (block_b.id, block_a.id)
                };
                if direct_refs.contains(&(lo, hi)) {
                    continue;
                }

                let structural = structural_similarity(block_a, block_b);
                let temporal = temporal_proximity(block_a, block_b, self.options.halflife_days);
                let composite = composite_score(structural, temporal);

                if composite < query.min_confidence {
                    continue;
                }

                let conn_type = if structural > temporal {
                    ConnectionType::Structural
                } else {
                    ConnectionType::Temporal
                };

                connections.push(SerendipityConnection {
                    idea_a: block_a.id,
                    idea_b: block_b.id,
                    bridge_concept: None,
                    confidence: composite,
                    explanation: format_serendipity_explanation(structural, temporal),
                    connection_type: conn_type,
                });
            }
        }

        connections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let total = connections.len();
        let offset = query.offset.min(total);
        let limit = query.limit.min(total - offset);
        let paginated: Vec<SerendipityConnection> =
            connections.into_iter().skip(offset).take(limit).collect();
        self.cache.insert(cache_key, paginated.clone()).await;
        Ok(paginated)
    }
}

fn format_serendipity_explanation(structural: f32, temporal: f32) -> String {
    let parts: Vec<&str> = vec![
        if structural > 0.3 {
            "shared references"
        } else {
            ""
        },
        if temporal > 0.5 {
            "temporal proximity"
        } else {
            ""
        },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect();

    if parts.is_empty() {
        "unexpected connection discovered".to_string()
    } else {
        format!("connected via {}", parts.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection_engine::types::SerendipityQuery;
    use async_trait::async_trait;
    use quilt_domain::errors::DomainError;
    use quilt_domain::repositories::BlockRepository;
    use quilt_domain::value_objects::BlockFormat;
    use std::collections::HashMap;

    fn uuid_from_u8(i: u8) -> Uuid {
        let mut b = [0u8; 16];
        b[0] = i;
        Uuid::from_bytes(b)
    }

    fn make_block(id: Uuid, refs: Vec<Uuid>, page_id: Uuid) -> Block {
        Block {
            id,
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: format!("Block {}", id),
            properties: HashMap::new(),
            refs,
            tags: Vec::new(),
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
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
        async fn get_by_page(&self, page_id: Uuid) -> Result<Vec<Block>, DomainError> {
            Ok(self
                .blocks
                .iter()
                .filter(|b| b.page_id.as_bytes() == page_id.as_bytes())
                .cloned()
                .collect())
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

    #[test]
    fn test_jaccard_partial_overlap() {
        let page_id = uuid_from_u8(1);
        let a = make_block(
            uuid_from_u8(10),
            vec![uuid_from_u8(20), uuid_from_u8(21), uuid_from_u8(22)],
            page_id,
        );
        let b = make_block(
            uuid_from_u8(11),
            vec![uuid_from_u8(20), uuid_from_u8(21), uuid_from_u8(23)],
            page_id,
        );
        assert_eq!(structural_similarity(&a, &b), 0.5);
    }

    #[test]
    fn test_jaccard_no_overlap() {
        let page_id = uuid_from_u8(1);
        let a = make_block(uuid_from_u8(10), vec![uuid_from_u8(20)], page_id);
        let b = make_block(uuid_from_u8(11), vec![uuid_from_u8(21)], page_id);
        assert_eq!(structural_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_both_empty() {
        let page_id = uuid_from_u8(1);
        let a = make_block(uuid_from_u8(10), vec![], page_id);
        let b = make_block(uuid_from_u8(11), vec![], page_id);
        assert_eq!(structural_similarity(&a, &b), 0.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let page_id = uuid_from_u8(1);
        let refs = vec![uuid_from_u8(20), uuid_from_u8(21)];
        let a = make_block(uuid_from_u8(10), refs.clone(), page_id);
        let b = make_block(uuid_from_u8(11), refs, page_id);
        assert_eq!(structural_similarity(&a, &b), 1.0);
    }

    #[test]
    fn test_temporal_same_timestamp() {
        let page_id = uuid_from_u8(1);
        let a = make_block(uuid_from_u8(10), vec![], page_id);
        let b = make_block(uuid_from_u8(11), vec![], page_id);
        let prox = temporal_proximity(&a, &b, 7.0);
        assert!((prox - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_temporal_halflife_apart() {
        let page_id = uuid_from_u8(1);
        let base_time = chrono::Utc::now();
        let a = make_block(uuid_from_u8(10), vec![], page_id);
        let seven_days_later = base_time + chrono::Duration::days(7);
        let b = Block {
            id: uuid_from_u8(11),
            page_id,
            parent_id: None,
            order: 1.0,
            level: 1,
            format: BlockFormat::Markdown,
            marker: None,
            priority: None,
            content: "B".to_string(),
            properties: HashMap::new(),
            refs: vec![],
            tags: vec![],
            scheduled: None,
            deadline: None,
            start_time: None,
            repeated: None,
            logbook: None,
            collapsed: false,
            created_at: seven_days_later,
            updated_at: seven_days_later,
        };
        let prox = temporal_proximity(&a, &b, 7.0);
        assert!((prox - 0.5).abs() < 0.05);
    }

    #[test]
    fn test_composite_score() {
        assert_eq!(composite_score(0.5, 0.5), 0.5);
    }

    #[test]
    fn test_composite_score_structural_heavy() {
        assert_eq!(composite_score(1.0, 0.0), 0.6);
    }

    #[tokio::test]
    async fn test_find_connections_excludes_direct_refs() {
        let page_id = uuid_from_u8(1);
        let a = uuid_from_u8(10);
        let b = uuid_from_u8(11);
        let blocks = vec![
            make_block(a, vec![b], page_id),
            make_block(b, vec![], page_id),
        ];
        let repo = Arc::new(MockBlockRepo { blocks });
        let engine = ConnectionEngine::new(repo);
        let query = SerendipityQuery {
            limit: 5,
            offset: 0,
            min_confidence: 0.0,
            temporal_window_days: None,
            topic: None,
            page_id: Some(page_id),
        };
        let results = engine.find_connections(query).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_find_connections_pagination() {
        let page_id = uuid_from_u8(1);
        let blocks: Vec<Block> = (0..10)
            .map(|i| make_block(uuid_from_u8(i), vec![], page_id))
            .collect();
        let repo = Arc::new(MockBlockRepo { blocks });
        let engine = ConnectionEngine::new(repo);
        let query = SerendipityQuery {
            limit: 5,
            offset: 0,
            min_confidence: 0.0,
            temporal_window_days: None,
            topic: None,
            page_id: Some(page_id),
        };
        let results = engine.find_connections(query).await.unwrap();
        assert!(results.len() <= 5);
    }

    #[tokio::test]
    async fn test_find_connections_min_confidence_filter() {
        let page_id = uuid_from_u8(1);
        let blocks: Vec<Block> = (0..3)
            .map(|i| make_block(uuid_from_u8(i), vec![], page_id))
            .collect();
        let repo = Arc::new(MockBlockRepo { blocks });
        let engine = ConnectionEngine::new(repo);
        let query = SerendipityQuery {
            limit: 20,
            offset: 0,
            min_confidence: 0.99,
            temporal_window_days: None,
            topic: None,
            page_id: Some(page_id),
        };
        let results = engine.find_connections(query).await.unwrap();
        assert!(results.is_empty());
    }
}
