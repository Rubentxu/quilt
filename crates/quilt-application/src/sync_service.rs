//! Sync service - bridges domain events to the sync engine
//!
//! This service listens to domain events (BlockCreated, BlockUpdated, etc.)
//! and converts them into SyncChanges that are stored in the offline queue
//! and eventually pushed to remote peers via the transport layer.

use crate::errors::ApplicationError;
use quilt_domain::events::{
    AppEvent, BlockCreated, BlockDeleted, BlockMoved, BlockUpdated, PageCreated, PageDeleted,
    PageRenamed,
};
use quilt_domain::value_objects::Uuid as DomainUuid;
use quilt_sync::{ConflictStrategy, CrdtSyncEngine, SyncChange, SyncState, SyncStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tracing::{error, info, warn, Instrument};
use uuid::Uuid;

/// Sync service configuration
#[derive(Debug, Clone)]
pub struct SyncServiceConfig {
    /// Node/peer identifier (usually device ID)
    pub node_id: Uuid,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Auto-sync interval in seconds (0 = disabled)
    pub auto_sync_interval_secs: u64,
    /// Enable auto-sync
    pub auto_sync_enabled: bool,
}

impl Default for SyncServiceConfig {
    fn default() -> Self {
        Self {
            node_id: Uuid::new_v4(),
            conflict_strategy: ConflictStrategy::LastWriteWins,
            auto_sync_interval_secs: 30,
            auto_sync_enabled: false,
        }
    }
}

/// Sync service state
pub struct SyncServiceState {
    /// CRDT engine instance
    pub engine: Arc<RwLock<CrdtSyncEngine>>,
    /// Offline queue for pending changes (optional)
    pub offline_queue: Arc<RwLock<Option<quilt_sync::offline::OfflineQueue>>>,
    /// Pending conflicts waiting for resolution
    pub pending_conflicts: Arc<RwLock<HashMap<Uuid, ConflictInfo>>>,
    /// Last successful sync timestamp
    pub last_synced_at: Arc<RwLock<Option<i64>>>,
}

/// Information about a pending conflict
#[derive(Debug, Clone)]
pub struct ConflictInfo {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub local_change: SyncChange,
    pub remote_change: SyncChange,
    pub detected_at: i64,
}

/// Sync service that bridges domain events to the sync engine
pub struct SyncService {
    config: SyncServiceConfig,
    state: Arc<SyncServiceState>,
}

impl SyncService {
    /// Creates a new SyncService with the given configuration.
    pub fn new(config: SyncServiceConfig) -> Self {
        let mut engine = CrdtSyncEngine::with_peer_id(config.node_id);
        engine.set_strategy(config.conflict_strategy);

        Self {
            config,
            state: Arc::new(SyncServiceState {
                engine: Arc::new(RwLock::new(engine)),
                offline_queue: Arc::new(RwLock::new(None)),
                pending_conflicts: Arc::new(RwLock::new(HashMap::new())),
                last_synced_at: Arc::new(RwLock::new(None)),
            }),
        }
    }

    /// Returns the node/peer ID.
    pub fn node_id(&self) -> Uuid {
        self.config.node_id
    }

    /// Returns the current sync status.
    pub async fn get_sync_status(&self) -> SyncStatus {
        let pending = self.state.offline_queue.read().await;
        let pending_count = pending.as_ref().map(|q| q.pending_count()).unwrap_or(0);
        let last_synced = *self.state.last_synced_at.read().await;

        SyncStatus {
            state: SyncState::Idle,
            pending_changes: pending_count,
            last_synced_at: last_synced,
            last_error: None,
        }
    }

    /// Returns the number of pending conflicts.
    pub async fn pending_conflicts_count(&self) -> usize {
        self.state.pending_conflicts.read().await.len()
    }

    /// Returns all pending conflicts for resolution.
    pub async fn get_pending_conflicts(&self) -> Vec<ConflictInfo> {
        let conflicts = self.state.pending_conflicts.read().await;
        conflicts.values().cloned().collect()
    }

    /// Resolves a conflict by choosing a version.
    ///
    /// # Arguments
    ///
    /// * `entity_id` - The entity ID with the conflict
    /// * `use_local` - If true, keep local version; if false, keep remote version
    pub async fn resolve_conflict(
        &self,
        entity_id: Uuid,
        use_local: bool,
    ) -> Result<(), ApplicationError> {
        let conflict = {
            let mut conflicts = self.state.pending_conflicts.write().await;
            conflicts.remove(&entity_id)
        };

        match conflict {
            Some(info) => {
                let mut engine = self.state.engine.write().await;
                if use_local {
                    // Re-apply local change
                    engine.apply_local_change(
                        info.entity_id,
                        &info.entity_type,
                        info.local_change.data.clone(),
                    );
                } else {
                    // Apply remote change
                    engine.apply_remote_change(&info.remote_change);
                }
                info!(
                    "Resolved conflict for entity {} using {}",
                    entity_id,
                    if use_local { "local" } else { "remote" }
                );
                Ok(())
            }
            None => Err(ApplicationError::NotFound(
                "Conflict",
                DomainUuid::from(entity_id),
            )),
        }
    }

    /// Handles an AppEvent by converting it to a SyncChange.
    ///
    /// This is the main entry point for domain events into the sync system.
    pub async fn handle_event(&self, event: &AppEvent) -> Result<(), ApplicationError> {
        match event {
            AppEvent::BlockCreated(e) => self.handle_block_created(e).await,
            AppEvent::BlockUpdated(e) => self.handle_block_updated(e).await,
            AppEvent::BlockDeleted(e) => self.handle_block_deleted(e).await,
            AppEvent::BlockMoved(e) => self.handle_block_moved(e).await,
            AppEvent::PageCreated(e) => self.handle_page_created(e).await,
            AppEvent::PageRenamed(e) => self.handle_page_renamed(e).await,
            AppEvent::PageDeleted(e) => self.handle_page_deleted(e).await,
            AppEvent::FileChanged(_) => {
                // File events are handled by the file watcher, not sync
                Ok(())
            }
        }
    }

    async fn handle_block_created(&self, event: &BlockCreated) -> Result<(), ApplicationError> {
        info!("Sync: Block created: {}", event.block_id);
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.block_id.into();
        engine.apply_local_change(id, "block", Vec::new());
        Ok(())
    }

    async fn handle_block_updated(&self, event: &BlockUpdated) -> Result<(), ApplicationError> {
        info!(
            "Sync: Block updated: {} ({:?})",
            event.block_id, event.changed_fields
        );
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.block_id.into();
        engine.apply_local_change(id, "block", event.changed_fields.join(",").into_bytes());
        Ok(())
    }

    async fn handle_block_deleted(&self, event: &BlockDeleted) -> Result<(), ApplicationError> {
        info!("Sync: Block deleted: {}", event.block_id);
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.block_id.into();
        engine.remove_entity(&id);
        Ok(())
    }

    async fn handle_block_moved(&self, event: &BlockMoved) -> Result<(), ApplicationError> {
        info!("Sync: Block moved: {}", event.block_id);
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.block_id.into();
        engine.apply_local_change(
            id,
            "block",
            format!(
                "moved: {:?} -> {:?}",
                event.old_parent_id, event.new_parent_id
            )
            .into_bytes(),
        );
        Ok(())
    }

    async fn handle_page_created(&self, event: &PageCreated) -> Result<(), ApplicationError> {
        info!("Sync: Page created: {} ({})", event.page_id, event.name);
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.page_id.into();
        engine.apply_local_change(id, "page", event.name.as_bytes().to_vec());
        Ok(())
    }

    async fn handle_page_renamed(&self, event: &PageRenamed) -> Result<(), ApplicationError> {
        info!(
            "Sync: Page renamed: {} ({} -> {})",
            event.page_id, event.old_name, event.new_name
        );
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.page_id.into();
        engine.apply_local_change(id, "page", event.new_name.as_bytes().to_vec());
        Ok(())
    }

    async fn handle_page_deleted(&self, event: &PageDeleted) -> Result<(), ApplicationError> {
        info!("Sync: Page deleted: {} ({})", event.page_id, event.name);
        let mut engine = self.state.engine.write().await;
        let id: Uuid = event.page_id.into();
        engine.remove_entity(&id);
        Ok(())
    }

    /// Exports all local changes for sync.
    pub async fn export_changes(&self) -> Result<Vec<SyncChange>, ApplicationError> {
        let engine = self.state.engine.read().await;
        Ok(engine.export_full())
    }

    /// Imports remote changes into the local engine.
    ///
    /// Returns the number of conflicts detected.
    pub async fn import_changes(
        &self,
        changes: Vec<SyncChange>,
    ) -> Result<usize, ApplicationError> {
        let mut engine = self.state.engine.write().await;
        let mut conflict_count = 0;

        for change in changes {
            let resolution = engine.apply_remote_change_with_resolution(&change);

            if resolution.winning_change.is_none() && resolution.losing_change.is_some() {
                // Conflict detected - stored for manual resolution
                let entity_id = change.entity_id;
                let conflict_info = ConflictInfo {
                    entity_id,
                    entity_type: change.entity_type.clone(),
                    local_change: resolution.losing_change.unwrap(),
                    remote_change: change.clone(),
                    detected_at: chrono::Utc::now().timestamp(),
                };

                let mut pending = self.state.pending_conflicts.write().await;
                pending.insert(entity_id, conflict_info);
                conflict_count += 1;

                warn!(
                    "Conflict detected for entity {} - {} conflicts pending",
                    entity_id, conflict_count
                );
            }
        }

        Ok(conflict_count)
    }

    /// Gets a conflict by entity ID.
    pub async fn get_conflict(&self, entity_id: Uuid) -> Option<ConflictInfo> {
        self.state
            .pending_conflicts
            .read()
            .await
            .get(&entity_id)
            .cloned()
    }

    /// Records a successful sync timestamp.
    pub async fn record_sync_success(&self) {
        let now = chrono::Utc::now().timestamp();
        let mut last_synced = self.state.last_synced_at.write().await;
        *last_synced = Some(now);
        info!("Sync completed successfully at {}", now);
    }

    /// Gets the CRDT engine for direct access (advanced use).
    pub fn engine(&self) -> Arc<RwLock<CrdtSyncEngine>> {
        self.state.engine.clone()
    }
}

/// Spawns a task that listens to the event bus and forwards events to the sync service.
pub fn spawn_sync_event_listener(
    service: Arc<SyncService>,
    mut receiver: broadcast::Receiver<AppEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(
        async move {
            while let Ok(event) = receiver.recv().await {
                if let Err(e) = service.handle_event(&event).await {
                    error!("Error handling sync event: {}", e);
                }
            }
        }
        .instrument(tracing::info_span!("sync_event_listener")),
    )
}

/// Creates a sync change from a page name and ID.
pub fn page_to_sync_change(
    page_id: DomainUuid,
    page_name: &str,
    peer_id: Uuid,
    timestamp: i64,
) -> SyncChange {
    SyncChange {
        entity_id: page_id.into(),
        entity_type: "page".to_string(),
        data: page_name.as_bytes().to_vec(),
        version: 0,
        peer_id,
        timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_service_new() {
        let service = SyncService::new(SyncServiceConfig::default());
        assert_ne!(service.node_id(), Uuid::nil());
    }

    #[tokio::test]
    async fn test_handle_block_created() {
        let service = SyncService::new(SyncServiceConfig::default());
        let uuid = Uuid::new_v4();
        let event = BlockCreated {
            block_id: uuid.into(),
            page_id: Uuid::new_v4().into(),
            parent_id: None,
        };

        service
            .handle_event(&AppEvent::BlockCreated(event.clone()))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pending_conflicts_count() {
        let service = SyncService::new(SyncServiceConfig::default());
        assert_eq!(service.pending_conflicts_count().await, 0);
    }

    #[tokio::test]
    async fn test_resolve_conflict_not_found() {
        let service = SyncService::new(SyncServiceConfig::default());
        let result = service.resolve_conflict(Uuid::new_v4(), true).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_export_changes_empty() {
        let service = SyncService::new(SyncServiceConfig::default());
        let changes = service.export_changes().await.unwrap();
        assert!(changes.is_empty());
    }

    #[tokio::test]
    async fn test_record_sync_success() {
        let service = SyncService::new(SyncServiceConfig::default());
        service.record_sync_success().await;

        let status = service.get_sync_status().await;
        assert!(status.last_synced_at.is_some());
    }
}
