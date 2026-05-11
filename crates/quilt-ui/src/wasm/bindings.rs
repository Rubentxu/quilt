//! wasm_bindgen exports for JavaScript interop
//!
//! These functions are exported to JavaScript via wasm_bindgen
//! and provide the public API for the WASM module.

use crate::wasm::client::McpClient;
use js_sys::Promise;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

/// Global MCP client instance wrapped in Arc<Mutex> for shared access
static MCP_CLIENT: Mutex<Option<Arc<McpClient>>> = Mutex::new(None);

/// Initialize the MCP client with the given server URL
///
/// # Arguments
/// * `server_url` - WebSocket URL to the MCP server (e.g., "ws://localhost:9100/mcp")
///
/// # Returns
/// * `Ok(JsValue)` - Success indicator
/// * `Err(JsValue)` - Error message
#[wasm_bindgen]
pub fn init_mcp_client(server_url: &str) -> Result<JsValue, JsValue> {
    let client = Arc::new(McpClient::new(server_url));

    let mut global = MCP_CLIENT.lock().unwrap();
    *global = Some(client);

    to_value(&serde_json::json!({
        "status": "initializing",
        "server_url": server_url
    }))
    .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Connect to the MCP server
///
/// # Returns
/// A JavaScript Promise that resolves on successful connection or rejects with an error
#[wasm_bindgen]
#[allow(clippy::await_holding_lock)]
pub fn connect_mcp_client() -> Promise {
    future_to_promise(async move {
        let global = MCP_CLIENT.lock().unwrap();
        let client = global.as_ref().ok_or_else(|| {
            JsValue::from_str("MCP client not initialized. Call init_mcp_client first.")
        })?;

        client
            .connect()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        to_value(&serde_json::json!({
            "status": "connected"
        }))
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    })
}

/// Reconnect to the MCP server with exponential backoff
///
/// # Returns
/// A JavaScript Promise that resolves on successful reconnection or rejects with an error
#[wasm_bindgen]
#[allow(clippy::await_holding_lock)]
pub fn reconnect_mcp_client() -> Promise {
    future_to_promise(async move {
        let global = MCP_CLIENT.lock().unwrap();
        let client = global.as_ref().ok_or_else(|| {
            JsValue::from_str("MCP client not initialized. Call init_mcp_client first.")
        })?;

        client
            .reconnect()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        to_value(&serde_json::json!({
            "status": "reconnected"
        }))
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    })
}

/// Send a JSON-RPC request to the MCP server
///
/// # Arguments
/// * `method` - The JSON-RPC method name
/// * `params` - The method parameters as a JsValue (will be converted to JSON)
///
/// # Returns
/// A JavaScript Promise that resolves with the response or rejects with an error
#[wasm_bindgen]
#[allow(clippy::await_holding_lock)]
pub fn send_request(method: &str, params: JsValue) -> Promise {
    let method = method.to_string();

    future_to_promise(async move {
        let params_value: serde_json::Value =
            from_value(params).map_err(|e| JsValue::from_str(&format!("Invalid params: {}", e)))?;

        let global = MCP_CLIENT.lock().unwrap();
        let client = global.as_ref().ok_or_else(|| {
            JsValue::from_str("MCP client not initialized. Call init_mcp_client first.")
        })?;

        let response = client
            .send_request(&method, params_value)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        to_value(&response).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    })
}

/// Update the MCP server WebSocket URL
///
/// # Arguments
/// * `url` - The new WebSocket URL
///
/// # Returns
/// * `Ok()` - Success
/// * `Err(JsValue)` - Error if client not initialized
#[wasm_bindgen]
pub fn set_server_url(url: &str) -> Result<JsValue, JsValue> {
    let global = MCP_CLIENT.lock().unwrap();
    let client = global.as_ref().ok_or_else(|| {
        JsValue::from_str("MCP client not initialized. Call init_mcp_client first.")
    })?;

    client.set_server_url(url);
    Ok(JsValue::TRUE)
}

/// Get the current connection state as a string
///
/// # Returns
/// One of: "Disconnected", "Connecting", "Connected(n)", "Reconnecting(attempt n)"
#[wasm_bindgen]
pub fn get_connection_state() -> String {
    let global = MCP_CLIENT.lock().unwrap();
    let client = match global.as_ref() {
        Some(c) => c,
        None => return "NotInitialized".to_string(),
    };

    client.get_state().to_string()
}

/// Disconnect from the MCP server
#[wasm_bindgen]
pub fn disconnect_mcp_client() -> Result<JsValue, JsValue> {
    let global = MCP_CLIENT.lock().unwrap();
    let client = global
        .as_ref()
        .ok_or_else(|| JsValue::from_str("MCP client not initialized."))?;

    client.disconnect();
    Ok(JsValue::TRUE)
}

// ── Domain type serialization helpers ──────────────────────────────────────────

/// Block data transfer object for JS interop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDto {
    pub id: String,
    pub page_id: String,
    pub content: String,
    pub marker: Option<String>,
    pub priority: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Page data transfer object for JS interop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDto {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub journal: bool,
    pub journal_day: Option<i64>,
    pub created_at: String,
}

/// Search result data transfer object for JS interop
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultDto {
    pub block_id: String,
    pub page_id: String,
    pub page_name: String,
    pub content: String,
    pub snippet: Option<String>,
    pub score: f64,
}

// ── Internal helpers for signal integration ───────────────────────────────────

/// Set the global MCP client (used by signals module)
#[doc(hidden)]
pub fn set_global_mcp_client(client: McpClient) {
    let mut global = MCP_CLIENT.lock().unwrap();
    *global = Some(Arc::new(client));
}

/// Get a reference to the global MCP client (used by signals module)
#[doc(hidden)]
pub fn get_global_mcp_client() -> std::sync::MutexGuard<'static, Option<Arc<McpClient>>> {
    MCP_CLIENT.lock().unwrap()
}
