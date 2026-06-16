//! Weekly Review module
//!
//! Standalone service that produces aggregate statistics for the
//! last 7 days (blocks created, updated, tasks completed, decay
//! trend) plus a heuristic list of "suggestions for next week".
//!
//! Per ADR-0001, no LLM integration: the heuristic is intentionally
//! simple and lives in pure code.

pub mod service;
pub mod types;

pub use service::WeeklyReviewService;
pub use types::{DecayTrend, WeeklyReviewDto};
