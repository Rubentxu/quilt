//! Sync state machine with retry support
//!
//! The sync state machine tracks the current sync status and supports
//! exponential backoff for retries.

use tracing::instrument;

/// Base backoff time in milliseconds
const BACKOFF_BASE_MS: u64 = 1000;

/// Maximum backoff time in milliseconds
const BACKOFF_MAX_MS: u64 = 60000;

/// Maximum retry attempts
const MAX_RETRY_ATTEMPTS: u32 = 3;

/// Sync state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncState {
    /// No sync in progress, idle
    #[default]
    Idle,
    /// Currently syncing
    Syncing,
    /// Successfully synced
    Synced,
    /// Sync failed with error
    Error,
    /// Offline, no connectivity
    Offline,
    /// Retrying after failure with backoff
    Retrying {
        /// Current attempt number (1-based)
        attempt: u32,
        /// Maximum attempts before giving up
        max_attempts: u32,
        /// Next backoff duration in milliseconds
        next_backoff_ms: u64,
    },
}

impl SyncState {
    /// Check if sync is currently active.
    pub fn is_active(&self) -> bool {
        matches!(self, SyncState::Syncing)
    }

    /// Check if sync can be initiated.
    pub fn can_sync(&self) -> bool {
        matches!(
            self,
            SyncState::Idle | SyncState::Error | SyncState::Offline
        )
    }

    /// Check if currently retrying.
    pub fn is_retrying(&self) -> bool {
        matches!(self, SyncState::Retrying { .. })
    }

    /// Get the current retry attempt if in retrying state.
    pub fn retry_attempt(&self) -> Option<u32> {
        match self {
            SyncState::Retrying { attempt, .. } => Some(*attempt),
            _ => None,
        }
    }

    /// Get the next backoff duration if in retrying state.
    pub fn next_backoff_ms(&self) -> Option<u64> {
        match self {
            SyncState::Retrying {
                next_backoff_ms, ..
            } => Some(*next_backoff_ms),
            _ => None,
        }
    }

    /// Transition to retrying state with exponential backoff.
    ///
    /// Returns the next state with calculated backoff.
    pub fn begin_retry(self) -> Self {
        let next_backoff = calculate_backoff(1);
        SyncState::Retrying {
            attempt: 1,
            max_attempts: MAX_RETRY_ATTEMPTS,
            next_backoff_ms: next_backoff,
        }
    }

    /// Transition from retrying to next attempt or give up.
    ///
    /// Returns the next state after processing retry result.
    pub fn retry_result(self, success: bool) -> Self {
        match self {
            SyncState::Retrying {
                attempt: _,
                max_attempts: _,
                ..
            } if success => {
                // Success - go to synced
                SyncState::Synced
            }
            SyncState::Retrying {
                attempt,
                max_attempts,
                next_backoff_ms: _,
            } => {
                if attempt >= max_attempts {
                    // Max retries exceeded - go to error
                    SyncState::Error
                } else {
                    // Next retry with increased backoff
                    let new_attempt = attempt + 1;
                    let new_backoff = calculate_backoff(new_attempt);
                    SyncState::Retrying {
                        attempt: new_attempt,
                        max_attempts,
                        next_backoff_ms: new_backoff,
                    }
                }
            }
            _ => self,
        }
    }

    /// Transition to syncing state.
    pub fn start_sync(self) -> Self {
        match self {
            // Can start sync from several states
            SyncState::Idle | SyncState::Error | SyncState::Offline | SyncState::Synced => {
                SyncState::Syncing
            }
            SyncState::Retrying { .. } => SyncState::Syncing, // Cancel retry and sync
            SyncState::Syncing => SyncState::Syncing,         // Already syncing
        }
    }

    /// Transition to synced state.
    pub fn complete_sync(self) -> Self {
        SyncState::Synced
    }

    /// Transition to error state.
    pub fn fail_sync(self) -> Self {
        SyncState::Error
    }

    /// Transition to offline state.
    pub fn go_offline(self) -> Self {
        SyncState::Offline
    }

    /// Transition back to idle (e.g., after acknowledging error).
    pub fn reset(self) -> Self {
        SyncState::Idle
    }

    /// Get a description of the current state for logging.
    pub fn description(&self) -> &'static str {
        match self {
            SyncState::Idle => "idle",
            SyncState::Syncing => "syncing",
            SyncState::Synced => "synced",
            SyncState::Error => "error",
            SyncState::Offline => "offline",
            SyncState::Retrying { .. } => "retrying",
        }
    }
}

/// Calculate exponential backoff with cap.
///
/// Backoff formula: min(BACKOFF_BASE_MS * 2^attempt, BACKOFF_MAX_MS)
///
/// Example: attempt 1 = 1000ms, attempt 2 = 2000ms, attempt 3 = 4000ms, etc.
#[instrument]
pub fn calculate_backoff(attempt: u32) -> u64 {
    let exponential = BACKOFF_BASE_MS * 2u64.pow(attempt.saturating_sub(1));
    exponential.min(BACKOFF_MAX_MS)
}

/// Sync status for reporting
#[derive(Debug, Clone)]
pub struct SyncStatus {
    pub state: SyncState,
    pub pending_changes: usize,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            state: SyncState::Idle,
            pending_changes: 0,
            last_synced_at: None,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_defaults_to_idle() {
        let state = SyncState::default();
        assert_eq!(state, SyncState::Idle);
    }

    #[test]
    fn test_can_sync_from_various_states() {
        assert!(SyncState::Idle.can_sync());
        assert!(SyncState::Error.can_sync());
        assert!(SyncState::Offline.can_sync());
        assert!(!SyncState::Syncing.can_sync());
        assert!(!SyncState::Synced.can_sync());
    }

    #[test]
    fn test_begin_retry() {
        let state = SyncState::Error.begin_retry();
        assert!(matches!(
            state,
            SyncState::Retrying {
                attempt: 1,
                max_attempts: 3,
                next_backoff_ms: 1000,
            }
        ));
    }

    #[test]
    fn test_retry_result_success() {
        let state = SyncState::Retrying {
            attempt: 1,
            max_attempts: 3,
            next_backoff_ms: 1000,
        }
        .retry_result(true);

        assert_eq!(state, SyncState::Synced);
    }

    #[test]
    fn test_retry_result_failure_continues() {
        let state = SyncState::Retrying {
            attempt: 1,
            max_attempts: 3,
            next_backoff_ms: 1000,
        }
        .retry_result(false);

        assert!(matches!(
            state,
            SyncState::Retrying {
                attempt: 2,
                max_attempts: 3,
                next_backoff_ms: 2000,
            }
        ));
    }

    #[test]
    fn test_retry_result_max_exceeded() {
        let state = SyncState::Retrying {
            attempt: 3,
            max_attempts: 3,
            next_backoff_ms: 4000,
        }
        .retry_result(false);

        assert_eq!(state, SyncState::Error);
    }

    #[test]
    fn test_calculate_backoff() {
        assert_eq!(calculate_backoff(1), 1000); // 1000 * 2^0
        assert_eq!(calculate_backoff(2), 2000); // 1000 * 2^1
        assert_eq!(calculate_backoff(3), 4000); // 1000 * 2^2
        assert_eq!(calculate_backoff(10), 60000); // Capped at max
    }

    #[test]
    fn test_start_sync_cancels_retry() {
        let state = SyncState::Retrying {
            attempt: 2,
            max_attempts: 3,
            next_backoff_ms: 2000,
        }
        .start_sync();

        assert_eq!(state, SyncState::Syncing);
    }

    #[test]
    fn test_retrying_state_helpers() {
        let state = SyncState::Retrying {
            attempt: 2,
            max_attempts: 3,
            next_backoff_ms: 2000,
        };

        assert!(state.is_retrying());
        assert_eq!(state.retry_attempt(), Some(2));
        assert_eq!(state.next_backoff_ms(), Some(2000));
    }

    #[test]
    fn test_non_retrying_state_helpers() {
        let state = SyncState::Synced;

        assert!(!state.is_retrying());
        assert_eq!(state.retry_attempt(), None);
        assert_eq!(state.next_backoff_ms(), None);
    }
}
