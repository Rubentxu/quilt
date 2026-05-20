//! Block-based Storage for Agent Memory

use crate::agent_memory::types::MemoryEntry;
use quilt_domain::entities::Block;
use quilt_domain::repositories::BlockRepository;
use quilt_domain::value_objects::{BlockFormat, PropertyValue, Uuid as DomainUuid};
use std::collections::HashMap;
use thiserror::Error;
use tracing::instrument;

/// Namespace prefix for agent memory pages
pub const AGENT_MEMORY_NAMESPACE: &str = "agent-memory";

/// Errors for agent memory storage operations
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Failed to serialize memory: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Repository error: {0}")]
    Repository(#[from] quilt_domain::errors::DomainError),
    #[error("Block not found: {0}")]
    BlockNotFound(DomainUuid),
}

#[allow(dead_code)]
pub fn domain_from_page(page_name: &str) -> Option<String> {
    page_name
        .strip_prefix(AGENT_MEMORY_NAMESPACE)
        .map(|s| s.trim_start_matches('/').to_string())
}

fn storage_key(entry: &MemoryEntry) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    entry.agent_id.hash(&mut hasher);
    entry.context.hash(&mut hasher);
    entry.content.hash(&mut hasher);
    format!("{}-{}-{}", entry.agent_id, entry.context, hasher.finish())
}

fn entry_to_block(entry: &MemoryEntry) -> Result<Block, StoreError> {
    use quilt_domain::content::BlockContent;
    let content = BlockContent::from_text(serde_json::to_string(entry)?);
    let _page_name = format!("{}/{}", AGENT_MEMORY_NAMESPACE, entry.context);

    let page_id = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        entry.agent_id.hash(&mut hasher);
        entry.context.hash(&mut hasher);
        let h = hasher.finish();
        let mut bytes = [0u8; 16];
        bytes[0..8].copy_from_slice(&h.to_be_bytes());
        DomainUuid::from_bytes(bytes)
    };

    let mut properties = HashMap::new();
    properties.insert(
        "agent_id".to_string(),
        PropertyValue::String(entry.agent_id.clone()),
    );
    properties.insert(
        "observation_type".to_string(),
        PropertyValue::String("memory_entry".to_string()),
    );
    properties.insert(
        "importance".to_string(),
        PropertyValue::Float(entry.importance as f64),
    );
    properties.insert(
        "decay_rate".to_string(),
        PropertyValue::Float(entry.decay_rate as f64),
    );
    properties.insert(
        "storage_key".to_string(),
        PropertyValue::String(storage_key(entry)),
    );

    let now = chrono::Utc::now();
    Ok(Block {
        id: entry.id,
        page_id,
        parent_id: None,
        order: 1.0,
        level: 1,
        format: BlockFormat::Markdown,
        marker: None,
        priority: None,
        content,
        properties,
        refs: Vec::new(),
        tags: vec!["agent-memory".to_string()],
        scheduled: None,
        deadline: None,
        start_time: None,
        repeated: None,
        logbook: None,
        collapsed: false,
        created_at: entry.created_at,
        updated_at: now,
        journal_day: None,
        updated_journal_day: None,
    })
}

fn block_to_entry(block: &Block) -> Result<MemoryEntry, StoreError> {
    let content_str = serde_json::to_string(&block.content)?;
    let entry: MemoryEntry = serde_json::from_str(&content_str)?;
    Ok(entry)
}

#[instrument(skip(repo, entry))]
pub async fn store(
    repo: &dyn BlockRepository,
    entry: &MemoryEntry,
) -> Result<DomainUuid, StoreError> {
    let block = entry_to_block(entry)?;
    repo.insert(&block).await?;
    Ok(entry.id)
}

#[instrument(skip(repo, entry))]
pub async fn update(
    repo: &dyn BlockRepository,
    entry: &MemoryEntry,
) -> Result<DomainUuid, StoreError> {
    let block = entry_to_block(entry)?;
    repo.update(&block).await?;
    Ok(entry.id)
}

/// Load all memory entries for a specific agent from the repository.
/// This is used to restore memory on startup.
#[instrument(skip(repo))]
pub async fn load_all(
    repo: &dyn BlockRepository,
    agent_id: &str,
) -> Result<Vec<MemoryEntry>, StoreError> {
    let search_term = format!("agent_id:{} observation_type:memory_entry", agent_id);
    let blocks = repo.search(&search_term, 1000).await?;

    let mut entries = Vec::new();
    for block in blocks {
        if let Ok(entry) = block_to_entry(&block) {
            entries.push(entry);
        }
    }

    // Sort by relevance score descending
    entries.sort_by(|a, b| {
        b.relevance_score()
            .partial_cmp(&a.relevance_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(entries)
}

pub async fn find_by_key(
    repo: &dyn BlockRepository,
    agent_id: &str,
    context: &str,
    content: &str,
) -> Result<Option<MemoryEntry>, StoreError> {
    let temp_entry = MemoryEntry {
        id: DomainUuid::nil(),
        agent_id: agent_id.to_string(),
        context: context.to_string(),
        content: content.to_string(),
        importance: 0.0,
        decay_rate: 0.0,
        created_at: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
    };
    let key = storage_key(&temp_entry);

    let blocks = repo
        .search(&format!("agent_id:{} storage_key:{}", agent_id, key), 10)
        .await?;

    for block in blocks {
        if let Ok(found) = block_to_entry(&block) {
            return Ok(Some(found));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid_from_u8(i: u8) -> DomainUuid {
        let mut b = [0u8; 16];
        b[0] = i;
        DomainUuid::from_bytes(b)
    }

    #[test]
    fn test_domain_from_page() {
        assert_eq!(
            domain_from_page("agent-memory/rust"),
            Some("rust".to_string())
        );
        assert_eq!(domain_from_page("agent-memory/"), Some("".to_string()));
        // "agent-memory" alone returns Some("") since strip yields empty string
        assert_eq!(domain_from_page("agent-memory"), Some("".to_string()));
        assert_eq!(domain_from_page("other/page"), None);
    }

    #[test]
    fn test_entry_to_block_roundtrip() {
        let now = chrono::Utc::now();
        let entry = MemoryEntry {
            id: uuid_from_u8(42),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Rust is fast".to_string(),
            importance: 0.85,
            decay_rate: 0.05,
            created_at: now,
            last_accessed: now,
        };

        let block = entry_to_block(&entry).unwrap();
        assert!(block.content.contains("Rust is fast"));
        assert_eq!(block.tags, vec!["agent-memory"]);

        let recovered = block_to_entry(&block).unwrap();
        assert_eq!(recovered.agent_id, "agent-1");
        assert_eq!(recovered.context, "rust");
        assert_eq!(recovered.importance, 0.85);
    }

    #[test]
    fn test_storage_key_deterministic() {
        let now = chrono::Utc::now();
        let entry1 = MemoryEntry {
            id: uuid_from_u8(1),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Same content".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: now,
            last_accessed: now,
        };
        let entry2 = MemoryEntry {
            id: uuid_from_u8(2),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Same content".to_string(),
            importance: 0.5,
            decay_rate: 0.1,
            created_at: now,
            last_accessed: now,
        };
        let key1 = storage_key(&entry1);
        let key2 = storage_key(&entry2);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_storage_key_differs_with_content() {
        let now = chrono::Utc::now();
        let entry1 = MemoryEntry {
            id: uuid_from_u8(1),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Content A".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: now,
            last_accessed: now,
        };
        let entry2 = MemoryEntry {
            id: uuid_from_u8(2),
            agent_id: "agent-1".to_string(),
            context: "rust".to_string(),
            content: "Content B".to_string(),
            importance: 0.9,
            decay_rate: 0.05,
            created_at: now,
            last_accessed: now,
        };
        assert_ne!(storage_key(&entry1), storage_key(&entry2));
    }
}
