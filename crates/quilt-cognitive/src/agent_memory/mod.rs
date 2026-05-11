//! Agent Memory — Persistent Memory for AI Agents
//!
//! Stores AI agent observations and learning as Blocks with reserved
//! `agent-memory/{domain}` namespace, leveraging the existing BlockRepository.
//!
//! Provides:
//! - Block-based storage with upsert semantics
//! - Retrieval by agent context, domain, and FTS5 free-text search
//! - Exponential relevance decay over time
//! - Cross-session persistence via BlockRepository

mod engine;
mod store;
mod types;

pub use engine::AgentMemory;
pub use types::{CognitiveBias, InteractionProfile, MemoryEntry, MemoryQuery, ThinkingPattern};
