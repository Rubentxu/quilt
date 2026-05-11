//! Stdio transport for MCP server
//!
//! Implements JSON-RPC over stdio for the Model Context Protocol.
//! Reads requests from stdin, writes responses to stdout.

use anyhow::Result;
use quilt_mcp::{server, McpServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::broadcast;

/// JSON-RPC request envelope
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(flatten)]
    pub method: MethodRequest,
}

/// Method-based request to extract method name for validation
#[derive(Debug, Deserialize)]
#[serde(tag = "method")]
pub enum MethodRequest {
    #[serde(rename = "initialize")]
    Initialize { params: () },
    #[serde(rename = "tools/list")]
    ToolsList,
    #[serde(rename = "tools/call")]
    CallTool { params: () },
    #[serde(rename = "resources/list")]
    ResourcesList,
    #[serde(rename = "resources/read")]
    ReadResource { params: () },
    #[serde(rename = "notifications_enabled")]
    EnableNotifications,
}

/// JSON-RPC error response
#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub error: JsonRpcErrorDetail,
    pub id: serde_json::Value,
}

/// JSON-RPC error detail
#[derive(Debug, Serialize)]
pub struct JsonRpcErrorDetail {
    pub code: i32,
    pub message: String,
}

/// JSON-RPC success response
#[derive(Debug, Serialize)]
pub struct JsonRpcSuccess {
    pub jsonrpc: String,
    pub result: serde_json::Value,
    pub id: serde_json::Value,
}

impl JsonRpcError {
    pub fn new(code: i32, message: &str, id: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            error: JsonRpcErrorDetail {
                code,
                message: message.to_string(),
            },
            id,
        }
    }
}

impl JsonRpcSuccess {
    pub fn new(result: serde_json::Value, id: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result,
            id,
        }
    }
}

/// Stdio transport for MCP server
///
/// Reads JSON-RPC requests from stdin line-by-line, calls the server,
/// and writes JSON-RPC responses to stdout.
pub struct StdioTransport;

impl StdioTransport {
    /// Start the MCP server with stdio transport
    ///
    /// Reads JSON-RPC requests from stdin, processes them via the server,
    /// and writes responses to stdout. Handles shutdown on EOF.
    ///
    /// # Arguments
    ///
    /// * `server` - The MCP server instance to handle requests
    ///
    /// # Errors
    ///
    /// Returns an error if stdin/stdout operations fail.
    pub async fn serve(server: Arc<McpServer>) -> Result<()> {
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();
        let mut stdout = io::stdout();

        // Broadcast channel for notifications (if needed in future)
        let (_tx, _rx) = broadcast::channel::<String>(100);

        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse the request
            let response = match serde_json::from_str::<JsonRpcRequest>(line) {
                Ok(req) => {
                    // Validate JSON-RPC version
                    if req.jsonrpc != "2.0" {
                        let error = JsonRpcError::new(
                            -32600,
                            "Invalid Request: jsonrpc must be \"2.0\"",
                            req.id,
                        );
                        serde_json::to_string(&error).unwrap_or_else(|_| {
                            r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"},"id":null}"#.to_string()
                        })
                    } else {
                        // Convert to server request and handle
                        let server_req = Self::convert_request(&req);
                        match server_req {
                            Ok(sreq) => {
                                let resp = server.handle_request(sreq).await;
                                let result =
                                    serde_json::to_value(&resp).unwrap_or(serde_json::Value::Null);
                                let success = JsonRpcSuccess::new(result, req.id);
                                serde_json::to_string(&success).unwrap_or_else(|_| {
                                    r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"},"id":null}"#.to_string()
                                })
                            }
                            Err(e) => {
                                let error = JsonRpcError::new(-32601, &e, req.id);
                                serde_json::to_string(&error).unwrap_or_else(|_| {
                                    r#"{"jsonrpc":"2.0","error":{"code":-32603,"message":"Internal error"},"id":null}"#.to_string()
                                })
                            }
                        }
                    }
                }
                Err(_) => {
                    // Parse error
                    let error = JsonRpcError::new(
                        -32700,
                        "Parse error: Invalid JSON",
                        serde_json::Value::Null,
                    );
                    serde_json::to_string(&error).unwrap_or_else(|_| {
                        r#"{"jsonrpc":"2.0","error":{"code":-32700,"message":"Parse error"},"id":null}"#.to_string()
                    })
                }
            };

            // Write response
            use tokio::io::AsyncWriteExt;
            stdout.write_all(response.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    /// Convert JSON-RPC request to server request
    fn convert_request(req: &JsonRpcRequest) -> Result<server::McpRequest, String> {
        use server::{CallToolParams, InitializeParams, McpRequest, ReadResourceParams};

        match &req.method {
            MethodRequest::Initialize { .. } => Ok(McpRequest::Initialize {
                params: InitializeParams {
                    protocol_version: "2024-11-05".to_string(),
                    capabilities: server::ClientCapabilities {
                        roots: None,
                        sampling: None,
                    },
                },
            }),
            MethodRequest::ToolsList => Ok(McpRequest::ListTools),
            MethodRequest::CallTool { .. } => Ok(McpRequest::CallTool {
                params: CallToolParams {
                    name: String::new(),
                    arguments: serde_json::Value::Null,
                },
            }),
            MethodRequest::ResourcesList => Ok(McpRequest::ListResources),
            MethodRequest::ReadResource { .. } => Ok(McpRequest::ReadResource {
                params: ReadResourceParams { uri: String::new() },
            }),
            MethodRequest::EnableNotifications => Ok(McpRequest::EnableNotifications),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_error_creation() {
        let error = JsonRpcError::new(-32600, "Invalid Request", serde_json::Value::Null);
        assert_eq!(error.jsonrpc, "2.0");
        assert_eq!(error.error.code, -32600);
        assert_eq!(error.error.message, "Invalid Request");
    }

    #[test]
    fn test_json_rpc_success_creation() {
        let success = JsonRpcSuccess::new(
            serde_json::json!({"result": "ok"}),
            serde_json::Value::Number(1.into()),
        );
        assert_eq!(success.jsonrpc, "2.0");
        assert_eq!(success.result, serde_json::json!({"result": "ok"}));
    }

    #[test]
    fn test_error_serialization() {
        let error = JsonRpcError::new(-32700, "Parse error", serde_json::Value::Null);
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"code\":-32700"));
        assert!(json.contains("\"message\":\"Parse error\""));
    }
}
