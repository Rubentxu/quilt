//! Hook system for plugin event notification
//!
//! This module provides the hook dispatch mechanism for plugins to receive
//! notifications about domain events (block changes, page changes, etc.).
//!
//! # Architecture
//!
//! - [`event::HookEvent`] - Enum of all hook event types
//! - [`event::HookPayload`] - Payloads for each hook event type
//! - [`dispatcher::HookDispatcher`] - Central dispatcher that routes events to subscribed plugins
//! - [`error::HookError`] - Error types for hook operations
//!
//! # Hook Flow
//!
//! 1. Core domain emits a hook event (e.g., block created)
//! 2. `HookDispatcher::dispatch()` is called with the event
//! 3. Dispatcher finds all plugins subscribed to that event type
//! 4. Each plugin's `on_hook()` is called in priority order
//! 5. Panics are caught per-plugin to isolate faulty plugins
//!
//! # Example: Creating and Dispatching Events
//!
//! ```
//! use quilt_mcp::hooks::{
//!     HookDispatcher, HookEvent, ChangeType, Priority, HookSubscription, HookEventKind
//! };
//! use quilt_mcp::hooks::event::BlockPayload;
//! use std::sync::Arc;
//!
//! // Create a dispatcher
//! let dispatcher = HookDispatcher::new();
//!
//! // Emit a block changed event
//! let event = HookEvent::BlockChanged(BlockPayload {
//!     id: "block-123".to_string(),
//!     page_id: "page-456".to_string(),
//!     change_type: ChangeType::Created,
//!     content: Some("Hello, world!".to_string()),
//! });
//!
//! // Dispatch to all subscribed plugins
//! let results = dispatcher.dispatch(&event);
//! for result in results {
//!     println!("Plugin '{}' result: {:?}", result.plugin_name, result.success);
//! }
//! ```
//!
//! # Example: Plugin Subscribing to Hooks
//!
//! ```
//! use quilt_mcp::hooks::{HookSubscription, HookEventKind, Priority};
//!
//! // In your Plugin implementation, override subscribed_hooks():
//! fn subscribed_hooks_example() -> Vec<HookSubscription> {
//!     vec![
//!         HookSubscription {
//!             event: HookEventKind::BlockChanged,
//!             priority: Priority::NORMAL,
//!             filter: None,
//!         },
//!     ]
//! }
//! ```

pub mod dispatcher;
pub mod error;
pub mod event;

pub use dispatcher::HookDispatcher;
pub use error::HookError;
pub use event::{
    ChangeType, HookEvent, HookEventKind, HookFilter, HookPayload, HookResult, HookSubscription,
    Priority,
};
