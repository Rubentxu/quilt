//! Offline queue and WAL with SQLite persistence
//!
//! The `OfflineQueue` provides durable storage for pending sync operations
//! using SQLite. This ensures changes are not lost across restarts.
//!
//! # Schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS sync_changes (
//!     id BLOB PRIMARY KEY NOT NULL,
//!     entity_type TEXT NOT NULL,
//!     entity_id BLOB NOT NULL,
//!     operation TEXT NOT NULL,
//!     payload BLOB NOT NULL,
//!     timestamp INTEGER NOT NULL,
//!     peer_id BLOB NOT NULL,
//!     version INTEGER NOT NULL DEFAULT 0,
//!     synced INTEGER NOT NULL DEFAULT 0
//! );
//! CREATE INDEX IF NOT EXISTS idx_sync_changes_synced ON sync_changes(synced);
//! ```

use crate::crdt::SyncChange;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::VecDeque;
use tracing::instrument;
use uuid::Uuid;

/// Maximum number of synced entries before compaction runs
const DEFAULT_COMPACTION_THRESHOLD: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeRecord {
    pub id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub operation: Operation,
    pub payload: Vec<u8>,
    pub timestamp: i64,
    pub peer_id: Uuid,
    pub version: u64,
    pub synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Operation {
    Create,
    Update,
    Delete,
}

impl Operation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Operation::Create => "create",
            Operation::Update => "update",
            Operation::Delete => "delete",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "create" => Some(Operation::Create),
            "update" => Some(Operation::Update),
            "delete" => Some(Operation::Delete),
            _ => None,
        }
    }
}

/// SQLite-backed offline queue for sync changes
pub struct OfflineQueue {
    pool: Pool<Sqlite>,
    /// In-memory cache of pending (not-yet-synced) changes for fast access
    pending_cache: VecDeque<ChangeRecord>,
    compaction_threshold: usize,
}

impl OfflineQueue {
    /// Create a new OfflineQueue with the given SQLite connection pool.
    ///
    /// Runs migrations automatically and recovers pending changes from DB.
    #[instrument(skip(pool))]
    pub async fn new(pool: Pool<Sqlite>) -> Result<Self> {
        let mut queue = Self {
            pool,
            pending_cache: VecDeque::new(),
            compaction_threshold: DEFAULT_COMPACTION_THRESHOLD,
        };

        queue.run_migrations().await?;
        queue.recover_pending().await?;

        Ok(queue)
    }

    /// Create a new OfflineQueue with a custom compaction threshold.
    #[instrument(skip(pool))]
    pub async fn with_compaction_threshold(pool: Pool<Sqlite>, threshold: usize) -> Result<Self> {
        let mut queue = Self {
            pool,
            pending_cache: VecDeque::new(),
            compaction_threshold: threshold,
        };

        queue.run_migrations().await?;
        queue.recover_pending().await?;

        Ok(queue)
    }

    /// Run database migrations for the sync_changes table.
    async fn run_migrations(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sync_changes (
                id BLOB PRIMARY KEY NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id BLOB NOT NULL,
                operation TEXT NOT NULL,
                payload BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                peer_id BLOB NOT NULL,
                version INTEGER NOT NULL DEFAULT 0,
                synced INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sync_changes_synced ON sync_changes(synced)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Recover pending (non-synced) changes from the database.
    async fn recover_pending(&mut self) -> Result<()> {
        let rows = sqlx::query_as::<_, SyncChangeRow>(
            "SELECT id, entity_type, entity_id, operation, payload, timestamp, peer_id, version, synced FROM sync_changes WHERE synced = 0 ORDER BY timestamp ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            self.pending_cache.push_back(row.into_change_record());
        }

        Ok(())
    }

    /// Enqueue a new change record.
    #[instrument(skip(self, record))]
    pub async fn enqueue(&mut self, record: ChangeRecord) -> Result<()> {
        let row = SyncChangeRow::from_change_record(&record);
        sqlx::query(
            r#"
            INSERT INTO sync_changes (id, entity_type, entity_id, operation, payload, timestamp, peer_id, version, synced)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(row.id)
        .bind(&row.entity_type)
        .bind(row.entity_id)
        .bind(&row.operation)
        .bind(&row.payload)
        .bind(row.timestamp)
        .bind(row.peer_id)
        .bind(row.version)
        .bind(row.synced)
        .execute(&self.pool)
        .await?;

        self.pending_cache.push_back(record);
        Ok(())
    }

    /// Dequeue the next pending change.
    #[instrument(skip(self))]
    pub async fn dequeue(&mut self) -> Result<Option<ChangeRecord>> {
        // Remove from DB
        let _result = sqlx::query(
            "DELETE FROM sync_changes WHERE id = (SELECT id FROM sync_changes WHERE synced = 0 ORDER BY timestamp ASC LIMIT 1) RETURNING id",
        )
        .fetch_optional(&self.pool)
        .await?;

        // Remove from cache
        if let Some(record) = self.pending_cache.pop_front() {
            return Ok(Some(record));
        }

        Ok(None)
    }

    /// Mark a specific change as synced by ID.
    #[instrument(skip(self))]
    pub async fn mark_synced(&mut self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE sync_changes SET synced = 1 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        // Update in-memory cache
        if let Some(record) = self.pending_cache.iter_mut().find(|r| r.id == id) {
            record.synced = true;
        }

        // Check if compaction is needed
        if self.should_compact().await? {
            self.compact().await?;
        }

        Ok(())
    }

    /// Get all pending (non-synced) changes.
    pub fn pending(&self) -> Vec<&ChangeRecord> {
        self.pending_cache.iter().filter(|r| !r.synced).collect()
    }

    /// Get count of pending changes.
    pub fn pending_count(&self) -> usize {
        self.pending_cache.iter().filter(|r| !r.synced).count()
    }

    /// Clear all synced entries from the database.
    #[instrument(skip(self))]
    pub async fn clear_synced(&mut self) -> Result<()> {
        sqlx::query("DELETE FROM sync_changes WHERE synced = 1")
            .execute(&self.pool)
            .await?;

        self.pending_cache.retain(|r| !r.synced);
        Ok(())
    }

    /// Check if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.pending_cache.is_empty()
    }

    /// Check if compaction should run based on synced entry count.
    async fn should_compact(&self) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sync_changes WHERE synced = 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(count.0 >= self.compaction_threshold as i64)
    }

    /// Run compaction to purge synced entries.
    async fn compact(&mut self) -> Result<()> {
        let deleted = sqlx::query("DELETE FROM sync_changes WHERE synced = 1")
            .execute(&self.pool)
            .await?
            .rows_affected();

        tracing::debug!("Compaction removed {} synced entries", deleted);
        Ok(())
    }

    /// Force compaction regardless of threshold.
    #[instrument(skip(self))]
    pub async fn force_compact(&mut self) -> Result<()> {
        self.compact().await
    }
}

/// Database row representation for sync_changes
#[derive(Debug, sqlx::FromRow)]
struct SyncChangeRow {
    id: Vec<u8>,
    entity_type: String,
    entity_id: Vec<u8>,
    operation: String,
    payload: Vec<u8>,
    timestamp: i64,
    peer_id: Vec<u8>,
    version: i64,
    synced: i32,
}

impl SyncChangeRow {
    fn from_change_record(record: &ChangeRecord) -> Self {
        Self {
            id: record.id.as_bytes().to_vec(),
            entity_type: record.entity_type.clone(),
            entity_id: record.entity_id.as_bytes().to_vec(),
            operation: record.operation.as_str().to_string(),
            payload: record.payload.clone(),
            timestamp: record.timestamp,
            peer_id: record.peer_id.as_bytes().to_vec(),
            version: record.version as i64,
            synced: if record.synced { 1 } else { 0 },
        }
    }

    fn into_change_record(self) -> ChangeRecord {
        ChangeRecord {
            id: Uuid::from_slice(&self.id).unwrap_or_default(),
            entity_type: self.entity_type,
            entity_id: Uuid::from_slice(&self.entity_id).unwrap_or_default(),
            operation: Operation::from_str(&self.operation).unwrap_or(Operation::Update),
            payload: self.payload,
            timestamp: self.timestamp,
            peer_id: Uuid::from_slice(&self.peer_id).unwrap_or_default(),
            version: self.version as u64,
            synced: self.synced != 0,
        }
    }
}

impl TryFrom<SyncChange> for ChangeRecord {
    type Error = anyhow::Error;

    fn try_from(change: SyncChange) -> Result<Self> {
        Ok(Self {
            id: Uuid::new_v4(),
            entity_type: change.entity_type,
            entity_id: change.entity_id,
            operation: Operation::Update, // Could infer from context
            payload: change.data,
            timestamp: change.timestamp,
            peer_id: change.peer_id,
            version: change.version,
            synced: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_offline_queue_enqueue_and_dequeue() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let mut queue = OfflineQueue::new(pool).await.unwrap();

        let record = ChangeRecord {
            id: Uuid::new_v4(),
            entity_type: "block".to_string(),
            entity_id: Uuid::new_v4(),
            operation: Operation::Create,
            payload: b"test payload".to_vec(),
            timestamp: chrono::Utc::now().timestamp(),
            peer_id: Uuid::new_v4(),
            version: 1,
            synced: false,
        };

        queue.enqueue(record.clone()).await.unwrap();
        assert_eq!(queue.pending_count(), 1);

        let dequeued = queue.dequeue().await.unwrap();
        assert!(dequeued.is_some());
        assert_eq!(queue.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_offline_queue_mark_synced() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let mut queue = OfflineQueue::new(pool).await.unwrap();

        let record = ChangeRecord {
            id: Uuid::new_v4(),
            entity_type: "block".to_string(),
            entity_id: Uuid::new_v4(),
            operation: Operation::Update,
            payload: b"test".to_vec(),
            timestamp: chrono::Utc::now().timestamp(),
            peer_id: Uuid::new_v4(),
            version: 1,
            synced: false,
        };

        queue.enqueue(record.clone()).await.unwrap();
        queue.mark_synced(record.id).await.unwrap();

        // After marking synced, it should be compacted away eventually
        // For now, just verify the record is marked
        let pending = queue.pending();
        assert!(pending.is_empty() || pending.iter().all(|r| r.id != record.id));
    }

    #[tokio::test]
    async fn test_offline_queue_recovery() {
        // Use a temp file for in-memory-like but persistent across pools
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_recovery.db");

        let pool = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();
        let mut queue1 = OfflineQueue::new(pool).await.unwrap();

        let record = ChangeRecord {
            id: Uuid::new_v4(),
            entity_type: "page".to_string(),
            entity_id: Uuid::new_v4(),
            operation: Operation::Create,
            payload: b"recovery test".to_vec(),
            timestamp: chrono::Utc::now().timestamp(),
            peer_id: Uuid::new_v4(),
            version: 1,
            synced: false,
        };

        queue1.enqueue(record.clone()).await.unwrap();

        // Simulate restart by creating new queue from same database file
        let pool2 = sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
            .await
            .unwrap();
        let queue2 = OfflineQueue::new(pool2).await.unwrap();

        // New queue should recover the pending change
        assert_eq!(queue2.pending_count(), 1);
        assert_eq!(queue2.pending()[0].entity_type, "page");
    }

    #[tokio::test]
    async fn test_operation_serialization() {
        assert_eq!(Operation::Create.as_str(), "create");
        assert_eq!(Operation::Update.as_str(), "update");
        assert_eq!(Operation::Delete.as_str(), "delete");

        assert_eq!(Operation::from_str("create"), Some(Operation::Create));
        assert_eq!(Operation::from_str("delete"), Some(Operation::Delete));
        assert_eq!(Operation::from_str("unknown"), None);
    }
}
