//! WASM integration tests for quilt-ui MCP client
//!
//! These tests run in a browser or WASM runtime using wasm-bindgen-test.
//! Note: Full integration tests require a running MCP server.

use quilt_ui::wasm::client::ConnectionState;
use wasm_bindgen_test::*;

/// Test that ConnectionState Display implementation works correctly
#[wasm_bindgen_test]
fn test_connection_state_display() {
    assert_eq!(ConnectionState::Disconnected.to_string(), "Disconnected");
    assert_eq!(ConnectionState::Connecting.to_string(), "Connecting");
    assert_eq!(ConnectionState::Connected(0).to_string(), "Connected(0)");
    assert_eq!(ConnectionState::Connected(5).to_string(), "Connected(5)");
    assert_eq!(
        ConnectionState::Reconnecting { attempt: 3 }.to_string(),
        "Reconnecting(attempt 3)"
    );
}

/// Test that ConnectionState Default implementation works
#[wasm_bindgen_test]
fn test_connection_state_default() {
    let state = ConnectionState::default();
    assert_eq!(state, ConnectionState::Disconnected);
}

/// Test that ConnectionState equality works
#[wasm_bindgen_test]
fn test_connection_state_equality() {
    assert_eq!(ConnectionState::Connected(0), ConnectionState::Connected(0));
    assert_ne!(ConnectionState::Connected(0), ConnectionState::Connected(1));
    assert_ne!(ConnectionState::Connected(0), ConnectionState::Connecting);
    assert_ne!(ConnectionState::Connecting, ConnectionState::Disconnected);
}

/// Test ConnectionState Clone
#[wasm_bindgen_test]
fn test_connection_state_clone() {
    let state = ConnectionState::Reconnecting { attempt: 4 };
    let cloned = state.clone();
    assert_eq!(state, cloned);
}
