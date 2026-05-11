//! Sync status indicator component
//!
//! Shows the current sync state with:
//! - Visual indicator (icon/color) based on state
//! - Pending changes count
//! - Last sync time

use leptos::prelude::*;

/// Sync state for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStateDisplay {
    /// Synced and up to date
    Synced,
    /// Currently syncing
    Syncing,
    /// Has pending changes
    Pending,
    /// Offline/not connected
    Offline,
    /// Error occurred
    Error,
    /// Conflict detected
    Conflict,
}

impl SyncStateDisplay {
    /// Get the icon for this state
    pub fn icon(&self) -> &'static str {
        match self {
            SyncStateDisplay::Synced => "[OK]",
            SyncStateDisplay::Syncing => "[...]",
            SyncStateDisplay::Pending => "[*]",
            SyncStateDisplay::Offline => "[O]",
            SyncStateDisplay::Error => "[X]",
            SyncStateDisplay::Conflict => "[!]",
        }
    }

    /// Get the color class for this state
    pub fn color_class(&self) -> &'static str {
        match self {
            SyncStateDisplay::Synced => "sync-synced",
            SyncStateDisplay::Syncing => "sync-syncing",
            SyncStateDisplay::Pending => "sync-pending",
            SyncStateDisplay::Offline => "sync-offline",
            SyncStateDisplay::Error => "sync-error",
            SyncStateDisplay::Conflict => "sync-conflict",
        }
    }

    /// Get the description text
    pub fn description(&self) -> &'static str {
        match self {
            SyncStateDisplay::Synced => "All changes synced",
            SyncStateDisplay::Syncing => "Syncing...",
            SyncStateDisplay::Pending => "Changes pending",
            SyncStateDisplay::Offline => "Offline",
            SyncStateDisplay::Error => "Sync error",
            SyncStateDisplay::Conflict => "Conflicts detected",
        }
    }
}

/// Format a timestamp for display
pub fn format_last_sync(timestamp: Option<i64>) -> String {
    match timestamp {
        Some(ts) => {
            let dt = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now);
            let duration = chrono::Utc::now().signed_duration_since(dt);

            if duration.num_seconds() < 60 {
                "Just now".to_string()
            } else if duration.num_minutes() < 60 {
                format!("{}m ago", duration.num_minutes())
            } else if duration.num_hours() < 24 {
                format!("{}h ago", duration.num_hours())
            } else {
                dt.format("%Y-%m-%d %H:%M").to_string()
            }
        }
        None => "Never".to_string(),
    }
}

/// Compact sync status for sidebar - shows just icon and pending count
#[component]
pub fn SyncStatusCompact(state: SyncStateDisplay, pending_count: usize) -> impl IntoView {
    let tooltip = format!("{} - {} pending", state.description(), pending_count);
    let count_str = if pending_count > 0 {
        pending_count.to_string()
    } else {
        String::new()
    };

    view! {
        <div
            class={format!("sync-indicator-compact {}", state.color_class())}
            title={tooltip}
        >
            <span class="sync-icon-compact">{state.icon()}</span>
            <span class="sync-count">{count_str}</span>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_state_display_icon() {
        assert_eq!(SyncStateDisplay::Synced.icon(), "[OK]");
        assert_eq!(SyncStateDisplay::Syncing.icon(), "[...]");
        assert_eq!(SyncStateDisplay::Pending.icon(), "[*]");
        assert_eq!(SyncStateDisplay::Offline.icon(), "[O]");
        assert_eq!(SyncStateDisplay::Error.icon(), "[X]");
        assert_eq!(SyncStateDisplay::Conflict.icon(), "[!]");
    }

    #[test]
    fn test_format_last_sync_never() {
        assert_eq!(format_last_sync(None), "Never");
    }

    #[test]
    fn test_format_last_sync_recent() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(format_last_sync(Some(now)), "Just now");
    }
}
