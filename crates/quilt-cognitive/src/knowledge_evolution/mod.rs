//! Knowledge Evolution Tracker
//!
//! Tracks how knowledge and beliefs evolve over time.
//!
//! # Overview
//!
//! The KnowledgeEvolutionTracker analyzes blocks over a timespan to detect
//! belief changes, abandoned ideas, and reinforced ideas.
//!
//! # Example
//!
//! ```
//! use quilt_cognitive::KnowledgeEvolutionTracker;
//! use std::sync::Arc;
//!
//! async {
//!     // let tracker = KnowledgeEvolutionTracker::new(block_repo, ai_client);
//!     // let timeline = tracker.track("Rust async", 30).await;
//! };
//! ```

pub mod engine;
pub mod types;

pub use engine::KnowledgeEvolutionTracker;
pub use types::{BeliefChange, KnowledgeTimeline};
