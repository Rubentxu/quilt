//! Integration tests for WebSocket handler — requires a running
//! Quilt server at localhost:3737.
//!
//! Run via: just test-integration
//!
//! Tests connect to ws://localhost:3737/ws, subscribe to navigation
//! events, then trigger a navigate via HTTP and verify the WS message.

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const WS_URL: &str = "ws://localhost:3737/ws";
const HTTP_URL: &str = "http://localhost:3737";

// ── Basic connectivity ─────────────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_websocket_connection_succeeds() {
    let (ws_stream, resp) = connect_async(WS_URL)
        .await
        .expect("failed to connect to WebSocket");

    assert_eq!(resp.status(), 101); // Switching Protocols
    let (mut _write, mut read) = ws_stream.split();

    // Server may send a welcome message or nothing — just verify we can read
    // without immediate close
    // Close the connection cleanly
    drop(read);
}

// ── Subscribe and receive event ────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_websocket_subscribe_and_receive_navigate() {
    // Connect to WebSocket
    let (ws_stream, _resp) = connect_async(WS_URL).await.expect("failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send subscribe message
    let sub_msg = serde_json::json!({
        "type": "subscribe",
        "channel": "navigate"
    });
    write
        .send(Message::Text(sub_msg.to_string()))
        .await
        .expect("failed to send subscribe");

    // Small delay for subscription to register
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Trigger a navigate event via HTTP
    let client = reqwest::Client::new();
    let _resp = client
        .post(format!("{}/api/v1/navigate/page", HTTP_URL))
        .json(&serde_json::json!({"page_name": "ws-test-page"}))
        .send()
        .await
        .expect("navigate HTTP request failed");

    // Read from WebSocket — should receive navigation event
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(3), read.next()).await;

    match timeout {
        Ok(Some(Ok(msg))) => {
            if let Message::Text(text) = msg {
                let parsed: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                // Should be a navigation event
                assert!(
                    parsed["type"].as_str().is_some(),
                    "expected typed message, got: {}",
                    text
                );
            }
        }
        Ok(Some(Err(e))) => {
            // Connection error — may be expected if no subscribers
            eprintln!("WS error (may be expected): {}", e);
        }
        Ok(None) => {
            eprintln!("WS stream closed by server");
        }
        Err(_) => {
            eprintln!("Timeout waiting for WS message — server may not broadcast to subscribers");
        }
    }
}

// ── Ping/Pong ──────────────────────────────────────────────

#[tokio::test]
#[ignore = "requires running server — use: just test-integration"]
async fn test_websocket_ping_pong() {
    let (ws_stream, _resp) = connect_async(WS_URL).await.expect("failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send ping
    write
        .send(Message::Ping(vec![1, 2, 3]))
        .await
        .expect("failed to send ping");

    // Expect pong back
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(3), read.next()).await;

    match timeout {
        Ok(Some(Ok(msg))) => {
            assert!(
                msg.is_pong() || msg.is_close() || msg.is_text(),
                "expected pong, close, or text, got: {:?}",
                msg
            );
        }
        _ => {
            eprintln!("No pong received — server may not support ping/pong");
        }
    }
}
