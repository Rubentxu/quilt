//! CRDT engine — Last-Writer-Wins with version vectors
//!
//! Provides conflict-free sync by tracking monotonic versions per entity
//! and resolving conflicts via timestamp + peer_id tiebreaker.
//!
//! # Conflict Resolution
//!
//! The engine supports multiple conflict resolution strategies:
//! - [`ConflictStrategy::LastWriteWins`]: Default LWW behavior
//! - [`ConflictStrategy::PreserveBoth`]: Creates conflict markers for manual resolution
//! - [`ConflictStrategy::Manual`]: Defers to user handler

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

/// Metadata for a single entity version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: u64,
    pub last_writer: Uuid,
    pub timestamp: i64,
}

/// A syncable change that can be applied or rejected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncChange {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub data: Vec<u8>,
    pub version: u64,
    pub peer_id: Uuid,
    pub timestamp: i64,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Last-Writer-Wins: highest timestamp (or peer_id tiebreaker) wins
    #[default]
    LastWriteWins,
    /// Preserve both versions, marking as conflict for manual resolution
    PreserveBoth,
    /// Manual resolution: changes are stored but not auto-applied
    Manual,
}

/// Conflict resolution error types
#[derive(Debug, Error, Clone)]
pub enum ConflictError {
    #[error("Concurrent modification detected for entity {entity_id}")]
    ConcurrentModification { entity_id: Uuid },

    #[error("Manual resolution required for entity {entity_id}")]
    RequiresManualResolution { entity_id: Uuid },

    #[error("Conflict marker creation failed: {reason}")]
    MarkerCreationFailed { reason: String },
}

/// Result of conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub entity_id: Uuid,
    pub winning_change: Option<SyncChange>,
    pub losing_change: Option<SyncChange>,
    pub conflict_marker: Option<Vec<u8>>,
    pub strategy_used: ConflictStrategy,
}

/// Conflict resolver trait - implement this to provide custom conflict resolution
pub trait ConflictResolver: Send + Sync {
    /// Resolve a conflict between local and remote changes.
    fn resolve(
        &self,
        local: Option<&SyncChange>,
        remote: &SyncChange,
        strategy: ConflictStrategy,
    ) -> ConflictResolution;

    /// Create a conflict marker that preserves both versions.
    fn create_conflict_marker(
        &self,
        local: &SyncChange,
        remote: &SyncChange,
    ) -> Result<Vec<u8>, ConflictError>;
}

/// Default conflict resolver using standard LWW semantics
pub struct DefaultConflictResolver;

impl DefaultConflictResolver {
    pub fn new() -> Self {
        Self
    }

    /// Create a conflict marker JSON with both versions.
    fn make_conflict_marker(local: &SyncChange, remote: &SyncChange) -> Vec<u8> {
        let marker = serde_json::json!({
            "conflict": true,
            "entity_id": remote.entity_id.to_string(),
            "local": {
                "data": local.data,
                "version": local.version,
                "peer_id": local.peer_id.to_string(),
                "timestamp": local.timestamp,
            },
            "remote": {
                "data": remote.data,
                "version": remote.version,
                "peer_id": remote.peer_id.to_string(),
                "timestamp": remote.timestamp,
            },
            "resolved_at": null,
        });
        marker.to_string().into_bytes()
    }
}

impl Default for DefaultConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver for DefaultConflictResolver {
    fn resolve(
        &self,
        local: Option<&SyncChange>,
        remote: &SyncChange,
        strategy: ConflictStrategy,
    ) -> ConflictResolution {
        match strategy {
            ConflictStrategy::LastWriteWins => {
                let should_accept = local
                    .map(|l| Self::should_accept_lww(l, remote))
                    .unwrap_or(true);
                ConflictResolution {
                    entity_id: remote.entity_id,
                    winning_change: if should_accept {
                        Some(remote.clone())
                    } else {
                        local.cloned()
                    },
                    losing_change: if should_accept {
                        local.cloned()
                    } else {
                        Some(remote.clone())
                    },
                    conflict_marker: None,
                    strategy_used: strategy,
                }
            }
            ConflictStrategy::PreserveBoth => {
                if let Some(local_change) = local {
                    let marker = Self::make_conflict_marker(local_change, remote);
                    ConflictResolution {
                        entity_id: remote.entity_id,
                        winning_change: Some(remote.clone()),
                        losing_change: Some(local_change.clone()),
                        conflict_marker: Some(marker),
                        strategy_used: strategy,
                    }
                } else {
                    // No conflict if no local version
                    ConflictResolution {
                        entity_id: remote.entity_id,
                        winning_change: Some(remote.clone()),
                        losing_change: None,
                        conflict_marker: None,
                        strategy_used: strategy,
                    }
                }
            }
            ConflictStrategy::Manual => {
                // Don't auto-apply either
                ConflictResolution {
                    entity_id: remote.entity_id,
                    winning_change: None,
                    losing_change: None,
                    conflict_marker: None,
                    strategy_used: strategy,
                }
            }
        }
    }

    fn create_conflict_marker(
        &self,
        local: &SyncChange,
        remote: &SyncChange,
    ) -> Result<Vec<u8>, ConflictError> {
        Ok(Self::make_conflict_marker(local, remote))
    }
}

impl DefaultConflictResolver {
    /// Determine if a remote change should be accepted over the local version using LWW.
    fn should_accept_lww(local: &SyncChange, remote: &SyncChange) -> bool {
        if remote.timestamp > local.timestamp {
            return true;
        }
        if remote.timestamp < local.timestamp {
            return false;
        }
        // Same timestamp: use peer_id as tiebreaker (higher UUID wins)
        remote.peer_id.to_string() > local.peer_id.to_string()
    }
}

/// LWW CRDT Sync Engine.
///
/// Uses Last-Writer-Wins (timestamp → peer_id tiebreaker) to resolve
/// concurrent edits without a centralized server.
///
/// The engine can be configured with different conflict resolution strategies.
pub struct CrdtSyncEngine {
    peer_id: Uuid,
    version: u64,
    /// Entity state storage: entity_id → raw bytes
    state: HashMap<Uuid, Vec<u8>>,
    /// Version tracking per entity
    versions: HashMap<Uuid, VersionInfo>,
    /// Conflict resolution strategy
    strategy: ConflictStrategy,
    /// Custom conflict resolver (optional)
    resolver: Box<dyn ConflictResolver>,
}

impl CrdtSyncEngine {
    /// Create a new engine with default LWW strategy.
    pub fn new() -> Self {
        Self {
            peer_id: Uuid::new_v4(),
            version: 0,
            state: HashMap::new(),
            versions: HashMap::new(),
            strategy: ConflictStrategy::LastWriteWins,
            resolver: Box::new(DefaultConflictResolver::new()),
        }
    }

    /// Create an engine with a specific peer ID.
    pub fn with_peer_id(peer_id: Uuid) -> Self {
        Self {
            peer_id,
            version: 0,
            state: HashMap::new(),
            versions: HashMap::new(),
            strategy: ConflictStrategy::LastWriteWins,
            resolver: Box::new(DefaultConflictResolver::new()),
        }
    }

    /// Create an engine with a custom conflict resolver.
    pub fn with_resolver(resolver: Box<dyn ConflictResolver>) -> Self {
        Self {
            peer_id: Uuid::new_v4(),
            version: 0,
            state: HashMap::new(),
            versions: HashMap::new(),
            strategy: ConflictStrategy::LastWriteWins,
            resolver,
        }
    }

    /// Set the conflict resolution strategy.
    pub fn set_strategy(&mut self, strategy: ConflictStrategy) {
        self.strategy = strategy;
    }

    /// Get the current conflict resolution strategy.
    pub fn strategy(&self) -> ConflictStrategy {
        self.strategy
    }

    /// Get this peer's ID.
    pub fn peer_id(&self) -> Uuid {
        self.peer_id
    }

    /// Current monotonic version.
    pub fn current_version(&self) -> u64 {
        self.version
    }

    /// Apply a local change.
    ///
    /// Increments the version counter and records the entity state.
    pub fn apply_local_change(
        &mut self,
        entity_id: Uuid,
        entity_type: &str,
        data: Vec<u8>,
    ) -> SyncChange {
        self.version += 1;
        let now = chrono::Utc::now().timestamp();

        let info = VersionInfo {
            version: self.version,
            last_writer: self.peer_id,
            timestamp: now,
        };

        self.state.insert(entity_id, data.clone());
        self.versions.insert(entity_id, info);

        SyncChange {
            entity_id,
            entity_type: entity_type.to_string(),
            data,
            version: self.version,
            peer_id: self.peer_id,
            timestamp: now,
        }
    }

    /// Apply a remote change with configurable conflict resolution.
    ///
    /// Returns `ConflictResolution` describing what happened.
    pub fn apply_remote_change_with_resolution(
        &mut self,
        change: &SyncChange,
    ) -> ConflictResolution {
        let local_change = self.versions.get(&change.entity_id).map(|info| SyncChange {
            entity_id: change.entity_id,
            entity_type: change.entity_type.clone(),
            data: self
                .state
                .get(&change.entity_id)
                .cloned()
                .unwrap_or_default(),
            version: info.version,
            peer_id: info.last_writer,
            timestamp: info.timestamp,
        });

        let resolution = self
            .resolver
            .resolve(local_change.as_ref(), change, self.strategy);

        if let Some(ref winning) = resolution.winning_change {
            let info = VersionInfo {
                version: winning.version,
                last_writer: winning.peer_id,
                timestamp: winning.timestamp,
            };

            self.state.insert(change.entity_id, winning.data.clone());
            self.versions.insert(change.entity_id, info);

            // Update our monotonic version to be >= remote
            if winning.version > self.version {
                self.version = winning.version;
            }
        }

        resolution
    }

    /// Apply a remote change with conflict resolution (legacy LWW method).
    ///
    /// Uses Last-Writer-Wins: the change with the highest timestamp wins.
    /// On equal timestamps, the higher peer_id (lexicographic UUID string) wins.
    ///
    /// Returns `true` if the remote change was accepted, `false` if rejected.
    pub fn apply_remote_change(&mut self, change: &SyncChange) -> bool {
        let resolution = self.apply_remote_change_with_resolution(change);
        // Return true only if the remote change was accepted (won the conflict)
        resolution
            .winning_change
            .as_ref()
            .is_some_and(|w| w.peer_id == change.peer_id)
    }

    /// Get entity data by ID.
    pub fn get_entity(&self, entity_id: &Uuid) -> Option<&Vec<u8>> {
        self.state.get(entity_id)
    }

    /// Get version info for an entity.
    pub fn get_version_info(&self, entity_id: &Uuid) -> Option<&VersionInfo> {
        self.versions.get(entity_id)
    }

    /// Remove an entity from the sync state.
    pub fn remove_entity(&mut self, entity_id: &Uuid) {
        self.state.remove(entity_id);
        self.versions.remove(entity_id);
    }

    /// Export all state as serialized changes for full sync.
    pub fn export_full(&self) -> Vec<SyncChange> {
        self.versions
            .iter()
            .filter_map(|(entity_id, info)| {
                self.state.get(entity_id).map(|data| SyncChange {
                    entity_id: *entity_id,
                    entity_type: String::new(), // caller should set this
                    data: data.clone(),
                    version: info.version,
                    peer_id: info.last_writer,
                    timestamp: info.timestamp,
                })
            })
            .collect()
    }

    /// Import a batch of remote changes (e.g., from initial sync).
    pub fn import_batch(&mut self, changes: &[SyncChange]) -> usize {
        let mut accepted = 0;
        for change in changes {
            if self.apply_remote_change(change) {
                accepted += 1;
            }
        }
        accepted
    }

    /// Number of tracked entities.
    pub fn entity_count(&self) -> usize {
        self.state.len()
    }
}

impl Default for CrdtSyncEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine_has_peer_id() {
        let engine = CrdtSyncEngine::new();
        assert_ne!(engine.peer_id(), Uuid::nil());
        assert_eq!(engine.current_version(), 0);
        assert_eq!(engine.entity_count(), 0);
    }

    #[test]
    fn test_apply_local_change_increments_version() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        let change = engine.apply_local_change(entity_id, "block", b"hello".to_vec());

        assert_eq!(change.version, 1);
        assert_eq!(change.entity_id, entity_id);
        assert_eq!(change.data, b"hello");
        assert_eq!(engine.current_version(), 1);
        assert_eq!(engine.entity_count(), 1);
    }

    #[test]
    fn test_get_entity_after_change() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        engine.apply_local_change(entity_id, "block", b"content".to_vec());

        let data = engine.get_entity(&entity_id).unwrap();
        assert_eq!(data, b"content");
    }

    #[test]
    fn test_remove_entity() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        engine.apply_local_change(entity_id, "block", b"temp".to_vec());
        assert_eq!(engine.entity_count(), 1);

        engine.remove_entity(&entity_id);
        assert_eq!(engine.entity_count(), 0);
        assert!(engine.get_entity(&entity_id).is_none());
    }

    #[test]
    fn test_export_and_import_batch() {
        let mut engine1 = CrdtSyncEngine::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        engine1.apply_local_change(id1, "block", b"data1".to_vec());
        engine1.apply_local_change(id2, "block", b"data2".to_vec());

        let changes = engine1.export_full();
        assert_eq!(changes.len(), 2);

        let mut engine2 = CrdtSyncEngine::new();
        let accepted = engine2.import_batch(&changes);
        assert_eq!(accepted, 2);
        assert_eq!(engine2.entity_count(), 2);
    }

    #[test]
    fn test_conflict_resolution_newer_timestamp_wins() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        // Local change with older timestamp
        engine.apply_local_change(entity_id, "block", b"local".to_vec());

        // Remote change with newer timestamp
        let remote = SyncChange {
            entity_id,
            entity_type: "block".to_string(),
            data: b"remote".to_vec(),
            version: 10,
            peer_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp() + 1000, // 1000s in the future
        };

        let accepted = engine.apply_remote_change(&remote);
        assert!(accepted, "Newer remote change should be accepted");
        assert_eq!(engine.get_entity(&entity_id).unwrap(), b"remote");
    }

    #[test]
    fn test_conflict_resolution_older_timestamp_rejected() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        // Local change with current timestamp
        engine.apply_local_change(entity_id, "block", b"local".to_vec());

        // Remote change with older timestamp
        let remote = SyncChange {
            entity_id,
            entity_type: "block".to_string(),
            data: b"remote_old".to_vec(),
            version: 5,
            peer_id: Uuid::new_v4(),
            timestamp: 0, // Very old
        };

        let accepted = engine.apply_remote_change(&remote);
        assert!(!accepted, "Older remote change should be rejected");
        assert_eq!(engine.get_entity(&entity_id).unwrap(), b"local");
    }

    #[test]
    fn test_version_info_tracking() {
        let mut engine = CrdtSyncEngine::new();
        let entity_id = Uuid::new_v4();

        engine.apply_local_change(entity_id, "block", b"v1".to_vec());

        let info = engine.get_version_info(&entity_id).unwrap();
        assert_eq!(info.version, 1);
        assert_eq!(info.last_writer, engine.peer_id());
    }

    #[test]
    fn test_preserve_both_strategy() {
        let mut engine = CrdtSyncEngine::new();
        engine.set_strategy(ConflictStrategy::PreserveBoth);

        let entity_id = Uuid::new_v4();
        engine.apply_local_change(entity_id, "block", b"local".to_vec());

        let remote = SyncChange {
            entity_id,
            entity_type: "block".to_string(),
            data: b"remote".to_vec(),
            version: 10,
            peer_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp() + 1000,
        };

        let resolution = engine.apply_remote_change_with_resolution(&remote);

        assert_eq!(resolution.strategy_used, ConflictStrategy::PreserveBoth);
        assert!(resolution.conflict_marker.is_some());
        assert!(resolution.winning_change.is_some());
        assert!(resolution.losing_change.is_some());
    }

    #[test]
    fn test_manual_strategy_defers_resolution() {
        let mut engine = CrdtSyncEngine::new();
        engine.set_strategy(ConflictStrategy::Manual);

        let entity_id = Uuid::new_v4();
        engine.apply_local_change(entity_id, "block", b"local".to_vec());

        let remote = SyncChange {
            entity_id,
            entity_type: "block".to_string(),
            data: b"remote".to_vec(),
            version: 10,
            peer_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp() + 1000,
        };

        let resolution = engine.apply_remote_change_with_resolution(&remote);

        assert_eq!(resolution.strategy_used, ConflictStrategy::Manual);
        assert!(resolution.winning_change.is_none()); // Neither applied automatically
        assert!(resolution.losing_change.is_none());
    }
}
