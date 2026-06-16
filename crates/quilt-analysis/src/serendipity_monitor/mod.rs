//! Serendipity Monitor module
//!
//! Standalone service that returns serendipity highlights as a focused DTO.
//! Reuses the connection engine from [`crate::connection_engine`]
//! but enriches each connection with block content previews.

pub mod service;
pub mod types;

pub use service::SerendipityMonitorService;
pub use types::{SerendipityHighlightDetail, SerendipityMonitorDto};
