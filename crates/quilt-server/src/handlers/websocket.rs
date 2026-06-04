//! WebSocket handler for real-time events
//!
//! Provides a WebSocket endpoint at /ws for bidirectional communication.
//!
//! Note: WebSocket support requires the `ws` feature flag on axum.

use axum::{
    extract::{
        Extension,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tracing::{error, info, instrument, warn};

use crate::error::AppError;
use crate::state::{AppState, NavigationEvent};

/// WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsClientMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: Option<String>,
}

/// WebSocket message to client
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsServerMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub channel: Option<String>,
    pub data: Option<NavigationEvent>,
}

/// GET /ws
///
/// WebSocket upgrade handler.
/// Clients can subscribe to navigation events by sending:
/// {"type": "subscribe", "channel": "navigate"}
#[instrument(skip(state))]
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
) -> Result<impl IntoResponse, AppError> {
    info!("WebSocket connection requested");

    let on_upgrade = ws.on_upgrade(move |socket| handle_socket(socket, state));

    Ok(on_upgrade)
}

/// Handle the WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.navigation_tx.subscribe();

    // Track subscribed channels
    let mut subscribed_channels = std::collections::HashSet::new();

    loop {
        tokio::select! {
            // Handle messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_client_message(
                            &text,
                            &mut subscribed_channels,
                            &mut sender,
                        ).await {
                            warn!("Error handling client message: {}", e);
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if let Err(e) = sender.send(Message::Pong(data)).await {
                            warn!("Error sending pong: {}", e);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket connection closed");
                        break;
                    }
                    _ => {}
                }
            }
            // Handle navigation events from broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        // Only send if client is subscribed to navigate channel
                        if subscribed_channels.contains("navigate") {
                            let server_msg = WsServerMessage {
                                msg_type: "navigate-to".to_string(),
                                channel: Some("navigate".to_string()),
                                data: Some(event),
                            };
                            let json = serde_json::to_string(&server_msg).unwrap();
                            if let Err(e) = sender.send(Message::Text(json)).await {
                                warn!("Error sending navigation event: {}", e);
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Missed {} navigation events", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        info!("Navigation broadcast channel closed");
                        break;
                    }
                }
            }
        }
    }

    info!("WebSocket handler finished");
}

/// Handle a message from the WebSocket client
async fn handle_client_message(
    text: &str,
    subscribed_channels: &mut std::collections::HashSet<String>,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
) -> Result<(), AppError> {
    let msg: WsClientMessage = serde_json::from_str(text)
        .map_err(|e| AppError::BadRequest(format!("Invalid JSON: {}", e)))?;

    match msg.msg_type.as_str() {
        "subscribe" => {
            if let Some(channel) = msg.channel {
                subscribed_channels.insert(channel.clone());
                info!(channel = %channel, "Client subscribed to channel");

                // Send acknowledgment
                let response = WsServerMessage {
                    msg_type: "subscribed".to_string(),
                    channel: Some(channel),
                    data: None,
                };
                sender
                    .send(Message::Text(serde_json::to_string(&response).unwrap()))
                    .await
                    .map_err(|e| AppError::Internal(format!("Send error: {}", e)))?;
            }
        }
        "unsubscribe" => {
            if let Some(channel) = msg.channel {
                subscribed_channels.remove(&channel);
                info!(channel = %channel, "Client unsubscribed from channel");

                // Send acknowledgment
                let response = WsServerMessage {
                    msg_type: "unsubscribed".to_string(),
                    channel: Some(channel),
                    data: None,
                };
                sender
                    .send(Message::Text(serde_json::to_string(&response).unwrap()))
                    .await
                    .map_err(|e| AppError::Internal(format!("Send error: {}", e)))?;
            }
        }
        _ => {
            warn!(msg_type = %msg.msg_type, "Unknown message type");
        }
    }

    Ok(())
}
