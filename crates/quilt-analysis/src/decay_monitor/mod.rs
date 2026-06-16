//! Decay Monitor module
//!
//! Standalone service that returns decay alerts for the user's
//! knowledge graph. Reuses the same algorithm as the morning
//! briefing via [`crate::shared_decay::detect_decay_alerts`],
//! but exposes only the decay section as a focused DTO.

pub mod service;
pub mod types;

pub use service::DecayMonitorService;
pub use types::{DecayMonitorDto, SeverityCounts};
