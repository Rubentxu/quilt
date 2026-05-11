//! Leptos signal integration for MCP client
//!
//! Provides reactive connection state and request/response signals
//! for integration with the Leptos UI framework.

use crate::wasm::bindings::get_global_mcp_client;
use crate::wasm::client::ConnectionState;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// MCP client signal wrapper for Leptos reactive UI
///
/// This struct wraps access to the global MCP client and provides
/// Leptos signals for reactive connection state updates.
#[derive(Clone)]
pub struct McpClientSignal {
    /// Reactive connection state
    pub connection_state: ReadSignal<ConnectionState>,
    /// Setter for connection state
    set_connection_state: WriteSignal<ConnectionState>,
}

impl McpClientSignal {
    /// Create a new MCP client signal wrapper
    ///
    /// Sets up reactive signals for connection state polling.
    pub fn new() -> Self {
        // Create Leptos signals
        let (connection_state, set_connection_state) = signal(ConnectionState::Disconnected);

        let client_signal = Self {
            connection_state,
            set_connection_state,
        };

        // Spawn a task to poll connection state changes
        spawn_local({
            let set_connection_state = client_signal.set_connection_state;

            async move {
                loop {
                    gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;

                    let global = get_global_mcp_client();
                    if let Some(ref client) = *global {
                        let state = client.get_state();
                        set_connection_state.set(state);
                    }
                }
            }
        });

        client_signal
    }

    /// Connect to the MCP server
    #[allow(clippy::await_holding_lock)]
    pub async fn connect(&self) -> Result<(), String> {
        let global = get_global_mcp_client();
        if let Some(ref client) = *global {
            client.connect().await.map_err(|e| e.to_string())
        } else {
            Err("MCP client not initialized".to_string())
        }
    }

    /// Send a JSON-RPC request
    #[allow(clippy::await_holding_lock)]
    pub async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let global = get_global_mcp_client();
        if let Some(ref client) = *global {
            client
                .send_request(method, params)
                .await
                .map_err(|e| e.to_string())
        } else {
            Err("MCP client not initialized".to_string())
        }
    }

    /// Disconnect from the MCP server
    pub fn disconnect(&self) {
        let global = get_global_mcp_client();
        if let Some(ref client) = *global {
            client.disconnect();
        }
    }
}

impl Default for McpClientSignal {
    fn default() -> Self {
        Self::new()
    }
}

/// Connection status indicator component for Leptos UI
///
/// Displays the current MCP connection state with appropriate styling.
#[component]
pub fn ConnectionStatus() -> impl IntoView {
    let state = expect_context::<McpClientSignal>().connection_state;

    view! {
        <div class="connection-status">
            <span class="status-indicator" class:connected={move || matches!(state.get(), ConnectionState::Connected(_))}>
                {move || {
                    match state.get() {
                        ConnectionState::Disconnected => "⚫ Disconnected".to_string(),
                        ConnectionState::Connecting => "🟡 Connecting...".to_string(),
                        ConnectionState::Connected(n) => format!("🟢 Connected ({})", n),
                        ConnectionState::Reconnecting { attempt } => format!("🟠 Reconnecting ({}...)", attempt),
                    }
                }}
            </span>
        </div>
    }
}
