//! Sync commands (CQRS - Write)
//!
//! Command handlers for sync operations using the CRDT sync engine.

use crate::errors::ApplicationError;
use quilt_sync::{CrdtSyncEngine, SyncState, SyncStatus};
use std::sync::Arc;

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of local changes exported during sync.
    pub changes_exported: usize,
    /// Number of remote changes imported during sync.
    pub changes_imported: usize,
    /// Number of conflicts detected during sync.
    pub conflicts: usize,
}

/// Command handler for sync operations.
///
/// Encapsulates operations that trigger or manage sync:
/// - [`sync_now()`][SyncCommand::sync_now]: Trigger immediate sync
/// - [`get_sync_status()`][SyncCommand::get_sync_status]: Return current sync state
/// - [`flush_offline_queue()`][SyncCommand::flush_offline_queue]: Clear synced entries
///
/// # Type Parameters
///
/// - `R`: A send + sync marker for consistency with other command handlers
pub struct SyncCommand<R: Send + Sync> {
    engine: Arc<CrdtSyncEngine>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Send + Sync> SyncCommand<R> {
    /// Creates a new `SyncCommand` handler with the given CRDT sync engine.
    ///
    /// # Arguments
    ///
    /// * `engine` - An `Arc`-wrapped CRDT sync engine
    pub fn new(engine: Arc<CrdtSyncEngine>) -> Self {
        Self {
            engine,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Triggers an immediate sync operation.
    ///
    /// Exports all local changes from the sync engine. In a full implementation,
    /// this would also connect to a transport to exchange changes with remote peers.
    ///
    /// # Returns
    ///
    /// Returns [`SyncResult`] with counts of exported, imported changes and conflicts.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the sync operation fails.
    pub async fn sync_now(&self) -> Result<SyncResult, ApplicationError> {
        let changes = self.engine.export_full();
        let changes_exported = changes.len();

        Ok(SyncResult {
            changes_exported,
            changes_imported: 0,
            conflicts: 0,
        })
    }

    /// Returns the current sync status.
    ///
    /// Provides a snapshot of the sync engine's state including:
    /// - Current [`SyncState`]
    /// - Number of pending (non-synced) changes
    /// - Timestamp of last successful sync
    /// - Last error message if any
    ///
    /// # Returns
    ///
    /// Returns [`SyncStatus`] with current sync information.
    pub fn get_sync_status(&self) -> SyncStatus {
        SyncStatus {
            state: SyncState::Idle,
            pending_changes: self.engine.entity_count(),
            last_synced_at: None,
            last_error: None,
        }
    }

    /// Flushes the offline queue by clearing all synced entries.
    ///
    /// This is useful for cleanup after a successful sync to reduce database size.
    /// In this implementation, the offline queue is not integrated into the command
    /// context - this method is a placeholder for full integration.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError`] if the flush operation fails.
    pub async fn flush_offline_queue(&self) -> Result<(), ApplicationError> {
        // The OfflineQueue requires a database pool which is not available in this
        // command context. Full integration would inject the queue as a dependency.
        Ok(())
    }
}

/// Type alias for `SyncCommand` implementing the command handler pattern.
///
/// This is the concrete type returned by command factory functions and
/// used in dependency injection for request handlers.
pub type SyncCommandHandler<R> = SyncCommand<R>;

#[cfg(test)]
mod tests {
    use super::*;
    use quilt_domain::value_objects::Uuid;

    #[tokio::test]
    async fn test_sync_now_returns_result() {
        let engine = Arc::new(CrdtSyncEngine::new());
        let command = SyncCommand::<()>::new(engine);

        let result = command.sync_now().await;

        assert!(result.is_ok());
        let sync_result = result.unwrap();
        assert_eq!(sync_result.changes_exported, 0);
        assert_eq!(sync_result.changes_imported, 0);
        assert_eq!(sync_result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_sync_now_with_changes() {
        let mut engine = CrdtSyncEngine::new();

        // Apply a local change
        let entity_id = Uuid::new_v4();
        engine.apply_local_change(entity_id.into(), "block", b"test content".to_vec());

        let command = SyncCommand::<()>::new(Arc::new(engine));
        let result = command.sync_now().await;

        assert!(result.is_ok());
        let sync_result = result.unwrap();
        assert_eq!(sync_result.changes_exported, 1);
    }

    #[test]
    fn test_get_sync_status_defaults_to_idle() {
        let engine = Arc::new(CrdtSyncEngine::new());
        let command = SyncCommand::<()>::new(engine);

        let status = command.get_sync_status();

        assert_eq!(status.state, SyncState::Idle);
        assert_eq!(status.pending_changes, 0);
        assert!(status.last_synced_at.is_none());
        assert!(status.last_error.is_none());
    }

    #[test]
    fn test_get_sync_status_reflects_entity_count() {
        let mut engine = CrdtSyncEngine::new();

        // Apply some local changes
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        engine.apply_local_change(id1.into(), "block", b"content 1".to_vec());
        engine.apply_local_change(id2.into(), "block", b"content 2".to_vec());

        let command = SyncCommand::<()>::new(Arc::new(engine));
        let status = command.get_sync_status();

        assert_eq!(status.pending_changes, 2);
    }

    #[tokio::test]
    async fn test_flush_offline_queue_returns_ok() {
        let engine = Arc::new(CrdtSyncEngine::new());
        let command = SyncCommand::<()>::new(engine);

        let result = command.flush_offline_queue().await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sync_command_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}

        let engine = Arc::new(CrdtSyncEngine::new());
        let _command = SyncCommand::<()>::new(engine);

        assert_send_sync::<SyncCommand<()>>();
    }
}
