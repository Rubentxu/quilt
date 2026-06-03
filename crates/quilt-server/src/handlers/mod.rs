//! HTTP request handlers
//!
//! Each module handles a specific domain area of the API.

#[cfg(feature = "cognitive")]
pub mod ai_config;
pub mod blocks;
#[cfg(feature = "cognitive")]
pub mod cognitive;
pub mod frontend;
pub mod health;
pub mod metrics;
pub mod navigate;
pub mod pages;
pub mod search;
pub mod settings;
pub mod templates;
pub mod websocket;
