//! MCP Protocol Error Codes
//!
//! Implements standard JSON-RPC error codes as defined in the MCP specification.

/// MCP error codes as defined by the JSON-RPC 2.0 specification
/// and the Model Context Protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpErrorCode {
    /// Parse error - Invalid JSON was received.
    /// The server could not parse the JSON text.
    ParseError = -32700,

    /// Invalid Request - The JSON sent is not a valid Request object.
    InvalidRequest = -32600,

    /// Method not found - The method does not exist or is not available.
    MethodNotFound = -32601,

    /// Invalid params - Invalid method parameter(s).
    InvalidParams = -32602,

    /// Internal error - Internal server error.
    InternalError = -32603,

    /// Server not initialized - The server is not yet initialized.
    ServerNotInitialized = -32002,

    /// Content type not supported - The content type is not supported.
    ContentTypeNotSupported = -32003,

    /// Request entity too large - The request is too large.
    RequestEntityTooLarge = -32004,
}

impl McpErrorCode {
    /// Returns the numeric code value.
    pub fn code(&self) -> i32 {
        *self as i32
    }

    /// Returns the error message for this code.
    pub fn message(&self) -> &'static str {
        match self {
            McpErrorCode::ParseError => "Parse error: Invalid JSON",
            McpErrorCode::InvalidRequest => "Invalid Request",
            McpErrorCode::MethodNotFound => "Method not found",
            McpErrorCode::InvalidParams => "Invalid params",
            McpErrorCode::InternalError => "Internal error",
            McpErrorCode::ServerNotInitialized => "Server not initialized",
            McpErrorCode::ContentTypeNotSupported => "Content type not supported",
            McpErrorCode::RequestEntityTooLarge => "Request entity too large",
        }
    }
}

impl std::fmt::Display for McpErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for McpErrorCode {}

/// MCP error type that includes code and message.
#[derive(Debug, Clone)]
pub struct McpError {
    pub code: McpErrorCode,
    pub message: String,
}

impl McpError {
    /// Creates a new MCP error.
    pub fn new(code: McpErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Creates an error for a method that was not found.
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: McpErrorCode::MethodNotFound,
            message: format!("Method not found: {}", method),
        }
    }

    /// Creates an error for invalid parameters.
    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: McpErrorCode::InvalidParams,
            message: msg.to_string(),
        }
    }

    /// Creates an error for invalid JSON-RPC version.
    pub fn invalid_jsonrpc_version() -> Self {
        Self {
            code: McpErrorCode::InvalidRequest,
            message: "Invalid Request: jsonrpc must be \"2.0\"".to_string(),
        }
    }

    /// Creates an internal error with the given message.
    pub fn internal(msg: &str) -> Self {
        Self {
            code: McpErrorCode::InternalError,
            message: msg.to_string(),
        }
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.message(), self.message)
    }
}

impl std::error::Error for McpError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(McpErrorCode::ParseError.code(), -32700);
        assert_eq!(McpErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(McpErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(McpErrorCode::InvalidParams.code(), -32602);
        assert_eq!(McpErrorCode::InternalError.code(), -32603);
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(
            McpErrorCode::ParseError.message(),
            "Parse error: Invalid JSON"
        );
        assert_eq!(McpErrorCode::MethodNotFound.message(), "Method not found");
    }

    #[test]
    fn test_mcp_error_creation() {
        let err = McpError::method_not_found("unknown_method");
        assert_eq!(err.code, McpErrorCode::MethodNotFound);
        assert!(err.message.contains("unknown_method"));
    }

    #[test]
    fn test_display() {
        let err = McpError::invalid_params("missing 'query' parameter");
        let display = format!("{}", err);
        assert!(display.contains("Invalid params"));
        assert!(display.contains("missing 'query' parameter"));
    }
}
