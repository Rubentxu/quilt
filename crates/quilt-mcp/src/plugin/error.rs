//! Plugin error types
//!
//! These errors represent the failure modes that can occur when working
//! with plugins — during registration, tool dispatch, or initialization.
//!
//! # Error Mapping to MCP Protocol
//!
//! Plugin errors are mapped to MCP error codes for JSON-RPC responses:
//! - [`NotFound`](PluginError::NotFound) → `METHOD_NOT_FOUND`
//! - [`AlreadyRegistered`](PluginError::AlreadyRegistered) → `INVALID_REQUEST`
//! - [`ExecutionFailed`](PluginError::ExecutionFailed) → `INTERNAL_ERROR`
//! - [`InitFailed`](PluginError::InitFailed) → `INTERNAL_ERROR`
//! - [`HookFailed`](PluginError::HookFailed) → `INTERNAL_ERROR`

use crate::errors::McpErrorCode;
use crate::hooks::HookError;
use thiserror::Error;

/// Errors that can occur during plugin operations.
///
/// These errors represent distinct failure modes that help callers
/// handle specific situations appropriately:
///
/// - **`NotFound`**: Tool or plugin lookup failed — caller may retry or suggest alternatives
/// - **`AlreadyRegistered`**: Collision detected — caller should pick a different name
/// - **`ExecutionFailed`**: Tool ran but failed — error message contains details
/// - **`InitFailed`**: Plugin setup failed — likely config or environment issue
/// - **`HookFailed`**: Hook dispatch or handler failed — plugin hook returned an error
#[derive(Debug, Error)]
pub enum PluginError {
    /// The requested tool or plugin was not found.
    ///
    /// This occurs when:
    /// - `execute_tool` is called with an unknown tool name
    /// - `unregister` is called with a plugin name that doesn't exist
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// A plugin with the same name is already registered.
    ///
    /// Plugin names must be unique in the registry. This error prevents
    /// accidentally overwriting an existing plugin. Choose a different
    /// name or unregister the existing plugin first.
    #[error("Plugin already registered: {0}")]
    AlreadyRegistered(String),

    /// Tool execution failed.
    ///
    /// The tool was found and invoked, but returned an error. This typically
    /// indicates an operational failure (network issue, invalid input, etc.)
    /// rather than a programming error. The string contains details.
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Plugin initialization failed.
    ///
    /// This occurs during [`Plugin::on_register`] when the plugin's
    /// setup logic fails. Common causes:
    /// - Invalid configuration (wrong path, missing credentials)
    /// - Unable to connect to external service
    /// - Missing system dependencies
    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    /// Hook handling failed.
    ///
    /// This occurs when a plugin's [`Plugin::on_hook`] handler returns an error.
    /// The error message contains details about what failed.
    #[error("Hook handler failed: {0}")]
    HookFailed(#[from] HookError),
}

impl PluginError {
    /// Returns the corresponding MCP error code for this plugin error.
    ///
    /// MCP uses numeric error codes in JSON-RPC responses. This maps
    /// our domain-specific errors to the appropriate protocol codes.
    pub fn to_mcp_error_code(&self) -> McpErrorCode {
        match self {
            PluginError::NotFound(_) => McpErrorCode::MethodNotFound,
            PluginError::AlreadyRegistered(_) => McpErrorCode::InvalidRequest,
            PluginError::ExecutionFailed(_) => McpErrorCode::InternalError,
            PluginError::InitFailed(_) => McpErrorCode::InternalError,
            PluginError::HookFailed(_) => McpErrorCode::InternalError,
        }
    }

    /// Converts this error to an MCP error string for JSON-RPC responses.
    ///
    /// The MCP protocol requires a string error message. This returns
    /// the user-friendly error description suitable for display.
    pub fn to_mcp_string(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let err = PluginError::NotFound("my_tool".to_string());
        assert!(err.to_string().contains("my_tool"));
        assert_eq!(err.to_mcp_error_code(), McpErrorCode::MethodNotFound);
    }

    #[test]
    fn test_already_registered_error() {
        let err = PluginError::AlreadyRegistered("git".to_string());
        assert!(err.to_string().contains("git"));
        assert_eq!(err.to_mcp_error_code(), McpErrorCode::InvalidRequest);
    }

    #[test]
    fn test_execution_failed_error() {
        let err = PluginError::ExecutionFailed("database connection failed".to_string());
        assert!(err.to_string().contains("database connection failed"));
        assert_eq!(err.to_mcp_error_code(), McpErrorCode::InternalError);
    }

    #[test]
    fn test_init_failed_error() {
        let err = PluginError::InitFailed("invalid config".to_string());
        assert!(err.to_string().contains("invalid config"));
        assert_eq!(err.to_mcp_error_code(), McpErrorCode::InternalError);
    }
}
