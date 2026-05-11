//! Counterfactual Explorer
//!
//! Explores "what if" scenarios and alternative paths in the knowledge graph.
//!
//! # Overview
//!
//! The CounterfactualExplorer analyzes scenarios and decision points to generate
//! alternative branches, consequences, and challenged assumptions.
//!
//! # Example
//!
//! ```
//! use quilt_cognitive::CounterfactualExplorer;
//! use std::sync::Arc;
//!
//! async {
//!     // let explorer = CounterfactualExplorer::new(block_repo, ai_client);
//!     // let tree = explorer.explore("Learning Rust", "Should I learn async?").await;
//! };
//! ```

pub mod engine;
pub mod types;

pub use engine::CounterfactualExplorer;
pub use types::{CounterfactualBranch, CounterfactualTree};
