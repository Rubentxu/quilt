//! Serendipity Engine — Unexpected Connection Discovery
//!
//! Discovers unexpected connections between knowledge blocks by computing:
//! - **Structural similarity**: Jaccard index on shared references
//! - **Temporal proximity**: Exponential decay based on creation timestamps
//! - **Semantic bridge**: AI-powered concept bridging (via AIClient)
//!
//! Results are paginated and cached (LRU, 5-minute TTL).

mod engine;
mod types;

pub use engine::SerendipityEngine;
pub use types::{ConnectionType, SerendipityConnection, SerendipityOptions, SerendipityQuery};
