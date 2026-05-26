//! In-memory repository wrappers with Arc-wrapped builder API.
//!
//! Each wrapper wraps the corresponding `InMemory*Repository` from
//! `quilt_infrastructure::database::in_memory` and provides a fluent
//! builder pattern that returns `Arc<Self>` for easy cloning and sharing.

pub mod block;
pub mod page;
pub mod tag;

pub use block::InMemoryBlockRepo;
pub use page::InMemoryPageRepo;
pub use tag::InMemoryTagRepo;
