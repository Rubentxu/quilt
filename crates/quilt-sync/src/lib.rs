//! Quilt Sync — CRDT sync engine
//!
//! Uses Last-Writer-Wins (LWW) with version vectors for conflict resolution.
//! Fully local-first with offline support via the OfflineQueue.
//!
//! # Architecture
//!
//! - [`CrdtSyncEngine`]: Main sync engine with LWW conflict resolution
//! - [`SyncState`]: Current sync state and version tracking
//! - [`offline`]: Offline queue for pending operations
//! - [`transport`]: Sync transport abstraction
//!
//! # Conflict Resolution Strategies
//!
//! - [`ConflictStrategy::LastWriteWins`]: Default, highest timestamp wins
//! - [`ConflictStrategy::PreserveBoth`]: Creates conflict markers for manual resolution
//! - [`ConflictStrategy::Manual`]: Defers resolution to user handler
//!
//! # Example
//!
//! ```
//! use quilt_sync::{CrdtSyncEngine, ConflictStrategy};
//!
//! let mut engine = CrdtSyncEngine::new();
//! engine.set_strategy(ConflictStrategy::PreserveBoth);
//! ```
//! use quilt_sync::{CrdtSyncEngine, ConflictStrategy};
//!
//! let engine = CrdtSyncEngine::new();
//! engine.set_strategy(ConflictStrategy::PreserveBoth);
//! ```

pub mod crdt;
pub mod offline;
pub mod state;
pub mod transport;

pub use crdt::{
    ConflictResolution, ConflictResolver, ConflictStrategy, DefaultConflictResolver, SyncChange,
    VersionInfo,
};
pub use state::{calculate_backoff, SyncState, SyncStatus};
pub use transport::{FileTransport, MockTransport, SyncTransport, TransportChange, TransportError};

#[cfg(feature = "http-client")]
pub use transport::HttpTransport;

pub use crdt::CrdtSyncEngine;
