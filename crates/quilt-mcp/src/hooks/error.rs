//! Hook error types
//!
//! These errors represent the failure modes that can occur during hook
//! registration, subscription, and dispatch operations.

use thiserror::Error;

/// Errors that can occur during hook operations.
///
/// These errors help callers handle specific failure situations:
#[derive(Debug, Error)]
pub enum HookError {
    /// The hook event kind is not supported or recognized.
    #[error("Unsupported hook event kind: {0}")]
    UnsupportedEventKind(String),

    /// Registration failed due to invalid plugin or data.
    #[error("Hook registration failed: {0}")]
    RegistrationFailed(String),

    /// A panic was caught during plugin hook execution.
    ///
    /// This indicates the plugin's hook handler panicked. The panic
    /// is caught to prevent it from crashing the host application.
    #[error("Plugin hook panicked: {0}")]
    PluginPanicked(String),

    /// Dispatch failed for an individual plugin.
    ///
    /// This captures execution errors that are not panics, such as
    /// returned error values from plugin hook handlers.
    #[error("Hook dispatch failed for plugin '{plugin}': {reason}")]
    DispatchFailed {
        /// Name of the plugin that failed
        plugin: String,
        /// Reason for the failure
        reason: String,
    },

    /// The plugin is not subscribed to this event kind.
    #[error("Plugin '{0}' is not subscribed to hook events")]
    NotSubscribed(String),

    /// Maximum hook depth exceeded (prevents infinite recursion).
    #[error("Maximum hook dispatch depth exceeded: {0}")]
    MaxDepthExceeded(usize),
}

impl HookError {
    /// Returns true if this error indicates a plugin fault (panic or dispatch failure).
    pub fn is_plugin_fault(&self) -> bool {
        matches!(
            self,
            HookError::PluginPanicked(_) | HookError::DispatchFailed { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_plugin_fault() {
        let panic_err = HookError::PluginPanicked("test panic".to_string());
        assert!(panic_err.is_plugin_fault());

        let dispatch_err = HookError::DispatchFailed {
            plugin: "test".to_string(),
            reason: "failed".to_string(),
        };
        assert!(dispatch_err.is_plugin_fault());

        let other_err = HookError::NotSubscribed("test".to_string());
        assert!(!other_err.is_plugin_fault());
    }

    #[test]
    fn test_error_display() {
        let err = HookError::PluginPanicked("null pointer".to_string());
        assert!(err.to_string().contains("null pointer"));

        let err = HookError::DispatchFailed {
            plugin: "my_plugin".to_string(),
            reason: "boom".to_string(),
        };
        assert!(err.to_string().contains("my_plugin"));
        assert!(err.to_string().contains("boom"));
    }
}
