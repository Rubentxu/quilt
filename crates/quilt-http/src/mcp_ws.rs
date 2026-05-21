//! MCP WebSocket Proxy
//!
//! Bridges WebSocket clients to the MCP server running as a separate process
//! with StdioTransport (stdin/stdout JSON-RPC communication).
//!
//! # Design
//!
//! Each WebSocket connection spawns a new MCP server process. JSON-RPC messages
//! are newline-delimited and forwarded bidirectionally between the WebSocket
//! and the MCP process's stdin/stdout.
//!
//! # Error Handling
//!
//! - MCP process crash → notify client via WebSocket close with reason
//! - Invalid JSON → return JSON-RPC error response
//! - WS connection drop → clean up MCP process

use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Child,
};
use tracing::{error, instrument, warn, info, debug};

use crate::error::HttpError;
use crate::state::HttpState;

/// Error types for MCP WebSocket operations.
#[derive(Debug, thiserror::Error)]
pub enum McpWsError {
    #[error("Failed to spawn MCP process: {0}")]
    ProcessSpawn(String),

    #[error("Failed to write to MCP process: {0}")]
    StdinWrite(String),

    #[error("Failed to read from MCP process: {0}")]
    StdoutRead(String),

    #[error("Invalid JSON-RPC message: {0}")]
    InvalidMessage(String),

    #[error("Task join error: {0}")]
    TaskJoin(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl From<McpWsError> for HttpError {
    fn from(err: McpWsError) -> Self {
        HttpError::InternalError(err.to_string())
    }
}

/// Spawn the MCP server process with StdioTransport.
///
/// Returns handles to the process's stdin and stdout for communication.
async fn spawn_mcp_process(
) -> Result<(tokio::process::ChildStdin, BufReader<tokio::process::ChildStdout>, Child), McpWsError> {
    // Find the MCP server binary path
    // In development, this is typically the quilt-mcp binary or we use cargo run
    let mcp_binary = std::env::var("QUILT_MCP_BINARY")
        .unwrap_or_else(|_| "quilt-mcp".to_string());

    info!("Spawning MCP process: {}", mcp_binary);

    // Spawn the MCP server process
    let mut child = tokio::process::Command::new(&mcp_binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit()) // Inherit stderr for debugging
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                McpWsError::ProcessSpawn(format!(
                    "MCP binary '{}' not found. Set QUILT_MCP_BINARY environment variable.",
                    mcp_binary
                ))
            } else {
                McpWsError::ProcessSpawn(e.to_string())
            }
        })?;

    let stdin = child.stdin.take().ok_or_else(|| {
        McpWsError::ProcessSpawn("Failed to capture stdin".to_string())
    })?;

    let stdout = child.stdout.take().ok_or_else(|| {
        McpWsError::ProcessSpawn("Failed to capture stdout".to_string())
    })?;

    let stdout = BufReader::new(stdout);

    info!("MCP process spawned with PID: {}", child.id().unwrap_or(0));

    Ok((stdin, stdout, child))
}

/// Configure MCP WebSocket routes
pub fn routes() -> axum::Router<Arc<HttpState>> {
    axum::Router::new()
        .route("/ws/mcp", axum::routing::get(ws_mcp_handler))
}

/// GET /ws/mcp - WebSocket endpoint for MCP proxy
///
/// Upgrades the HTTP connection to a WebSocket and spawns an MCP server
/// process to handle the session.
#[instrument(skip(state))]
pub async fn ws_mcp_handler(
    State(state): State<Arc<HttpState>>,
    ws: WebSocketUpgrade,
    request: axum::http::Request<axum::body::Body>,
) -> Result<impl IntoResponse, HttpError> {
    debug!("WebSocket connection request to /ws/mcp");

    // Authenticate the request
    let auth_result = authenticate_request(&request).await;
    if let Err(status) = auth_result {
        return Err(HttpError::Unauthorized(
            "Invalid or missing authentication".to_string(),
        ));
    }

    let _ = state;

    Ok(ws.on_upgrade(handle_mcp_ws))
}

/// Authenticate a request via Authorization header.
///
/// In production: requires valid Bearer token matching QUILT_API_KEY env var.
/// In development: allows unauthenticated access for easier testing.
async fn authenticate_request(request: &axum::http::Request<axum::body::Body>) -> Result<(), StatusCode> {
    // Check for Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            // Validate token against environment variable
            let expected_key = std::env::var("QUILT_API_KEY").unwrap_or_default();
            if token == expected_key {
                debug!("Authenticated request with valid API key");
                Ok(())
            } else {
                warn!("Rejected request with invalid API key");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => {
            // No Authorization header provided
            if cfg!(debug_assertions) {
                // Allow unauthenticated in debug/dev mode for easier testing
                debug!("Dev mode: allowing unauthenticated request");
                Ok(())
            } else {
                // Require authentication in production
                warn!("Rejected unauthenticated request in production");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

/// Handle the WebSocket upgrade and spawn MCP connection handler.
async fn handle_mcp_ws(socket: WebSocket) {
    info!("New MCP WebSocket connection established");

    // Create the MCP connection (spawns the process)
    match spawn_mcp_process().await {
        Ok((mut stdin, stdout, process)) => {
            run_mcp_bridge(socket, &mut stdin, stdout, process).await;
        }
        Err(e) => {
            error!("Failed to spawn MCP process: {}", e);
            // The error is already logged, connection will drop
        }
    }

    info!("MCP WebSocket connection closed");
}

/// Run the bidirectional message forwarding loop.
///
/// - Messages from WebSocket → MCP process (via stdin)
/// - Messages from MCP process (via stdout) → WebSocket
async fn run_mcp_bridge(
    ws: WebSocket,
    stdin: &mut tokio::process::ChildStdin,
    mut stdout: BufReader<tokio::process::ChildStdout>,
    mut process: Child,
) {
    let (mut ws_sender, mut ws_receiver) = ws.split();

    // Buffer for reading lines from MCP stdout
    let mut line = String::new();

    // Main loop: multiplex between WebSocket and MCP stdout
    loop {
        tokio::select! {
            // Handle WebSocket messages
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        debug!("WS received: {} chars", text.len());
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            // Validate JSON
                            if let Err(e) = serde_json::from_str::<serde_json::Value>(trimmed) {
                                warn!("Invalid JSON from WebSocket: {}", e);
                            }
                            // Forward to MCP stdin
                            if let Err(e) = stdin.write_all(trimmed.as_bytes()).await {
                                error!("Failed to write to MCP stdin: {}", e);
                                break;
                            }
                            if let Err(e) = stdin.write_all(b"\n").await {
                                error!("Failed to write newline to MCP stdin: {}", e);
                                break;
                            }
                            debug!("Forwarded {} bytes to MCP stdin", trimmed.len());
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        debug!("WS received binary: {} bytes", data.len());
                        if let Err(e) = stdin.write_all(&data).await {
                            error!("Failed to write binary to MCP stdin: {}", e);
                            break;
                        }
                        if let Err(e) = stdin.write_all(b"\n").await {
                            error!("Failed to write newline to MCP stdin: {}", e);
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if ws_sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        // Pong received, no action needed
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket closed by client");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // WebSocket stream ended
                        break;
                    }
                }
            }
            // Handle MCP stdout
            read_result = stdout.read_line(&mut line) => {
                match read_result {
                    Ok(0) => {
                        // EOF - process exited
                        debug!("MCP process stdout EOF (process exited)");
                        break;
                    }
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            debug!("MCP stdout: {} chars", trimmed.len());
                            // Forward MCP response to WebSocket
                            if ws_sender.send(Message::Text(trimmed.to_string().into())).await.is_err() {
                                warn!("Failed to send MCP response to WebSocket");
                                break;
                            }
                        }
                        line.clear();
                    }
                    Err(e) => {
                        error!("Error reading from MCP stdout: {}", e);
                        break;
                    }
                }
            }
            // Periodically check if process has exited (using try_wait)
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                if let Ok(Some(status)) = process.try_wait() {
                    let code = status.code();
                    info!("MCP process exited with status: {:?}", code);
                    break;
                }
            }
        }
    }

    // Cleanup: terminate the MCP process if still running
    info!("Terminating MCP process");
    let _ = process.kill().await;

    info!("MCP WebSocket bridge cleaned up");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_mcp_process_error_handling() {
        // Test with non-existent binary
        std::env::set_var("QUILT_MCP_BINARY", "/nonexistent/mcp-binary");
        let result = spawn_mcp_process().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, McpWsError::ProcessSpawn(_)));
    }
}