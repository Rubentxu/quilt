//! MCP WebSocket client for WASM runtime
//!
//! Provides async WebSocket communication with the MCP JSON-RPC server.
//! Uses web-sys for WebSocket and wasm-bindgen-futures for async support.

use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// Maximum retry attempts for reconnection
const MAX_RETRIES: u32 = 5;

/// Maximum backoff time in milliseconds
const MAX_BACKOFF_MS: u64 = 30_000;

/// Initial backoff time in milliseconds
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Connection state for the MCP client
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// Not connected
    #[default]
    Disconnected,
    /// Currently attempting to connect
    Connecting,
    /// Connected with active request count
    Connected(u32),
    /// Reconnecting after disconnect
    Reconnecting { attempt: u32 },
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionState::Disconnected => write!(f, "Disconnected"),
            ConnectionState::Connecting => write!(f, "Connecting"),
            ConnectionState::Connected(n) => write!(f, "Connected({})", n),
            ConnectionState::Reconnecting { attempt } => {
                write!(f, "Reconnecting(attempt {})", attempt)
            }
        }
    }
}

/// JSON-RPC 2.0 request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub method: String,
    pub params: Value,
    pub id: u32,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC 2.0 request
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);
        Self {
            jsonrpc: "2.0",
            method: method.into(),
            params,
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// JSON-RPC 2.0 response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u32,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// MCP client errors
#[derive(Debug, Clone)]
pub enum McpError {
    ConnectionFailed(String),
    SendFailed(String),
    ResponseError(String),
    NotConnected,
    SerializationError(String),
    Timeout,
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            McpError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            McpError::ResponseError(msg) => write!(f, "Response error: {}", msg),
            McpError::NotConnected => write!(f, "Not connected to server"),
            McpError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            McpError::Timeout => write!(f, "Request timed out"),
        }
    }
}

impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::SerializationError(err.to_string())
    }
}

/// Pending request handle for response matching
struct PendingRequest {
    response_tx: oneshot::Sender<Result<Value, McpError>>,
}

/// MCP client for WebSocket communication
///
/// Uses Arc<Mutex<>> for interior mutability to satisfy Send bounds.
pub struct McpClient {
    socket: Mutex<Option<WebSocket>>,
    server_url: Mutex<String>,
    state: Arc<Mutex<ConnectionState>>,
    pending_requests: Arc<Mutex<HashMap<u32, PendingRequest>>>,
    retry_count: Arc<Mutex<u32>>,
}

impl McpClient {
    /// Create a new MCP client with the given server URL
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            socket: Mutex::new(None),
            server_url: Mutex::new(server_url.into()),
            state: Arc::new(Mutex::new(ConnectionState::Disconnected)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            retry_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Get current connection state
    pub fn get_state(&self) -> ConnectionState {
        self.state.lock().unwrap().clone()
    }

    /// Update the server URL
    pub fn set_server_url(&self, url: &str) {
        *self.server_url.lock().unwrap() = url.to_string();
    }

    /// Get current server URL
    pub fn get_server_url(&self) -> String {
        self.server_url.lock().unwrap().clone()
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff(retries: u32) -> u64 {
        let backoff = INITIAL_BACKOFF_MS * 2u64.pow(retries);
        backoff.min(MAX_BACKOFF_MS)
    }

    /// Set up WebSocket event listeners
    fn setup_event_listeners(&self, socket: &WebSocket) {
        let state = Arc::clone(&self.state);
        let pending_requests = Arc::clone(&self.pending_requests);
        let socket_clone = socket.clone();

        // on_close handler
        let state_for_close = Arc::clone(&state);
        let pending_for_close = Arc::clone(&pending_requests);
        let on_close =
            Closure::<dyn FnMut(CloseEvent)>::wrap(Box::new(move |event: CloseEvent| {
                log::debug!(
                    "WebSocket closed: code={}, reason={}",
                    event.code(),
                    event.reason()
                );

                let current_state = state_for_close.lock().unwrap().clone();
                match current_state {
                    ConnectionState::Connected(_) | ConnectionState::Connecting => {
                        // Unexpected close - trigger reconnection
                        *state_for_close.lock().unwrap() =
                            ConnectionState::Reconnecting { attempt: 1 };
                    }
                    _ => {
                        *state_for_close.lock().unwrap() = ConnectionState::Disconnected;
                    }
                }

                // Clean up pending requests with error
                let mut pending = pending_for_close.lock().unwrap();
                for (_, req) in pending.drain() {
                    req.response_tx
                        .send(Err(McpError::ConnectionFailed(
                            "Connection closed".to_string(),
                        )))
                        .ok();
                }
            }));

        // on_error handler
        let on_error =
            Closure::<dyn FnMut(ErrorEvent)>::wrap(Box::new(move |event: ErrorEvent| {
                log::error!("WebSocket error: {}", event.message());
            }));

        // on_message handler
        let state_for_msg = Arc::clone(&state);
        let pending_for_msg = Arc::clone(&pending_requests);
        let socket_for_msg = socket_clone.clone();
        let on_message =
            Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |event: MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    match serde_json::from_str::<JsonRpcResponse>(&data) {
                        Ok(response) => {
                            let mut pending = pending_for_msg.lock().unwrap();
                            if let Some(request) = pending.remove(&response.id) {
                                let result = match (response.result, response.error) {
                                    (Some(val), None) => Ok(val),
                                    (None, Some(err)) => Err(McpError::ResponseError(err.message)),
                                    _ => Err(McpError::ResponseError(
                                        "Invalid response: missing result or error".to_string(),
                                    )),
                                };
                                request.response_tx.send(result).ok();
                            } else {
                                log::warn!(
                                    "Received response for unknown request id: {}",
                                    response.id
                                );
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse JSON-RPC response: {}", e);
                        }
                    }
                }

                // Update connection state if socket is still open
                if socket_for_msg.ready_state() == WebSocket::CONNECTING {
                    *state_for_msg.lock().unwrap() = ConnectionState::Connecting;
                } else if socket_for_msg.ready_state() == WebSocket::OPEN {
                    // Increment active requests count
                    if let ConnectionState::Connected(n) = *state_for_msg.lock().unwrap() {
                        *state_for_msg.lock().unwrap() =
                            ConnectionState::Connected(n.saturating_add(1));
                    }
                }
            }));

        socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
        socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        // Forget the closures to keep them alive
        on_close.forget();
        on_error.forget();
        on_message.forget();
    }

    /// Connect to the MCP server
    pub async fn connect(&self) -> Result<(), McpError> {
        // Close existing connection if any
        if let Some(ref socket) = *self.socket.lock().unwrap() {
            socket.close().ok();
        }

        {
            *self.state.lock().unwrap() = ConnectionState::Connecting;
        }

        let url = self.server_url.lock().unwrap().clone();
        let socket = WebSocket::new(&url).map_err(|e| {
            McpError::ConnectionFailed(format!("WebSocket creation failed: {:?}", e))
        })?;

        self.setup_event_listeners(&socket);
        *self.socket.lock().unwrap() = Some(socket);

        {
            *self.state.lock().unwrap() = ConnectionState::Connected(0);
        }
        *self.retry_count.lock().unwrap() = 0;

        Ok(())
    }

    /// Disconnect from the MCP server
    pub fn disconnect(&self) {
        if let Some(ref socket) = *self.socket.lock().unwrap() {
            socket.close().ok();
        }
        *self.socket.lock().unwrap() = None;
        *self.state.lock().unwrap() = ConnectionState::Disconnected;

        // Clean up pending requests with error
        let mut pending = self.pending_requests.lock().unwrap();
        for (_, req) in pending.drain() {
            req.response_tx.send(Err(McpError::NotConnected)).ok();
        }
    }

    /// Send a JSON-RPC request and wait for response
    #[allow(clippy::await_holding_lock)]
    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, McpError> {
        let state = self.state.lock().unwrap().clone();
        match state {
            ConnectionState::Connected(_) | ConnectionState::Connecting => {}
            _ => return Err(McpError::NotConnected),
        }

        let socket = self.socket.lock().unwrap();
        let socket = socket.as_ref().ok_or(McpError::NotConnected)?;

        let request = JsonRpcRequest::new(method, params);
        let request_json = serde_json::to_string(&request)?;
        let request_id = request.id;

        socket
            .send_with_str(&request_json)
            .map_err(|e| McpError::SendFailed(format!("Send failed: {:?}", e)))?;

        // Create channel for response
        let (response_tx, response_rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.insert(request_id, PendingRequest { response_tx });
        }

        // Wait for response with timeout
        let response = response_rx.await.map_err(|_| McpError::Timeout)?;

        {
            let mut pending = self.pending_requests.lock().unwrap();
            pending.remove(&request_id);
        }

        // Decrement active requests count
        if let ConnectionState::Connected(n) = *self.state.lock().unwrap() {
            if n > 0 {
                *self.state.lock().unwrap() = ConnectionState::Connected(n - 1);
            }
        }

        response
    }

    /// Attempt to reconnect with exponential backoff
    pub async fn reconnect(&self) -> Result<(), McpError> {
        let retries = *self.retry_count.lock().unwrap();

        if retries >= MAX_RETRIES {
            *self.state.lock().unwrap() = ConnectionState::Disconnected;
            return Err(McpError::ConnectionFailed(
                "Max retries exceeded".to_string(),
            ));
        }

        *self.state.lock().unwrap() = ConnectionState::Reconnecting {
            attempt: retries + 1,
        };

        let backoff_ms = Self::calculate_backoff(retries);
        log::debug!("Reconnecting in {}ms (attempt {})", backoff_ms, retries + 1);

        // Use gloo_timers for async sleep in WASM
        gloo_timers::future::sleep(std::time::Duration::from_millis(backoff_ms)).await;

        *self.retry_count.lock().unwrap() += 1;

        self.connect().await
    }
}
